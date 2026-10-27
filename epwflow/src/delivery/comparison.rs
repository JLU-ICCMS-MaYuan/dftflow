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
    serde_json::json!({"schema_version":1, "operation_status":"ok", "workflow_complete":false, "comparison_complete":data.manifest.missing.is_empty(), "differences":data.differences, "run_id":data.manifest.run_id, "missing":data.manifest.missing})
}

pub fn result_dir(work: &Path) -> PathBuf {
    if crate::foundation::layout::enabled(work) {
        work.join("results/6.epw-coarse")
    } else {
        crate::foundation::layout::path(work, "results")
    }
}

pub fn collect(work: &Path, output: Option<&Path>) -> Result<PathBuf> {
    let source = crate::foundation::layout::path(&work, "interpolation/comparison");
    let data = load(&source)?;
    let output = output
        .map(PathBuf::from)
        .unwrap_or_else(|| result_dir(work));
    fs::create_dir_all(&output)?;
    let _lock = store::Lock::acquire(&output.join("comparison"))?;
    if crate::foundation::layout::enabled(work)
        && output == result_dir(work)
        && output.join("comparison").is_dir()
        && store::hash_file(&source.join("manifest.json"))?
            != store::hash_file(&output.join("comparison/manifest.json"))?
    {
        fs::remove_dir_all(output.join("comparison"))?;
        let figures = output.join("figures");
        if figures.is_dir() {
            fs::remove_dir_all(figures)?;
        }
        let summary = output.join("comparison-summary.json");
        if summary.is_file() {
            fs::remove_file(summary)?;
        }
    }
    if output.join("comparison").is_dir() {
        ensure!(
            store::hash_file(&source.join("manifest.json"))?
                == store::hash_file(&output.join("comparison/manifest.json"))?,
            "input_conflict: 结果绑定其他来源；请使用新的 --output"
        );
        for file in &data.manifest.files {
            let target = output.join("comparison").join(&file.relative_path);
            if !target.exists() {
                fs::copy(source.join(&file.relative_path), &target)?;
            }
            file.verify(&output.join("comparison"))?;
        }
        load(&output.join("comparison"))?;
        if !output.join("comparison-summary.json").exists() {
            store::write(&output.join("comparison-summary.json"), &summary(&data))?;
        }
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
