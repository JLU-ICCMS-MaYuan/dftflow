//! 编号阶段直接读取原生粗网格、细网格和谱函数；不发布共享状态清单。
use super::{coarse, epw_config::EpwConfig, fine, manifest::*, native, wannier};
use crate::{
    foundation::{config::Config, layout},
    qe_data,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

pub fn coarse(cfg: &Config) -> Result<InterpolationManifest> {
    let qe = qe_data::direct::load(cfg)?;
    let states = qe.states.as_ref().context("data_invalid: 缺 NSCF")?;
    let structure = qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw = EpwConfig::from_config(cfg)?
        .with_fermi(states.fermi)?
        .with_bands(&states.eigenvalues)?
        .with_species(&structure.cell.elements)?;
    let result = coarse::collect_coarse(&cfg.work_dir, &epw)?;
    let wan = wannier::check_native(
        &layout::path(&cfg.work_dir, "coarse").join(format!("{}.wout", cfg.qe.prefix)),
        cfg.wannier.num_wann as usize,
    )?;
    ensure!(wan.converged, "data_invalid: Wannier 未收敛");
    let validation = Validation {
        status: "valid".into(),
        checks: vec![check(
            "coarse_complete",
            true,
            "原生矩阵、日志、Wannier 拟合逐项检查",
        )],
        units: BTreeMap::new(),
        thresholds: BTreeMap::new(),
        evidence: vec![],
    };
    Ok(InterpolationManifest {
        schema_version: 2,
        run_id: qe.run_id.clone(),
        f2_digest: crate::foundation::store::hash_bytes(&serde_json::to_vec(&qe)?),
        wannier_digest: wan.wannier_digest,
        coarse: Some(coarse::into_coarse_data(&result)),
        fine: None,
        spectra: vec![],
        mode_readiness: BTreeMap::from([
            ("wannier".into(), true),
            ("coarse".into(), true),
            ("fine".into(), false),
        ]),
        validation,
    })
}
pub fn fine(cfg: &Config) -> Result<InterpolationManifest> {
    let mut manifest = coarse(cfg)?;
    let states = qe_data::nscf::verified_states(cfg, &cfg.work_dir)?;
    let structure = qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw = EpwConfig::from_config(cfg)?
        .with_fermi(states.fermi)?
        .with_bands(&states.eigenvalues)?
        .with_species(&structure.cell.elements)?;
    let mut other = vec![];
    for (index, smearing) in cfg.interpolation.phonon_smearings_mev.iter().enumerate() {
        let dir = layout::path(&cfg.work_dir, "fine").join(format!("smearing-{:04}", index + 1));
        let result = fine::collect_native_fine_at(&cfg.work_dir, &dir, &epw)?;
        let raw = native::NativeSpectrum::read(
            &dir.join(format!("{}.a2f", cfg.qe.prefix)),
            cfg.interpolation.spectral_points as usize,
        )?;
        ensure!(raw.a2f.len() == 1, "data_invalid: 每个展宽应产生一列原生谱");
        manifest.spectra.push(Spectrum {
            smearing_id: format!("mev-{smearing:.16e}"),
            energy_mev: raw.energy_mev,
            a2f: raw.a2f[0].clone(),
            lambda_integral: raw.lambda_integral[0],
            lambda_sum: raw.lambda_sum,
            cutoff_ev: cfg.superconductivity.frequency_cutoff_ev,
        });
        if index == cfg.superconductivity.spectrum_index as usize {
            manifest.fine = Some(fine::into_fine_data(&result));
        } else {
            for mut artifact in result.artifacts {
                artifact.role = format!("smearing_{:04}_{}", index + 1, artifact.role);
                other.push(artifact);
            }
        }
    }
    let data = manifest
        .fine
        .as_mut()
        .context("data_invalid: 缺少所选展宽的细网格数据")?;
    data.artifacts.extend(other);
    manifest.mode_readiness.insert("fine".into(), true);
    manifest.validation.check(
        "fine_complete",
        true,
        "全部展宽、矩阵分片、重启计数及原生谱检查通过",
    );
    Ok(manifest)
}
