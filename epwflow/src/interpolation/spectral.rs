use anyhow::{ensure, Result};
use super::manifest::Spectrum;

/// 低频规则：|ω| 低于声学截止的频率保留符号并夹到 ±eps，避免除零/污染 λ（FR-008）。
pub fn apply_low_frequency_rule(frequencies_cm1: &[f64], eps_acoustic_cm1: f64) -> Vec<f64> {
    frequencies_cm1
        .iter()
        .map(|w| {
            if w.abs() < eps_acoustic_cm1 {
                if w == &0.0 {
                    0.0
                } else {
                    w.signum() * eps_acoustic_cm1
                }
            } else {
                *w
            }
        })
        .collect()
}

/// 由 α²F(ω) 与频率计算 Eliashberg 谱：λ = 2 ∫ α²F(ω)/ω dω（梯形积分，FR-008）。
pub fn compute_spectrum(
    frequencies_cm1: &[f64],
    a2f: &[f64],
    smearing_id: &str,
    cutoff_ev: f64,
    eps_acoustic_cm1: f64,
) -> Result<Spectrum> {
    ensure!(
        frequencies_cm1.len() == a2f.len(),
        "data_invalid: 频率与 α²F 长度不一致"
    );
    let w = apply_low_frequency_rule(frequencies_cm1, eps_acoustic_cm1);
    let mut lambda_integral = 0.0_f64;
    for i in 1..w.len() {
        let dw = (w[i] - w[i - 1]).abs();
        let f = a2f[i] / w[i].max(eps_acoustic_cm1);
        let fprev = a2f[i - 1] / w[i - 1].max(eps_acoustic_cm1);
        lambda_integral += 0.5 * (f + fprev) * dw;
    }
    let lambda_integral = 2.0 * lambda_integral;
    let lambda_sum: f64 = a2f.iter().sum();
    Ok(Spectrum {
        smearing_id: smearing_id.to_string(),
        energy_mev: frequencies_cm1.iter().map(|x| *x / 8.06554).collect(), // cm⁻¹ → meV 近似
        a2f: a2f.to_vec(),
        lambda_integral,
        lambda_sum,
        cutoff_ev,
    })
}
