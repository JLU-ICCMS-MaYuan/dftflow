//! 消费 F3 比较数据，发布独立结果包；不要求 F4 产物。
use crate::foundation::store;
pub use crate::interpolation::comparison_data::*;
use anyhow::{ensure, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub const KINDS: [&str; 5] = [
    "electronic_bands_comparison",
    "electronic_dos_comparison",
    "phonon_dispersion_comparison",
    "phonon_dos_comparison",
    "comparison_overview",
];

pub fn summary(data: &Dataset) -> serde_json::Value {
    serde_json::json!({"schema_version":1, "operation_status":"ok", "workflow_complete":false, "comparison_complete":data.manifest.missing.is_empty(), "quality_status":data.manifest.quality_status, "run_id":data.manifest.run_id, "missing":data.manifest.missing})
}

pub fn collect(work: &Path, output: Option<&Path>) -> Result<PathBuf> {
    let source = work.join("interpolation/comparison");
    let data = load(&source)?;
    let output = output
        .map(PathBuf::from)
        .unwrap_or_else(|| work.join("results"));
    fs::create_dir_all(&output)?;
    let _lock = store::Lock::acquire(&output.join("comparison"))?;
    if output.join("comparison").is_dir() {
        load(&output.join("comparison"))?;
        ensure!(
            store::hash_file(&source.join("manifest.json"))?
                == store::hash_file(&output.join("comparison/manifest.json"))?,
            "input_conflict: 结果绑定其他来源；请使用新的 --output"
        );
        return Ok(output);
    }
    ensure!(
        !output.join("comparison").exists() && !output.join("comparison-summary.json").exists(),
        "input_conflict: 结果已存在；使用新的 --output，避免覆盖"
    );
    let stage = output.join(format!(".comparison-{}", store::token()));
    fs::create_dir(&stage)?;
    for file in &data.manifest.files {
        fs::copy(
            source.join(&file.relative_path),
            stage.join(&file.relative_path),
        )?;
    }
    fs::copy(source.join("manifest.json"), stage.join("manifest.json"))?;
    load(&stage)?;
    fs::rename(stage, output.join("comparison"))?;
    store::write(&output.join("comparison-summary.json"), &summary(&data))?;
    Ok(output)
}
