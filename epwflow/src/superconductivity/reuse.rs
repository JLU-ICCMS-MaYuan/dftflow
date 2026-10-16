use anyhow::{ensure, Result};
use std::path::Path;
use super::manifest::SuperconductivityManifest;

/// 复用前置超导运行（FR-010）：导入并校验已发布的超导清单。
pub fn import_prior(source: &Path) -> Result<SuperconductivityManifest> {
    ensure!(source.exists(), "data_invalid: 前置超导清单缺失 {}", source.display());
    let m = SuperconductivityManifest::load(source)
        .map_err(|e| anyhow::anyhow!("data_invalid: 无法加载前置超导清单: {e}"))?;
    ensure!(
        m.validation.all_passed(),
        "data_invalid: 前置超导清单本地校验未通过，拒绝复用"
    );
    Ok(m)
}
