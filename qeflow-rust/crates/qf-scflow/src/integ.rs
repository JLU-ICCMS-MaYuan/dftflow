//! 数值积分工具（对齐 `scflow/compat.py: simpson`，复刻 SciPy composite Simpson）。

/// 复合 Simpson 积分（对齐 `scipy.integrate.simpson` 的均匀网格行为）。
/// 当点数为偶数（区间数为奇数）时，末段回退梯形法，与 SciPy 一致。
pub fn simpson(y: &[f64], x: &[f64]) -> f64 {
    let n = y.len();
    if n < 2 {
        return 0.0;
    }
    let h = x[1] - x[0];
    if n == 2 {
        return h * (y[0] + y[1]) / 2.0;
    }
    let mut total = h / 3.0 * (y[0] + y[n - 1]);
    for i in 1..n - 1 {
        let w = if i % 2 == 1 { 4.0 } else { 2.0 };
        total += h / 3.0 * w * y[i];
    }
    if n % 2 == 0 {
        // 区间数为奇数：末段用梯形法替代 Simpson
        let trap_last = h * (y[n - 2] + y[n - 1]) / 2.0;
        let simp_last = h / 3.0 * (y[n - 3] + 4.0 * y[n - 2] + y[n - 1]);
        total = total - simp_last + trap_last;
    }
    total
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
