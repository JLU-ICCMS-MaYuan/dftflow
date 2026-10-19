//! 先冻结独立参考，再将当前粗网格输出配对；历史插值结果不作为执行前置。
use super::native_quality::{self, NativeQuality, QualitySources, SourceFile};
use crate::{
    foundation::{
        config::Config,
        store::{self, Identity},
    },
    qe_data::{nscf, xml::Geometry},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frozen {
    pub source: Identity,
    pub sources: QualitySources,
}

fn resolve(file: &SourceFile, base: &Path) -> Result<SourceFile> {
    Ok(SourceFile {
        path: file.checked_path(base)?.canonicalize()?,
        sha256: file.sha256.clone(),
    })
}

/// 参考文件、ASR、点集、Fermi 和窗口先通过。这里不读取旧的插值输出。
pub fn freeze(cfg: &Config, fermi: f64) -> Result<Frozen> {
    let path = cfg
        .interpolation
        .quality
        .reference_manifest
        .as_ref()
        .context("data_invalid: 缺少独立插值质量来源清单")?;
    let base = path.parent().context("data_invalid: 质量清单缺父目录")?;
    let mut s: QualitySources = store::read(path)?;
    ensure!(
        s.schema_version == 1 && s.asr == "epw_simple_gamma_onsite" && !s.provenance.is_empty(),
        "data_invalid: 参考版本、ASR 或来源无效"
    );
    s.fermi_reference = resolve(&s.fermi_reference, base)?;
    s.electron_reference = resolve(&s.electron_reference, base)?;
    s.phonon_reference_cm1 = resolve(&s.phonon_reference_cm1, base)?;
    s.provenance = s
        .provenance
        .iter()
        .map(|f| resolve(f, base))
        .collect::<Result<_>>()?;
    let reference_fermi = nscf::parse_xml(&s.fermi_reference.path)?.fermi;
    // quality.energy_window_ev 由配置契约定义为相对共同 Fermi；独立来源保存绝对 eV。
    let requested_window = cfg
        .interpolation
        .quality
        .energy_window_ev
        .map(|v| v + fermi);
    ensure!(
        (reference_fermi - fermi).abs() < 1e-7
            && s.window_ev
                .iter()
                .zip(requested_window)
                .all(|(a, b)| (a - b).abs() < 1e-9)
            && s.window_ev[0] <= fermi - 0.5
            && s.window_ev[1] >= fermi + 0.5,
        "input_conflict: 独立参考 Fermi／目标窗与 F2 或固定配置不相容"
    );
    let electron = nscf::parse_xml(&s.electron_reference.path)?;
    let phonon = super::native::parse_bands(&fs::read_to_string(&s.phonon_reference_cm1.path)?)?;
    let geometry = Geometry::read(&s.electron_reference.path)?;
    let q = &cfg.interpolation.quality;
    ensure!(
        electron.kpoints.len() >= 6
            && phonon.points_cartesian.len() >= 4
            && electron.kpoints.len() == q.k_points_crystal.len()
            && phonon.points_cartesian.len() == q.q_points_crystal.len(),
        "data_invalid: 独立质量参考点集覆盖不足或与固定配置不同"
    );
    for (actual, expected) in [
        (&electron.kpoints, &q.k_points_crystal),
        (&phonon.points_cartesian, &q.q_points_crystal),
    ] {
        for (cart, crystal) in actual.iter().zip(expected) {
            let converted = geometry.crystal(*cart);
            ensure!(
                converted
                    .iter()
                    .zip(crystal)
                    .all(|(a, b)| (a - b).abs() < 1e-6),
                "input_conflict: 独立参考坐标／顺序不同"
            );
        }
    }
    ensure!(
        phonon
            .values
            .iter()
            .all(|v| v.len() == geometry.atoms.len() * 3),
        "data_invalid: 参考声子缺失模式"
    );
    // 丢弃旧插值身份，输出必须来自本次新粗网格。占位不写成通过报告。
    s.electron_interpolated = SourceFile {
        path: "band.eig".into(),
        sha256: String::new(),
    };
    s.phonon_interpolated_mev = SourceFile {
        path: "phband.freq".into(),
        sha256: String::new(),
    };
    Ok(Frozen {
        source: Identity::capture(path)?,
        sources: s,
    })
}

impl Frozen {
    pub fn verify(&self) -> Result<()> {
        self.source.verify()?;
        for f in [
            &self.sources.fermi_reference,
            &self.sources.electron_reference,
            &self.sources.phonon_reference_cm1,
        ] {
            f.checked_path(Path::new("/"))?;
        }
        for f in &self.sources.provenance {
            f.checked_path(Path::new("/"))?;
        }
        Ok(())
    }

    pub fn pair(self, work: &Path) -> Result<NativeQuality> {
        self.verify()?;
        let origin = self.source.clone();
        let mut sources = self.sources;
        let source_file = |name: &str| -> Result<SourceFile> {
            let path = work.join("coarse").join(name).canonicalize()?;
            Ok(SourceFile {
                sha256: store::hash_file(&path)?,
                path,
            })
        };
        sources.electron_interpolated = source_file("band.eig")?;
        sources.phonon_interpolated_mev = source_file("phband.freq")?;
        sources.provenance.push(SourceFile {
            path: origin.path.clone(),
            sha256: origin.sha256.clone(),
        });
        let path = work.join("quality-reference.json");
        ensure!(
            !path.exists() && !work.join("quality-binding.json").exists(),
            "input_conflict: 已有质量发布现场"
        );
        store::write(&path, &sources)?;
        let report = native_quality::check_current_coarse(&path, &work.join("coarse"))?;
        store::write(&work.join("quality-report.json"), &report)?;
        store::write(
            &work.join("quality-binding.json"),
            &Binding {
                schema_version: 1,
                origin,
                paired: Identity::capture(&path)?,
            },
        )?;
        Ok(report)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    schema_version: u32,
    origin: Identity,
    paired: Identity,
}

/// F4 重验配对来源及原配置身份；兼容已有显式指向当前 coarse 的历史清单。
pub fn check(cfg: &Config, work: &Path) -> Result<NativeQuality> {
    let source = cfg
        .interpolation
        .quality
        .reference_manifest
        .as_ref()
        .context("data_invalid: 缺独立质量来源")?;
    let binding_path = work.join("quality-binding.json");
    let path = if binding_path.exists() {
        let binding: Binding = store::read(&binding_path)?;
        ensure!(
            binding.schema_version == 1
                && binding.origin.path.canonicalize()? == source.canonicalize()?
                && binding.paired.path.canonicalize()?
                    == work.join("quality-reference.json").canonicalize()?,
            "input_conflict: 质量绑定不属于当前来源或工作目录"
        );
        binding.origin.verify()?;
        binding.paired.verify()?;
        binding.paired.path
    } else {
        source.clone()
    };
    native_quality::check_current_coarse(&path, &work.join("coarse"))
}
