use super::parse::Spectrum;

/// k_B 以 meV/K 计（FR-007/008）。
pub const K2MEV: f64 = 0.086173;

/// 玻色子 Matsubara 频率（meV）：ω_n = (2n+1) π k_B T。
pub fn matsubara(t_k: f64, n_max: usize) -> Vec<f64> {
    let t_mev = K2MEV * t_k;
    (0..=n_max).map(|n| (2.0 * n as f64 + 1.0) * std::f64::consts::PI * t_mev).collect()
}

/// 电子-声子耦合在 Matsubara 轴：λ(ω_n) = 2 ∫ ω α²F(ω)/(ω² + ω_n²) dω（梯形积分）。
pub fn lambda_on_matsubara(frequencies_mev: &[f64], a2f: &[f64], omega_n: f64) -> f64 {
    let n = frequencies_mev.len();
    let mut s = 0.0_f64;
    for i in 0..n {
        let w = frequencies_mev[i];
        if w <= 0.0 {
            continue;
        }
        let dw = if i + 1 < n {
            (frequencies_mev[i + 1] - frequencies_mev[i]).abs()
        } else if i > 0 {
            (frequencies_mev[i] - frequencies_mev[i - 1]).abs()
        } else {
            1.0
        };
        s += 2.0 * w * a2f[i] / (w * w + omega_n * omega_n) * dw;
    }
    s
}

/// Allen-Dynes/McMillan 临界温度（K）：与温度无关的本征值判据（FR-004/008）。
pub fn allen_dynes_tc(lambda_eff: f64, omega_log_k: f64, mu_star: f64) -> f64 {
    let f1 = (1.0 + (lambda_eff / (2.46 * (1.0 + 3.8 * mu_star))).powf(1.5)).powf(1.0 / 3.0);
    let exponent = -1.04 * (1.0 + lambda_eff) / (lambda_eff - mu_star * (1.0 + 0.62 * lambda_eff));
    (f1 * omega_log_k / 1.20) * exponent.exp()
}

/// BCS 型能隙幅度（meV）：Δ(T) = 1.76 k_B Tc · tanh(1.74 √(Tc/T − 1))（FR-007）。
pub fn bcs_gap(t_k: f64, tc_k: f64) -> f64 {
    if tc_k <= 0.0 || t_k >= tc_k || t_k <= 0.0 {
        return 0.0;
    }
    let delta0 = 1.76 * K2MEV * tc_k;
    let arg = (tc_k / t_k - 1.0).sqrt();
    delta0 * arg.tanh() * 1.74_f64
}

/// 重整化函数 Z(ω_n) = 1 + πT Σ_m λ(ω_m)/√(Δ_m² + ω_n²)（FR-007）。
pub fn renormalization_z(delta: &[f64], omega: &[f64], spectrum: &Spectrum, _mu_star: f64, t_k: f64) -> Vec<f64> {
    let t_mev = K2MEV * t_k;
    let mut z = vec![0.0_f64; omega.len()];
    for (n, wn) in omega.iter().enumerate() {
        let mut acc = 0.0_f64;
        for (m, wm) in omega.iter().enumerate() {
            let lam = lambda_on_matsubara(&spectrum.frequencies_mev, &spectrum.a2f, *wm);
            acc += lam / (delta[m] * delta[m] + wn * wn).sqrt();
        }
        z[n] = 1.0 + std::f64::consts::PI * t_mev * acc;
    }
    z
}

/// 归一化幂迭代求解非线性能隙方程形状（FR-005/007）：与初值无关地收敛到主导本征矢。
/// 返回（形状向量，模为 1 的本征值）。
pub fn power_iterate_gap(
    t_k: f64,
    spectrum: &Spectrum,
    mu_star: f64,
    n_max: usize,
    seed: f64,
    max_iter: usize,
    conv_rel: f64,
) -> (Vec<f64>, f64) {
    let omega = matsubara(t_k, n_max);
    let t_mev = K2MEV * t_k;
    let mut delta = vec![seed; omega.len()];
    let mut lambda = 1.0_f64;
    for _ in 0..max_iter {
        let mut new_delta = vec![0.0_f64; omega.len()];
        for (n, wn) in omega.iter().enumerate() {
            let mut acc = 0.0_f64;
            for (m, wm) in omega.iter().enumerate() {
                let lam = lambda_on_matsubara(&spectrum.frequencies_mev, &spectrum.a2f, *wm);
                let w = if wm.abs() < spectrum.cutoff_mev { lam - mu_star } else { lam };
                acc += w * delta[m] / (delta[m] * delta[m] + wn * wn).sqrt();
            }
            new_delta[n] = std::f64::consts::PI * t_mev * acc;
        }
        let amax = new_delta.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
        if amax < 1e-12 {
            return (delta, lambda);
        }
        for v in new_delta.iter_mut() {
            *v /= amax;
        }
        let mut diff = 0.0_f64;
        for n in 0..delta.len() {
            diff += (new_delta[n] - delta[n]).abs();
            delta[n] = new_delta[n];
        }
        lambda = amax;
        if diff / (delta.iter().map(|x| x.abs()).sum::<f64>() + 1e-12) < conv_rel {
            break;
        }
    }
    (delta, lambda)
}
