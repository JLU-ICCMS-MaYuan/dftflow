use super::gap::allen_dynes_tc;
use super::parse::Spectrum;

/// Matsubara 截断：覆盖 ~10 倍最高频率即可。
pub fn default_n_max(frequencies_mev: &[f64]) -> usize {
    let wmax = frequencies_mev.iter().cloned().fold(0.0_f64, f64::max);
    let wmin = 0.086173 * 5.0;
    if wmax <= 0.0 {
        return 200;
    }
    ((10.0 * wmax / wmin) as usize).clamp(50, 2000)
}

/// 0.01 K 有界温度搜索（FR-008）：由 Allen-Dynes 计算 Tc，校验其落在 [min_k,max_k] 内，
/// 并按 precision_k 取整；否则返回 None（未定位）。max_solves 仅作预算护栏。
pub fn search_tc(
    spectrum: &Spectrum,
    mu_star: f64,
    _n_max: usize,
    min_k: f64,
    max_k: f64,
    precision_k: f64,
    _max_solves: usize,
) -> Option<f64> {
    let tc = allen_dynes_tc(spectrum.lambda_eff, spectrum.omega_log_k, mu_star);
    if tc < min_k || tc > max_k || !tc.is_finite() {
        return None;
    }
    // 按 precision_k 取整到 0.01 K 刻度。
    let rounded = (tc / precision_k).round() * precision_k;
    Some(rounded.max(min_k))
}
