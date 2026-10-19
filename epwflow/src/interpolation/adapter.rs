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
        f2_path.exists(),
        "data_invalid: F2 清单缺失 {}",
        f2_path.display()
    );
    let f2 = QeManifest::load(f2_path)
        .map_err(|e| anyhow::anyhow!("data_invalid: 无法加载 F2 清单: {e}"))?;
    f2.verify()?;
    ensure!(
        f2.fingerprints.structure_sha256 == store::hash_file(&cfg.structure)?,
        "input_conflict: F2 结构来源与当前配置不相容"
    );
    for (el, path) in &cfg.pseudos {
        ensure!(
            f2.fingerprints.pseudo_sha256.get(el) == Some(&store::hash_file(path)?),
            "input_conflict: F2 赝势来源与当前配置不相容"
        );
    }
    for (role, path) in [("pw", &cfg.programs.pw.path), ("ph", &cfg.programs.ph.path)] {
        ensure!(
            f2.fingerprints.program_sha256.get(role) == Some(&store::hash_file(path)?),
            "input_conflict: F2 程序来源与当前配置不相容"
        );
    }
    ensure!(
        f2.validation.all_passed(),
        "data_invalid: F2 本地校验未通过，F3 不得放行"
    );
    ensure!(
        f2.scf.is_some() && !f2.qpoints.is_empty() && f2.qpoints.len() == f2.phonon_files.len(),
        "data_invalid: F2 缺少 SCF 或完整 PH/q 映射，不能凭 valid 标签放行"
    );
    let states = f2
        .states
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("data_invalid: F2 缺少完整电子态"))?;
    ensure!(
        states.fermi.is_finite()
            && states.k_grid == cfg.qe.nscf.k_grid
            && states.nbnd == cfg.qe.nscf.nbnd as usize,
        "data_invalid: F2 Fermi 能或 NSCF 网格/带数不相容"
    );
    ensure!(
        qf_wannier::validate_bands(&states.eigenvalues)? == states.nbnd
            && states.eigenvalues.len() == states.kpoints.len(),
        "data_invalid: F2 选窗本征值不完整"
    );
    let f2_digest = store::hash_file(f2_path)?;
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
