//! 超导转变温度求解：McMillan、Allen-Dynes 闭式公式与 Eliashberg 矩阵本征值法。
//! 复刻 `scflow/tc.py` 与 `scflow/sc_e.py: Superconducting.get_tc_e`。

use crate::a2f::A2f;
use crate::constants::K_T_THZ;
use crate::eig::max_eigenvalue;
use crate::integ::simpson;
use crate::Integration;
use ndarray::Array2;

/// McMillan Tc（闭式）。`den <= 0` 时返回 0（对应 Python 中零填充分支）。
pub fn mc_millan_tc(wlog: f64, lambda: f64, mu: f64) -> f64 {
    let den = lambda * (1.0 - 0.62 * mu) - mu;
    if den <= 0.0 {
        return 0.0;
    }
    wlog / 1.2 * (-1.04 * (1.0 + lambda) / den).exp()
}

/// Allen-Dynes Tc（闭式）。
pub fn allen_dynes_tc(wlog: f64, w2: f64, lambda: f64, mu: f64) -> f64 {
    if wlog <= 0.0 {
        return 0.0;
    }
    let f1 = (1.0 + (lambda / (2.46 * (1.0 + 3.8 * mu))).powf(1.5)).cbrt();
    let denom = lambda * lambda + 3.312 * (1.0 + 6.3 * mu).powi(2);
    let f2 = 1.0 - (lambda * lambda * (1.0 - w2 / wlog)) / denom;
    f1 * f2 * mc_millan_tc(wlog, lambda, mu)
}

/// Eliashberg 核 `l(r, t)`：∫ 2·α²F(ω)·ω / (ω² + (r·t·k_K_THz·2π)²) dω。
/// 复刻 `scflow/sc_e.py: l`，使用 gamma 通道 α²F。
fn eliashberg_kernel(a2f: &A2f, r: usize, t: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let c = (r as f64) * t * K_T_THZ * 2.0 * pi;
    let integrand: Vec<f64> = a2f
        .freqs
        .iter()
        .zip(a2f.a2f_gamma.iter())
        .map(|(w, a)| 2.0 * a * w / (w * w + c * c))
        .collect();
    simpson(&integrand, &a2f.freqs)
}

/// Eliashberg Tc：在给定温度 T 构造 (dim+1)×(dim+1) 对称矩阵 K，
/// 其最大本征值在超导态（T<Tc）为正、在常态（T>Tc）为负；
/// 通过二分搜索定位 b=0 处即为 Tc。复刻 `scflow/sc_e.py: get_tc_e`。
///
/// 返回值：`(tc, status)`，其中 `status` 为 `"resolved"` 或 `"resolution_limit"`。
pub fn eliashberg_tc(
    a2f: &A2f,
    mu: f64,
    dim: usize,
    _integration: Integration,
) -> (f64, String) {
    let size = dim + 1;
    let mut t = 0.0_f64;
    let mut dt = 10.0_f64;
    while dt > 0.001 {
        let mut b = 100.0_f64;
        while b > 0.0 {
            b = max_eig_at_t(a2f, mu, dim, size, t);
            if b > 0.0 {
                t += dt;
            }
        }
        if t > 0.0 {
            t -= dt;
        }
        if dt > 0.001 {
            dt *= 0.1;
        }
        if dt < 0.001 {
            dt = 0.001;
        }
    }
    let mut tc_e = t - dt;
    let status = if tc_e <= 0.001 {
        tc_e = 0.001;
        "resolution_limit".to_string()
    } else {
        "resolved".to_string()
    };
    (tc_e, status)
}

/// 在温度 `t` 计算 K 矩阵最大本征值。
fn max_eig_at_t(a2f: &A2f, mu: f64, dim: usize, size: usize, t: f64) -> f64 {
    // 预计算 l(r, t) for r in 0..=2*dim+1（m,n ≤ dim+1 时 m+n-1 最大为 2*dim+1）
    let mut ks = vec![0.0; 2 * dim + 2];
    for r in 0..=2 * dim + 1 {
        ks[r] = eliashberg_kernel(a2f, r, t);
    }
    // s[y-1] = Σ_{r=1..=y} l(r, t)
    let mut s = vec![0.0_f64; dim];
    let mut acc = 0.0;
    for y in 1..=dim {
        acc += ks[y];
        s[y - 1] = acc;
    }
    let mut k = Array2::<f64>::zeros((size, size));
    for n in 1..=size {
        for m in 1..=size {
            let d = if n == m {
                if m == 1 {
                    1.0 + ks[0]
                } else {
                    2.0 * (m as f64 - 1.0) + 1.0 + ks[0] + 2.0 * s[m - 2]
                }
            } else {
                0.0
            };
            let r_diff = (m as isize - n as isize).unsigned_abs() as usize;
            let k_val = ks[r_diff] + ks[m + n - 1] - 2.0 * mu - d;
            k[[n - 1, m - 1]] = k_val;
        }
    }
    max_eigenvalue(&k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mc_millan_positive_for_strong_coupling() {
        // λ=1, wlog=100K, μ=0.1 -> 应有正 Tc
        let tc = mc_millan_tc(100.0, 1.0, 0.1);
        assert!(tc > 0.0, "Tc 应为正，实际 {tc}");
    }

    #[test]
    fn mc_millan_zero_when_den_nonpositive() {
        assert_eq!(mc_millan_tc(100.0, 0.5, 0.13), 0.0);
    }

    #[test]
    fn allen_dynes_reduces_to_mcmillan_for_harmonic() {
        // w2 == wlog（谐振近似）时 Allen-Dynes 退化为 McMillan 乘 f1（>1）
        let wlog = 100.0;
        let lambda = 1.0;
        let mu = 0.1;
        let tc_ad = allen_dynes_tc(wlog, wlog, lambda, mu);
        let tc_mcm = mc_millan_tc(wlog, lambda, mu);
        assert!(tc_ad > tc_mcm);
    }
}
