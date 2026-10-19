use super::super::foundation::store;
use super::manifest::QeManifest;
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

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
    let source = QeManifest::load(&source_manifest_path).with_context(|| {
        format!(
            "data_invalid: 无法加载来源清单 {}",
            source_manifest_path.display()
        )
    })?;
    ensure!(
        source.validation.all_passed(),
        "data_invalid: 来源清单校验未通过，拒绝复用"
    );

    source.verify()?;
    let source_root = source
        .artifact_root
        .as_ref()
        .context("data_invalid: 缺原生产物根目录")?
        .canonicalize()?;
    std::fs::create_dir_all(work)?;
    ensure!(
        !work.canonicalize()?.starts_with(&source_root),
        "input_conflict: 不能把导入目标放入来源树"
    );
    let target = work.join("imported/qe-data");
    ensure!(!target.exists(), "input_conflict: 导入目标已存在");
    std::fs::create_dir_all(&target)?;
    let mut checks = vec![];
    let mut roles = vec![];
    for a in &source.artifacts {
        let src = source_root.join(&a.relative_path);
        let dst = target.join(&a.relative_path);
        std::fs::create_dir_all(dst.parent().unwrap())?;
        std::fs::copy(&src, &dst)?;
        ensure!(
            store::hash_file(&dst)? == a.sha256,
            "input_conflict: 导入复制后身份不符"
        );
        roles.push(a.role.clone());
        checks.push(Check {
            role: a.role.clone(),
            passed: true,
            detail: "按原生清单复制并重验身份".into(),
        });
    }
    source.verify()?;
    let rebase = |p: &std::path::PathBuf| -> Result<std::path::PathBuf> {
        Ok(target.join(
            p.strip_prefix(&source_root)
                .context("data_invalid: 来源角色越出 artifact_root")?,
        ))
    };
    let mut imported = source.clone();
    imported.artifact_root = Some(target.canonicalize()?);
    if let Some(r) = &mut imported.scf {
        r.snapshot_dir = rebase(&r.snapshot_dir)?;
    }
    if let Some(r) = &mut imported.states {
        r.wavefunctions = rebase(&r.wavefunctions)?;
    }
    for r in &mut imported.phonon_files {
        r.dyn_file = rebase(&r.dyn_file)?;
        r.patterns = rebase(&r.patterns)?;
        r.control_metadata = rebase(&r.control_metadata)?;
        r.dvscf = r.dvscf.as_ref().map(rebase).transpose()?;
    }
    let imported_dyn0 = target.join(
        &source
            .artifacts
            .iter()
            .find(|a| a.role == "dyn0")
            .context("data_invalid: 来源缺 dyn0")?
            .relative_path,
    );
    for q in &mut imported.qpoints {
        q.source_dyn0 = imported_dyn0.clone();
    }
    imported.verify()?;
    store::write(&target.join("manifest.json"), &imported)?;

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
    m.insert(
        "structure".to_string(),
        source.fingerprints.structure_sha256.clone(),
    );
    m.insert("n_qpoints".to_string(), source.qpoints.len().to_string());
    m.insert(
        "n_phonon_files".to_string(),
        source.phonon_files.len().to_string(),
    );
    Ok(m)
}
