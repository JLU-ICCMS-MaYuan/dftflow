/// F3 数值一致性容差与误差统计（FR-009）。

/// 频率最大绝对误差与均方根误差（cm⁻¹）。
pub fn error_stats(reference: &[f64], computed: &[f64]) -> (f64, f64) {
    let mut max = 0.0_f64;
    let mut s = 0.0_f64;
    let n = reference.len().max(1) as f64;
    for (a, b) in reference.iter().zip(computed.iter()) {
        let d = (a - b).abs();
        if d > max {
            max = d;
        }
        s += d * d;
    }
    (max, (s / n).sqrt())
}

/// 矩阵元/能量逐元素最大绝对差（FR-009 一致性）。
pub fn max_abs(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).fold(None, |acc, d| match acc {
        None => Some(d),
        Some(m) => Some(m.max(d)),
    })
}

/// 权重一致性：记录原始约定后归一化比较（FR-009）。
pub fn weights_match(a: &[f64], b: &[f64], tol: f64) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    let sa: f64 = a.iter().sum();
    let sb: f64 = b.iter().sum();
    if sa.abs() < 1e-300 || sb.abs() < 1e-300 {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| ((x / sa) - (y / sb)).abs() <= tol)
}
