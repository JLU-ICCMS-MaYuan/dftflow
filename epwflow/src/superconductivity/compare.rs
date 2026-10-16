/// F4 数值一致性容差（FR-009）。

/// Tc 比较容差（K）。
pub const TOL_TC_K: f64 = 0.1;
/// 能隙比较容差（meV）。
pub const TOL_GAP_MEV: f64 = 0.05;

/// 两个 Tc 是否一致（均在界内且差 ≤ 容差，或均界外未定位）。
pub fn tc_close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => (x - y).abs() <= TOL_TC_K,
        (None, None) => true,
        _ => false,
    }
}

/// 能隙集合（按大小）是否一致。
pub fn gaps_close(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut sa = a.to_vec();
    let mut sb = b.to_vec();
    sa.sort_by(|x, y| x.partial_cmp(y).unwrap());
    sb.sort_by(|x, y| x.partial_cmp(y).unwrap());
    sa.iter().zip(sb.iter()).all(|(x, y)| (x - y).abs() <= TOL_GAP_MEV)
}
