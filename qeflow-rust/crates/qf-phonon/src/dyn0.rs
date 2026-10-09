//! 复刻 `PHonon/PH/q_points.f90` 中 dyn0 的写出格式，并额外写出拆分用的 `q_index_map.json`。
//!
//! dyn0 坐标格式为 Fortran `(3e24.15)`：24 宽右对齐、15 位小数、两位指数、大写 `E`。
//! 该格式已被真实 `Mg1B2.dyn0` 字节级验证。

use crate::qgrid::{QGridParams, QPoint};
use serde::Serialize;
use std::path::Path;

/// `NINT` 的半数远离零行为由调用方在 qgrid 内处理；此处仅负责格式化。

/// 半数取偶取整（round-half-to-even），用于 E 格式的小数舍入，与 gfortran 一致。
fn round_half_even(x: f64) -> i64 {
    let floor = x.floor();
    let diff = x - floor;
    let r = if diff < 0.5 {
        floor
    } else if diff > 0.5 {
        floor + 1.0
    } else if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    };
    r as i64
}

/// 将双精度浮点数格式化为 Fortran `(e24.15)` 形式的 24 字符字符串（右对齐）。
///
/// 规则：
/// - 零值输出 `   0.000000000000000E+00`（含 3 个前导空格，24 宽）。
/// - 非零值写成 `±0.ddddddddddddddd E±dd`，mantissa ∈ [0.1, 1)，15 位小数，指数两位。
pub fn format_e24_15(v: f64) -> String {
    if v == 0.0 {
        return format!("{:>24}", "0.000000000000000E+00");
    }
    let negative = v < 0.0;
    let av = v.abs();
    // 选取指数 e 使 mantissa = av / 10^e ∈ [0.1, 1)。
    let mut e = av.log10().ceil() as i32;
    let mut m = av * 10f64.powi(-e);
    if m >= 1.0 {
        // mantissa 落入 [0.1,1) 需要提高一档指数。
        m /= 10.0;
        e += 1;
    }
    // 将 mantissa 四舍五入到 15 位小数（半数取偶，与 gfortran 的 E 格式一致）。
    let scaled = m * 1e15_f64;
    let mut digits = round_half_even(scaled);
    // 进位：舍入后 mantissa 达到 1.0 -> 表示为 0.1 × 10^(e+1)。
    if digits >= 1_000_000_000_000_000 {
        digits = 100_000_000_000_000; // 10^14：15 位（1 + 14 个 0）
        e += 1;
    }
    let digits_str = format!("{:015}", digits);
    let sign = if negative { "-" } else { "" };
    let exp_sign = if e < 0 { '-' } else { '+' };
    let mantissa = format!("{sign}0.{digits_str}E{exp_sign}{:02}", e.abs());
    format!("{mantissa:>24}")
}

/// 写出 `prefix.dyn0`：第一行 `(3i4)` 网格、第二行 `(i4)` 不可约点数、随后每行 `(3e24.15)`。
pub fn write_dyn0(
    path: &Path,
    qpoints: &[QPoint],
    nk1: i32,
    nk2: i32,
    nk3: i32,
) -> anyhow::Result<()> {
    let mut s = String::new();
    s.push_str(&format!("{:>4}{:>4}{:>4}\n", nk1, nk2, nk3));
    s.push_str(&format!("{:>4}\n", qpoints.len()));
    for q in qpoints {
        s.push_str(&format_e24_15(q.q_cart[0]));
        s.push_str(&format_e24_15(q.q_cart[1]));
        s.push_str(&format_e24_15(q.q_cart[2]));
        s.push('\n');
    }
    std::fs::write(path, s)?;
    Ok(())
}

#[derive(Serialize)]
struct QIndexEntry {
    directory: String,
    index: usize,
    q_crystal: [f64; 3],
    q_cart: [f64; 3],
    weight: f64,
    is_gamma: bool,
}

#[derive(Serialize)]
struct QIndexMap {
    mode: String,
    nk1: i32,
    nk2: i32,
    nk3: i32,
    qpoints: Vec<QIndexEntry>,
}

/// 写出 `q_index_map.json`：以晶体坐标作为拆分步骤的规范来源（ph.x 输入 `q` 为晶体坐标）。
pub fn write_q_index_map(path: &Path, qpoints: &[QPoint], params: &QGridParams) -> anyhow::Result<()> {
    let map = QIndexMap {
        mode: "kpoint_grid".to_string(),
        nk1: params.nk1,
        nk2: params.nk2,
        nk3: params.nk3,
        qpoints: qpoints
            .iter()
            .map(|q| QIndexEntry {
                directory: format!("{:02}", q.index),
                index: q.index,
                q_crystal: q.q_crystal,
                q_cart: q.q_cart,
                weight: q.weight,
                is_gamma: q.is_gamma,
            })
            .collect(),
    };
    let text = serde_json::to_string_pretty(&map)?;
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::format_e24_15;
    use crate::qgrid::nint_test as _nint;

    #[test]
    fn nint_half_away_from_zero() {
        assert_eq!(_nint(0.5), 1);
        assert_eq!(_nint(-0.5), -1);
        assert_eq!(_nint(2.3), 2);
        assert_eq!(_nint(-2.3), -2);
        assert_eq!(_nint(0.0), 0);
    }

    #[test]
    fn e24_15_zero_and_known() {
        assert_eq!(format_e24_15(0.0), "   0.000000000000000E+00");
        // 与真实 MgB2.dyn0 第二行 y 分量一致。
        assert_eq!(format_e24_15(-0.577350269189585), "  -0.577350269189585E+00");
        assert_eq!(format_e24_15(0.577350269189585), "   0.577350269189585E+00");
        assert_eq!(format_e24_15(1.0), "   0.100000000000000E+01");
        assert_eq!(format_e24_15(0.1), "   0.100000000000000E+00");
    }
}
