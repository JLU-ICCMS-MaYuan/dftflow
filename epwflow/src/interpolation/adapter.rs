use super::super::foundation::config;
use super::super::foundation::store;
use super::super::qe_data::manifest::QeManifest;
use anyhow::{ensure, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// F3 与 F2 的绑定：F2 清单有效、程序身份与来源不可变（FR-003）。
#[derive(Debug, Clone)]
pub struct AdapterBinding {
    pub f2_digest: String,
    pub program_identities: BTreeMap<String, String>,
    pub fermi_ev: f64,
    pub eigenvalues: Vec<Vec<f64>>,
}

/// 校验 F2 来源清单有效并固定程序身份；前置未满足不得放行（FR-003）。
pub fn bind_f2(f2_path: &Path, cfg: &config::Config) -> Result<AdapterBinding> {
    ensure!(
        f2_path.exists() || cfg.layout.as_deref() == Some("numbered"),
        "data_invalid: F2 manifest is missing {}",
        f2_path.display()
    );
    let f2 = QeManifest::load(f2_path)
        .map_err(|e| anyhow::anyhow!("data_invalid: Cannot Load F2 manifest: {e}"))?;
    f2.verify()?;
    ensure!(
        f2.fingerprints.structure_sha256 == store::hash_file(&cfg.structure)?,
        "input_conflict: F2 Structure source is incompatible with current configuration"
    );
    for (el, path) in &cfg.pseudos {
        ensure!(
            f2.fingerprints.pseudo_sha256.get(el) == Some(&store::hash_file(path)?),
            "input_conflict: F2 The power source is incompatible with the current configuration"
        );
    }
    for (role, path) in [("pw", &cfg.programs.pw.path), ("ph", &cfg.programs.ph.path)] {
        ensure!(
            f2.fingerprints.program_sha256.get(role) == Some(&store::hash_file(path)?),
            "input_conflict: F2 Program source is incompatible with current configuration"
        );
    }
    ensure!(
        f2.validation.all_passed(),
        "data_invalid: F2 Local validation not passed,F3 Don't let go"
    );
    ensure!(
        f2.scf.is_some() && !f2.qpoints.is_empty() && f2.qpoints.len() == f2.phonon_files.len(),
        "data_invalid: F2 Missing SCF Or complete PH/q It's a map. I can't believe it valid Label Release"
    );
    let states = f2
        .states
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("data_invalid: F2 Missing complete electronic state"))?;
    ensure!(
        states.fermi.is_finite()
            && states.k_grid == cfg.qe.nscf.k_grid
            && states.nbnd == cfg.qe.nscf.nbnd as usize,
        "data_invalid: F2 Fermi Yes or no NSCF Grid/band incompatibility"
    );
    ensure!(
        qf_wannier::validate_bands(&states.eigenvalues)? == states.nbnd
            && states.eigenvalues.len() == states.kpoints.len(),
        "data_invalid: F2 The window selection is incomplete"
    );
    let f2_digest = if cfg.layout.as_deref() == Some("numbered") {
        store::hash_bytes(&serde_json::to_vec(&f2)?)
    } else {
        store::hash_file(f2_path)?
    };
    let mut program_identities = BTreeMap::new();
    program_identities.insert("pw".to_string(), store::hash_file(&cfg.programs.pw.path)?);
    program_identities.insert("ph".to_string(), store::hash_file(&cfg.programs.ph.path)?);
    program_identities.insert("epw".to_string(), store::hash_file(&cfg.programs.epw.path)?);
    Ok(AdapterBinding {
        f2_digest,
        program_identities,
        fermi_ev: states.fermi,
        eigenvalues: states.eigenvalues.clone(),
    })
}
