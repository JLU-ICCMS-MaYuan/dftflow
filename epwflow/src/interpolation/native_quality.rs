//! 对实际 QE/EPW 产物进行只读质量对照；保留超差结果，不修改原始数据。
use super::native::{electronic_quality, parse_bands, ElectronicQuality, NativeBands};
use super::quality::{FREQUENCY_MAX_ERROR_CM1, FREQUENCY_RMS_ERROR_CM1};
use crate::{foundation::store, qe_data::nscf};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MEV_CM1: f64 = 8.065543937;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: PathBuf,
    pub sha256: String,
}

impl SourceFile {
    pub(crate) fn checked_path(&self, base: &Path) -> Result<PathBuf> {
        let path = base.join(&self.path);
        ensure!(
            self.sha256.len() == 64 && store::hash_file(&path)? == self.sha256,
            "data_invalid: 质量来源指纹不符 {}",
            path.display()
        );
        Ok(path)
    }
}

/// 路径均相对本清单。ASR 派生文件必须同时登记原始 PH 和转换程序的指纹。
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualitySources {
    pub schema_version: u32,
    pub asr: String,
    pub window_ev: [f64; 2],
    pub fermi_reference: SourceFile,
    pub electron_reference: SourceFile,
    pub electron_interpolated: SourceFile,
    pub phonon_reference_cm1: SourceFile,
    pub phonon_interpolated_mev: SourceFile,
    pub provenance: Vec<SourceFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhononQuality {
    pub points: usize,
    pub modes_per_point: usize,
    pub errors_cm1: Vec<Vec<f64>>,
    pub max_error_cm1: f64,
    pub rms_error_cm1: f64,
    pub max_error_limit_cm1: f64,
    pub rms_error_limit_cm1: f64,
    pub passed: bool,
}

/// 参考为 cm⁻¹，EPW phband.freq 为 meV；保留虚频符号，覆盖所有 3N 模。
pub fn phonon_quality(
    reference: &NativeBands,
    interpolated: &NativeBands,
    modes: usize,
) -> Result<PhononQuality> {
    ensure!(
        modes > 0
            && modes % 3 == 0
            && !reference.values.is_empty()
            && reference.values.len() == reference.points_cartesian.len()
            && interpolated.values.len() == interpolated.points_cartesian.len()
            && reference.values.len() == interpolated.values.len(),
        "data_invalid: 声子验证点或模式不完整"
    );
    let mut errors = Vec::new();
    for i in 0..reference.values.len() {
        ensure!(
            reference.points_cartesian[i]
                .iter()
                .zip(interpolated.points_cartesian[i])
                .all(|(a, b)| a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-6),
            "data_invalid: 声子验证点坐标不一致"
        );
        ensure!(
            reference.values[i].len() == modes
                && interpolated.values[i].len() == modes
                && reference.values[i]
                    .iter()
                    .chain(&interpolated.values[i])
                    .all(|v| v.is_finite()),
            "data_invalid: 声子模式缺失或非有限"
        );
        let mut r = reference.values[i].clone();
        let mut c: Vec<f64> = interpolated.values[i].iter().map(|v| v * MEV_CM1).collect();
        r.sort_by(f64::total_cmp);
        c.sort_by(f64::total_cmp);
        errors.push(
            r.iter()
                .zip(c)
                .map(|(a, b)| (a - b).abs())
                .collect::<Vec<_>>(),
        );
    }
    let max_error_cm1 = errors.iter().flatten().copied().fold(0.0, f64::max);
    let count = reference.values.len() * modes;
    let rms_error_cm1 = (errors.iter().flatten().map(|v| v * v).sum::<f64>() / count as f64).sqrt();
    // 存在动力学不稳定时，即使误差小也不能进入首版常规超导交付。
    let stable = reference.values.iter().flatten().all(|v| *v >= -1.0)
        && interpolated
            .values
            .iter()
            .flatten()
            .all(|v| *v * MEV_CM1 >= -1.0);
    Ok(PhononQuality {
        points: reference.values.len(),
        modes_per_point: modes,
        errors_cm1: errors,
        max_error_cm1,
        rms_error_cm1,
        max_error_limit_cm1: FREQUENCY_MAX_ERROR_CM1,
        rms_error_limit_cm1: FREQUENCY_RMS_ERROR_CM1,
        passed: stable
            && max_error_cm1 <= FREQUENCY_MAX_ERROR_CM1
            && rms_error_cm1 <= FREQUENCY_RMS_ERROR_CM1,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeQuality {
    pub schema_version: u32,
    pub workflow_complete: bool,
    pub asr: String,
    pub electron: ElectronicQuality,
    pub phonon: PhononQuality,
    pub passed: bool,
    pub source_manifest_sha256: String,
}

/// 检查现有参考，不生成参考或调用科学程序。失败数值仍可用于诊断报告。
pub fn inspect(path: &Path) -> Result<NativeQuality> {
    let base = path.parent().context("data_invalid: 质量清单缺少目录")?;
    let sources: QualitySources = serde_json::from_str(&fs::read_to_string(path)?)?;
    ensure!(
        sources.schema_version == 1
            && sources.asr == "epw_simple_gamma_onsite"
            && !sources.provenance.is_empty(),
        "data_invalid: 质量清单版本、ASR 或来源证据缺失"
    );
    for file in &sources.provenance {
        file.checked_path(base)?;
    }
    let fermi = nscf::parse_xml(&sources.fermi_reference.checked_path(base)?)?.fermi;
    ensure!(
        sources.window_ev[0] <= fermi - 0.5 && sources.window_ev[1] >= fermi + 0.5,
        "data_invalid: 电子质量能窗未覆盖绑定 Fermi 能左右各 0.5 eV"
    );
    let reference_path = sources.electron_reference.checked_path(base)?;
    let states = nscf::parse_xml(&reference_path)?;
    let xml = fs::read_to_string(&reference_path)?;
    let doc = roxmltree::Document::parse(&xml)?;
    let atoms: usize = doc
        .descendants()
        .find(|n| n.has_tag_name("atomic_structure"))
        .and_then(|n| n.attribute("nat"))
        .context("data_invalid: 质量参考缺少原子数")?
        .parse()?;
    let reference = NativeBands {
        points_cartesian: states.kpoints,
        values: states.eigenvalues,
    };
    let interpolated = parse_bands(&fs::read_to_string(
        sources.electron_interpolated.checked_path(base)?,
    )?)?;
    let electron = electronic_quality(&reference, &interpolated, sources.window_ev, 0.05, 0.02)?;
    let reference = parse_bands(&fs::read_to_string(
        sources.phonon_reference_cm1.checked_path(base)?,
    )?)?;
    let interpolated = parse_bands(&fs::read_to_string(
        sources.phonon_interpolated_mev.checked_path(base)?,
    )?)?;
    let phonon = phonon_quality(
        &reference,
        &interpolated,
        atoms.checked_mul(3).context("data_invalid: 原子数溢出")?,
    )?;
    // 点的独立性/网格内外身份仍须与冻结计算计划核对；本命令不自动替代最终验收。
    ensure!(
        electron.points >= 6 && phonon.points >= 4,
        "data_invalid: 独立质量点覆盖不足"
    );
    Ok(NativeQuality {
        schema_version: 1,
        workflow_complete: false,
        asr: sources.asr,
        passed: electron.passed && phonon.passed,
        electron,
        phonon,
        source_manifest_sha256: store::hash_file(path)?,
    })
}

/// 调度器仅消费当前粗网格阶段的对照，不能借用另一套已通过报告放行。
pub fn check_current_coarse(path: &Path, coarse: &Path) -> Result<NativeQuality> {
    let base = path.parent().context("data_invalid: 质量清单缺少目录")?;
    let sources: QualitySources = serde_json::from_str(&fs::read_to_string(path)?)?;
    for (file, name) in [
        (&sources.electron_interpolated, "band.eig"),
        (&sources.phonon_interpolated_mev, "phband.freq"),
    ] {
        ensure!(
            file.checked_path(base)?.canonicalize()? == coarse.join(name).canonicalize()?,
            "data_invalid: 质量来源不属于当前粗网格阶段"
        );
    }
    inspect(path)
}
