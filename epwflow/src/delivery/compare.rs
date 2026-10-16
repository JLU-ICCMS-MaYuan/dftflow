/// F5 交付数值一致性容差（FR-005）。

/// Tc 比较容差（K）。
pub const TOL_TC_K: f64 = 0.1;
/// 能隙比较容差（meV）。
pub const TOL_GAP_MEV: f64 = 0.05;
/// λ 比较容差（相对）。
pub const TOL_LAMBDA: f64 = 0.02;

/// 两个 Tc 是否一致。
pub fn tc_close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => (x - y).abs() <= TOL_TC_K,
        (None, None) => true,
        _ => false,
    }
}

/// 两个 λ 是否一致（相对容差）。
pub fn lambda_close(a: f64, b: f64) -> bool {
    if a.abs() < 1e-12 && b.abs() < 1e-12 {
        return true;
    }
    let denom = a.abs().max(b.abs());
    (a - b).abs() / denom <= TOL_LAMBDA
}
