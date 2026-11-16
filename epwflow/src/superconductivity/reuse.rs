use anyhow::{ensure, Result};
use std::path::Path;
use super::manifest::SuperconductivityManifest;

/// 复用前置超导运行（FR-010）：导入并校验已发布的超导清单。
pub fn import_prior(source: &Path) -> Result<SuperconductivityManifest> {
    ensure!(source.exists(), "data_invalid: preceding superconductivity manifest missing {}", source.display());
    let m = SuperconductivityManifest::load(source)
        .map_err(|e| anyhow::anyhow!("data_invalid: cannot load preceding superconductivity manifest: {e}"))?;
    ensure!(
        m.validation.all_passed(),
        "data_invalid: preceding superconductivity manifest failed local validation; reuse refused"
    );
    Ok(m)
}
