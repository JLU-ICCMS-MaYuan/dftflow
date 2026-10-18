//! 原生超导阶段的交付边界。未接通 EPW 求解时不得用经验公式发布结果。
use super::manifest::SuperconductivityManifest;
use crate::{
    foundation::{config, stage::Backend},
    interpolation::manifest::InterpolationManifest,
};
use anyhow::{bail, ensure, Result};
use std::path::Path;

pub fn run(
    _cfg: &config::Config,
    work: &Path,
    _launcher: &[String],
    _backend: Backend,
) -> Result<SuperconductivityManifest> {
    let path = work.join("interpolation/manifest.json");
    let f3 = InterpolationManifest::load(&path)?;
    ensure!(
        f3.validation.all_passed() && f3.quality.as_ref().is_some_and(|q| q.status == "valid"),
        "data_invalid: 插值质量尚未通过，F4 不得放行"
    );
    bail!("not_implemented: 原生 EPW Eliashberg 执行和完整能隙/收敛证据尚未接通；未发布 Tc 或能隙结果")
}
