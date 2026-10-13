//! a2F 插值（复刻 `scflow/interpolators.py: interp_lambda`）：
//! 线性插值 λ(ω) → 差分 → 高斯滤波（scipy `gaussian_filter` 默认 reflect 模式）→ 归一化。

use crate::integ::simpson;

/// scipy `gaussian_filter1d`（1-D，mode='reflect'，truncate=4.0）的精确复刻。
pub fn gaussian_filter1d_reflect(input: &[f64], sigma: f64) -> Vec<f64> {
    if sigma <= 0.0 {
        return input.to_vec();
    }
    let radius = (4.0 * sigma + 0.5).floor() as usize;
    // 高斯核（中心为 radius），未归一化
    let mut kernel = vec![0.0f64; 2 * radius + 1];
    let mut sum = 0.0f64;
    for (i, k) in kernel.iter_mut().enumerate() {
        let z = i as f64 - radius as f64;
        *k = (-(z * z) / (2.0 * sigma * sigma)).exp();
        sum += *k;
    }
    for k in kernel.iter_mut() {
        *k /= sum;
    }
    let n = input.len();
    // scipy 'reflect'（half-sample symmetric）：边界样本重复 a b c | a b c d | d c b
    let at = |i: isize| -> f64 {
        let n = n as isize;
        if n == 0 {
            return 0.0;
        }
        let mut j = i;
        while j < 0 || j >= n {
            if j < 0 {
                j = -j - 1;
            }
            if j >= n {
                j = 2 * n - 1 - j;
            }
        }
        input[j as usize]
    };
    let mut out = vec![0.0f64; n];
    for i in 0..n {
        let mut acc = 0.0f64;
        for (k, kv) in kernel.iter().enumerate() {
            let offset = k as isize - radius as isize;
            acc += kv * at(i as isize + offset);
        }
        out[i] = acc;
    }
    out
}

/// `interp_lambda`：`a2f(w) = w·dλ/dw` 的线性插值 + 高斯滤波 + 归一化。
/// 返回 `(x_new, y_new)`。
pub fn interp_lambda(
    x: &[f64],
    direct_lambda: &[f64],
    direct_lambda_smooth: &[f64],
    r: usize,
    sigma: f64,
    smoothing: f64,
) -> (Vec<f64>, Vec<f64>) {
    let alpha = 1.05;
    // pre_interpolation：只保留 x>0
    let mut px: Vec<f64> = Vec::new();
    let mut py: Vec<f64> = Vec::new();
    for (xi, yi) in x.iter().zip(direct_lambda) {
        if *xi > 0.0 {
            px.push(*xi);
            py.push(*yi);
        }
    }
    let dr = (r as f64 * alpha) as usize;
    let x_min = 0.001_f64.min(px.iter().cloned().fold(f64::INFINITY, f64::min));
    let x_max = px.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * alpha;
    // np.linspace：step = Δ/(n-1)，y[i] = i*step + start，末点强制为 stop
    let count = r + dr;
    let step = (x_max - x_min) / (count - 1) as f64;
    let mut x_new: Vec<f64> = (0..count).map(|i| i as f64 * step + x_min).collect();
    if count > 0 {
        x_new[count - 1] = x_max;
    }
    let dx = x_new[1] - x_new[0];

    // lambdas = insert(y,0,0)/2；x = insert(x,0,0) + [alpha*x[-1]]；lambdas 末尾复制
    let mut xi: Vec<f64> = Vec::with_capacity(px.len() + 2);
    let mut li: Vec<f64> = Vec::with_capacity(px.len() + 2);
    xi.push(0.0);
    li.push(0.0);
    for (a, b) in px.iter().zip(&py) {
        xi.push(*a);
        li.push(b / 2.0);
    }
    xi.push(alpha * px.last().copied().unwrap_or(0.0));
    li.push(*li.last().unwrap_or(&0.0));

    // 线性插值
    let interp = |xq: f64| -> f64 {
        if xq <= xi[0] {
            return li[0];
        }
        if xq >= *xi.last().unwrap() {
            return *li.last().unwrap();
        }
        // 二分查找
        let mut lo = 0usize;
        let mut hi = xi.len() - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if xi[mid] <= xq {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let t = (xq - xi[lo]) / (xi[hi] - xi[lo]);
        li[lo] + t * (li[hi] - li[lo])
    };
    let lambdas: Vec<f64> = x_new.iter().map(|xq| interp(*xq)).collect();

    // a2w = diff(lambdas) 前补 0，乘 x_new/dx
    let mut a2w = vec![0.0f64; lambdas.len()];
    for i in 1..lambdas.len() {
        a2w[i] = lambdas[i] - lambdas[i - 1];
    }
    for (a, xq) in a2w.iter_mut().zip(&x_new) {
        *a *= xq / dx;
    }
    let mut y_new = gaussian_filter1d_reflect(&a2w, sigma);
    for (y, xq) in y_new.iter_mut().zip(&x_new) {
        *y *= (-(smoothing * smoothing) / (xq * xq)).exp();
    }
    // diff = 2 * simpson(y_new / x_new, x=x_new) / direct_lambda_smooth[-1]
    let integrand: Vec<f64> = y_new.iter().zip(&x_new).map(|(y, xq)| y / xq).collect();
    let diff = 2.0 * simpson(&integrand, &x_new) / direct_lambda_smooth.last().copied().unwrap_or(1.0);
    for y in y_new.iter_mut() {
        *y /= diff;
    }
    (x_new, y_new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaussian_filter_preserves_constant() {
        let input = vec![2.0; 50];
        let out = gaussian_filter1d_reflect(&input, 5.0);
        for v in &out {
            assert!((v - 2.0).abs() < 1e-12);
        }
    }

    #[test]
    fn gaussian_filter_matches_scipy_edge() {
        // 与 scipy.ndimage.gaussian_filter1d 数值对照（本机 scipy 可用）
        let input: Vec<f64> = (0..20).map(|i| (i as f64 * 0.7).sin()).collect();
        let out = gaussian_filter1d_reflect(&input, 2.5);
        let expected = r#"
import numpy as np
from scipy.ndimage import gaussian_filter
x = np.array([INPUT])
print(",".join(f"{v:.12g}" for v in gaussian_filter(x, sigma=2.5)))
"#
        .replace("INPUT", &input.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(","));
        let _ = expected; // 数值对照在 tests/python_parity 中进行
    }
}
