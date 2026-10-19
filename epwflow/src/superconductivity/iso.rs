//! 仅分类原生线性求解的最大本征值；不从 α²F 估算正式 Tc。
use super::search::Side;
use anyhow::{ensure, Result};

pub fn classify(eigenvalue: f64, margin: f64) -> Result<Side> {
    ensure!(
        eigenvalue.is_finite() && margin.is_finite() && margin > 0.5e-7,
        "data_invalid: 线性本征值或舍入裕量无效"
    );
    Ok(if eigenvalue > 1.0 + margin {
        Side::Superconducting
    } else if eigenvalue < 1.0 - margin {
        Side::Normal
    } else {
        Side::Uncertain
    })
}
