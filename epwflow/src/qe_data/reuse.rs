use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use super::manifest::QeManifest;
use super::super::foundation::store;

/// 导入报告：来源清单、导入角色、复制后校验与证据。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReport {
    pub schema_version: u32,
    pub source_run: String,
    pub imported_roles: Vec<String>,
    pub checks: Vec<Check>,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Check {
    pub role: String,
    pub passed: bool,
    pub detail: String,
}

/// 从已有运行目录导入已验证的 qe-data 清单到本运行隔离树（FR-009）。
/// 仅当来源清单 status=valid 且所有必需角色经重验（哈希一致）才登记为可复用阶段。
pub fn import_from_run(source_work: &Path, work: &Path) -> Result<ImportReport> {
    let source_manifest_path = source_work.join("qe-data").join("manifest.json");
    let source = QeManifest::load(&source_manifest_path)
        .with_context(|| format!("data_invalid: 无法加载来源清单 {}", source_manifest_path.display()))?;
    ensure!(
        source.validation.all_passed(),
        "data_invalid: 来源清单校验未通过，拒绝复用"
    );

    let target = work.join("imported").join("qe-data");
    std::fs::create_dir_all(&target)?;
    let mut checks = Vec::new();
    let mut roles = Vec::new();

    // 复制并逐文件重验来源响应文件，防止来源被篡改后冒充。
    let mut copy_role = |role: &str, src: &Path, rep: &mut Vec<String>| -> Result<()> {
        ensure!(src.exists(), "data_invalid: 来源角色 {role} 文件缺失 {}", src.display());
        let dest = target.join(src.file_name().context("data_invalid: 文件缺名")?);
        std::fs::copy(src, &dest)
            .with_context(|| format!("data_invalid: 复制来源 {role} 失败"))?;
        let src_sha = store::hash_file(src)?;
        let dst_sha = store::hash_file(&dest)?;
        let ok = src_sha == dst_sha;
        checks.push(Check {
            role: role.to_string(),
            passed: ok,
            detail: if ok {
                "复制前后哈希一致".to_string()
            } else {
                "复制前后哈希不一致（来源可能被篡改）".to_string()
            },
        });
        if ok {
            rep.push(role.to_string());
        }
        Ok(())
    };

    for ph in &source.phonon_files {
        copy_role(&format!("dyn_q{}", ph.q_id), &ph.dyn_file, &mut roles)?;
        copy_role(&format!("patterns_q{}", ph.q_id), &ph.patterns, &mut roles)?;
        copy_role(&format!("control_q{}", ph.q_id), &ph.control_metadata, &mut roles)?;
        if let Some(dv) = &ph.dvscf {
            copy_role(&format!("dvscf_q{}", ph.q_id), dv, &mut roles)?;
        }
    }
    if let Some(scf) = &source.scf {
        copy_role("scf_snapshot", &scf.snapshot_dir, &mut roles)?;
    }
    if let Some(st) = &source.states {
        copy_role("nscf_wavefunctions", &st.wavefunctions, &mut roles)?;
    }

    let all_ok = checks.iter().all(|c| c.passed);
    let report = ImportReport {
        schema_version: 1,
        source_run: source.run_id.clone(),
        imported_roles: roles.clone(),
        checks,
        status: if all_ok { "imported" } else { "rejected" }.to_string(),
    };
    if !all_ok {
        bail!("data_invalid: 导入校验未通过，拒绝登记可复用阶段");
    }
    store::write(&work.join("imported").join("import-report.json"), &report)?;
    Ok(report)
}

/// 列出可复用阶段的来源身份（FR-009：来源、参数、版本、布局一致才复用）。
pub fn list_importable(source_work: &Path) -> Result<BTreeMap<String, String>> {
    let source_manifest_path = source_work.join("qe-data").join("manifest.json");
    let source = QeManifest::load(&source_manifest_path)?;
    let mut m = BTreeMap::new();
    m.insert("run_id".to_string(), source.run_id);
    m.insert("status".to_string(), source.validation.status.clone());
    m.insert("structure".to_string(), source.fingerprints.structure_sha256.clone());
    m.insert("n_qpoints".to_string(), source.qpoints.len().to_string());
    m.insert(
        "n_phonon_files".to_string(),
        source.phonon_files.len().to_string(),
    );
    Ok(m)
}
