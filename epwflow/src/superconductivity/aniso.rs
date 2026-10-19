//! 双初值必须逐元素一致；只容许整个 Δ 数组同时翻转符号。
use super::{gap::GapData, search::Side};
use anyhow::{ensure, Result};

pub fn classify(a: &GapData, b: &GapData, normal_mev: f64, super_mev: f64) -> Result<Side> {
    a.validate()?;
    b.validate()?;
    ensure!(
        normal_mev.is_finite()
            && super_mev.is_finite()
            && normal_mev > 0.0
            && super_mev > normal_mev,
        "configuration: 能隙分类阈值无效"
    );
    ensure!(
        a.temperature_centikelvin == b.temperature_centikelvin
            && a.states == b.states
            && a.frequencies_mev == b.frequencies_mev,
        "data_invalid: 双初值物理索引不同"
    );
    let delta_match = |sign: f64| {
        a.delta_mev
            .iter()
            .zip(&b.delta_mev)
            .all(|(x, y)| (x - sign * y).abs() <= 1e-4 + 1e-3 * x.abs().max(y.abs()))
    };
    let z_match =
        a.z.iter()
            .zip(&b.z)
            .all(|(x, y)| (x - y).abs() <= 1e-5 + 1e-3 * x.abs().max(y.abs()));
    ensure!(
        (delta_match(1.0) || delta_match(-1.0)) && z_match,
        "classification_uncertain: 双初值完整 Δ/Z 不一致"
    );
    Ok(if a.d_inf() <= normal_mev && b.d_inf() <= normal_mev {
        Side::Normal
    } else if a.d0_inf() >= super_mev && b.d0_inf() >= super_mev {
        Side::Superconducting
    } else {
        Side::Uncertain
    })
}
