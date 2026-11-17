//! 编号阶段直接读取原生粗网格、细网格和谱函数；不发布共享状态清单。
use super::{coarse, epw_config::EpwConfig, fine, manifest::*, native};
use crate::{
    foundation::config::Config,
    qe_data,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

pub fn coarse(cfg: &Config) -> Result<InterpolationManifest> {
    let structure = qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw = EpwConfig::from_config(cfg)?.with_species(&structure.cell.elements)?;
    let result = coarse::collect_coarse(&cfg.work_dir, &epw)?;
    let validation = Validation {
        status: "valid".into(),
        checks: vec![check(
            "coarse_complete",
            true,
            "Native Matrix, Log、Wannier Proposed line-by-line inspection",
        )],
        units: BTreeMap::new(),
        thresholds: BTreeMap::new(),
        evidence: vec![],
    };
    Ok(InterpolationManifest {
        schema_version: 2,
        run_id: "native-coarse".into(),
        f2_digest: String::new(),
        wannier_digest: String::new(),
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
    let mut manifest = InterpolationManifest {
        schema_version: 2, run_id: "native-fine".into(), f2_digest: String::new(),
        wannier_digest: String::new(), coarse: None, fine: None, spectra: vec![],
        mode_readiness: BTreeMap::new(),
        validation: Validation { status: "valid".into(), checks: vec![], units: BTreeMap::new(), thresholds: BTreeMap::new(), evidence: vec![] },
    };
    let structure = qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw = EpwConfig::from_config(cfg)?.with_species(&structure.cell.elements)?;
    let mut other = vec![];
    for (index, smearing) in cfg.interpolation.phonon_smearings_mev.iter().take(1).enumerate() {
        let dir=super::fine::directory(cfg)?;
        let result = fine::collect_native_fine_at(&cfg.work_dir, &dir, &epw)?;
        let raw = native::NativeSpectrum::read(
            &dir.join(format!("{}.a2f", cfg.qe.prefix)),
            cfg.interpolation.spectral_points as usize,
        )?;
        ensure!(raw.a2f.len() == 1, "data_invalid: Each extension should produce an native spectra");
        manifest.spectra.push(Spectrum {
            smearing_id: format!("mev-{smearing:.16e}"),
            energy_mev: raw.energy_mev,
            a2f: raw.a2f[0].clone(),
            lambda_integral: raw.lambda_integral[0],
            lambda_sum: raw.lambda_sum,
            cutoff_ev: cfg.superconductivity.frequency_cutoff_ev,
        });
        if index == 0 {
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
        .context("data_invalid: Missing selected extension fine grid data")?;
    data.artifacts.extend(other);
    manifest.mode_readiness.insert("fine".into(), true);
    manifest.validation.check(
        "fine_complete",
        true,
        "All widths, arrays, restart count and native spectroscopy pass",
    );
    Ok(manifest)
}
