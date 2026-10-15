use anyhow::{bail, ensure, Context, Result};
use std::path::Path;
use super::super::foundation::store;
use super::manifest::InterpolationManifest;

/// 导入报告（FR-010）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReport {
    pub schema_version: u32,
    pub source_run: String,
    pub imported_roles: Vec<String>,
    pub status: String,
}

/// 从已有插值运行导入已验证的粗/细数据到本运行隔离树（FR-010）。
/// 仅当来源清单 status=valid 且角色经重验（哈希一致）才登记为可复用阶段。
pub fn import_from_run(source_work: &Path, work: &Path) -> Result<ImportReport> {
    let src = source_work.join("interpolation").join("manifest.json");
    let source = InterpolationManifest::load(&src)
        .with_context(|| format!("data_invalid: 无法加载来源插值清单 {}", src.display()))?;
    ensure!(source.validation.all_passed(), "data_invalid: 来源插值清单未通过，拒绝复用");

    let target = work.join("imported").join("interpolation");
    std::fs::create_dir_all(&target)?;
    let mut roles = Vec::new();
    let mut copy_role = |role: &str, p: &Path| -> Result<()> {
        ensure!(p.exists(), "data_invalid: 来源角色 {role} 缺失 {}", p.display());
        let dest = target.join(p.file_name().context("data_invalid: 缺文件名")?);
        std::fs::copy(p, &dest).with_context(|| format!("data_invalid: 复制 {role} 失败"))?;
        let a = store::hash_file(p)?;
        let b = store::hash_file(&dest)?;
        ensure!(a == b, "data_invalid: 复制前后哈希不一致 {role}");
        roles.push(role.to_string());
        Ok(())
    };
    if let Some(c) = &source.coarse {
        for art in &c.artifacts {
            copy_role(&art.role, Path::new(&art.relative_path))?;
        }
    }
    if let Some(f) = &source.fine {
        for art in &f.artifacts {
            copy_role(&art.role, Path::new(&art.relative_path))?;
        }
    }
    let report = ImportReport {
        schema_version: 1,
        source_run: source.run_id,
        imported_roles: roles.clone(),
        status: if roles.is_empty() { "rejected" } else { "imported" }.to_string(),
    };
    if report.status == "rejected" {
        bail!("data_invalid: 无可复用的粗/细数据");
    }
    store::write(&work.join("imported").join("interpolation-import-report.json"), &report)?;
    Ok(report)
}
