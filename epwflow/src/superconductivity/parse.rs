use anyhow::{ensure, Context, Result};
use std::path::Path;
use super::super::interpolation::fine;

/// 谱数据：频率（meV）、α²F、有效耦合 λ 与对数平均频率（K）。
#[derive(Debug, Clone)]
pub struct Spectrum {
    pub frequencies_mev: Vec<f64>,
    pub a2f: Vec<f64>,
    pub lambda_eff: f64,
    pub omega_log_k: f64,
    pub cutoff_mev: f64,
}

/// 从插值谱读取 α²F 与频率（FR-003）。频率为 cm⁻¹，转换为 meV。
pub fn parse_spectrum(frequency_cm1_path: &Path, a2f_mev_path: &Path, cutoff_ev: f64) -> Result<Spectrum> {
    let freq_cm1 = fine::read_f64_col(frequency_cm1_path)
        .with_context(|| format!("data_invalid: 无法读取频率 {}", frequency_cm1_path.display()))?;
    let a2f = fine::read_f64_col(a2f_mev_path)
        .with_context(|| format!("data_invalid: 无法读取 α²F {}", a2f_mev_path.display()))?;
    ensure!(
        freq_cm1.len() == a2f.len() && !freq_cm1.is_empty(),
        "data_invalid: 频率与 α²F 长度不一致"
    );
    let cm1_to_mev = 0.123984;
    let frequencies_mev: Vec<f64> = freq_cm1.iter().map(|w| *w * cm1_to_mev).collect();
    compute_lambda(&frequencies_mev, &a2f, cutoff_ev * 1000.0)
}

/// 由频率与 α²F 计算有效耦合 λ 与对数平均频率 ω_log（K）。
pub fn compute_lambda(frequencies_mev: &[f64], a2f: &[f64], cutoff_mev: f64) -> Result<Spectrum> {
    // λ = 2 ∫ α²F(ω)/ω dω；ω_log = exp[(2/λ) ∫ α²F(ω)/ω ln ω dω]。
    let mut lambda = 0.0_f64;
    let mut lognum = 0.0_f64;
    for i in 0..frequencies_mev.len() {
        let w = frequencies_mev[i];
        if w <= 0.0 {
            continue;
        }
        let dw = if i + 1 < frequencies_mev.len() {
            (frequencies_mev[i + 1] - frequencies_mev[i]).abs()
        } else if i > 0 {
            (frequencies_mev[i] - frequencies_mev[i - 1]).abs()
        } else {
            1.0
        };
        let contrib = 2.0 * a2f[i] / w * dw;
        lambda += contrib;
        lognum += contrib * w.ln();
    }
    ensure!(lambda > 0.0, "data_invalid: 有效耦合 λ 非正");
    let omega_log_mev = (lognum / lambda).exp();
    let k2mev = 0.086173_f64;
    Ok(Spectrum {
        frequencies_mev: frequencies_mev.to_vec(),
        a2f: a2f.to_vec(),
        lambda_eff: lambda,
        omega_log_k: omega_log_mev / k2mev,
        cutoff_mev,
    })
}
