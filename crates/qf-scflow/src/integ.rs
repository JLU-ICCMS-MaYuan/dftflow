//! 数值积分工具（对齐 `scflow/compat.py: simpson`，精确复刻 `scipy.integrate.simpson`）。

/// `_basic_simpson`：从 `start` 到 `stop`（含 `stop+1` 索引）按步长 2 配对，支持非均匀网格。
fn basic_simpson(y: &[f64], x: &[f64], start: usize, stop: usize) -> f64 {
    let mut result = 0.0;
    let mut i = start;
    while i + 2 <= stop + 1 {
        let h0 = x[i + 1] - x[i];
        let h1 = x[i + 2] - x[i + 1];
        let hsum = h0 + h1;
        result += hsum
            / 6.0
            * (y[i] * (2.0 - h0 / h1)
                + y[i + 1] * (hsum * hsum / (h0 * h1))
                + y[i + 2] * (2.0 - h0 / h1));
        i += 2;
    }
    result
}

/// 复合 Simpson 积分（对齐 `scipy.integrate.simpson`）：
/// 奇数样本走标准复合 Simpson；偶数样本（奇区间）末段加 Cartwright 修正。
pub fn simpson(y: &[f64], x: &[f64]) -> f64 {
    let n = y.len();
    if n < 2 {
        return 0.0;
    }
    if n == 2 {
        return 0.5 * (x[n - 1] - x[n - 2]) * (y[n - 1] + y[n - 2]);
    }
    if n % 2 == 1 {
        return basic_simpson(y, x, 0, n - 2);
    }
    // 偶数样本：Simpson 覆盖 0..=n-2，末区间用 Cartwright 修正
    let mut result = basic_simpson(y, x, 0, n - 3);
    let h0 = x[n - 2] - x[n - 3];
    let h1 = x[n - 1] - x[n - 2];
    let alpha = (2.0 * h1 * h1 + 3.0 * h0 * h1) / (6.0 * (h1 + h0));
    let beta = (h1 * h1 + 3.0 * h0 * h1) / (6.0 * h0);
    let eta = (h1 * h1 * h1) / (6.0 * h0 * (h0 + h1));
    result += alpha * y[n - 1] + beta * y[n - 2] - eta * y[n - 3];
    result
}

/// 累积 Simpson 积分：`out[i] = ∫_{x_0}^{x_i} y dx`。
pub fn cumulative_simpson(y: &[f64], x: &[f64]) -> Vec<f64> {
    let n = y.len();
    let mut out = vec![0.0; n];
    for i in 1..n {
        out[i] = simpson(&y[0..=i], &x[0..=i]);
    }
    out
}

/// 累积求和积分（对齐 `scflow` 的 `sum` 模式）：
/// `out[i] = Σ_{j=0}^{i} y[j] * dx`，`dx` 取均匀网格步长。
pub fn cumulative_sum(y: &[f64], dx: f64) -> Vec<f64> {
    let n = y.len();
    let mut out = vec![0.0; n];
    let mut acc = 0.0;
    for i in 0..n {
        acc += y[i] * dx;
        out[i] = acc;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simpson_linear_is_trapezoid() {
        // ∫ 0..2 x dx = 2
        let x = [0.0, 1.0, 2.0];
        let y = [0.0, 1.0, 2.0];
        assert!((simpson(&y, &x) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn simpson_quadratic_exact() {
        // ∫ 0..2 x^2 dx = 8/3
        let x = [0.0, 1.0, 2.0];
        let y = [0.0, 1.0, 4.0];
        assert!((simpson(&y, &x) - 8.0 / 3.0).abs() < 1e-9);
    }
}
