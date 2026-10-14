//! 复刻 `PW/src/kpoint_grid.f90` 的不可约 q 点（k 点）网格折叠算法。
//!
//! 与 Fortran 源码逐操作对齐，保证：整数网格的枚举、对称旋转映射、半数远离零的 `NINT`、
//! 以及最终的 `cryst_to_cart` 都产生与 `ph.x` 完全相同的双精度值与坐标。

use crate::xml::QeGridInfo;

/// 单个不可约 q 点（同时保留晶体坐标与笛卡尔坐标）。
#[derive(Debug, Clone)]
pub struct QPoint {
    /// 1-based 不可约序号。
    pub index: usize,
    /// 晶体坐标（第一布里渊区归一化）。用于坐标对照；ph.x 显式 q 输入使用下方笛卡尔坐标。
    pub q_crystal: [f64; 3],
    /// 笛卡尔坐标（tpiba = 2π/a₀）。dyn0 文件使用此坐标（与 ph.x 字节一致）。
    pub q_cart: [f64; 3],
    /// 归一化后的权重（所有不可约点权重之和 = 1）。
    pub weight: f64,
    /// 是否为 Γ 点（dyn0 第一个点）。
    pub is_gamma: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct QGridParams {
    pub nk1: i32,
    pub nk2: i32,
    pub nk3: i32,
    /// 网格偏移（Monkhorst-Pack 半偏移），声子网格通常为 0。
    pub k1: i32,
    pub k2: i32,
    pub k3: i32,
    /// 覆盖 time_reversal；为 None 时使用 XML 中的值。
    pub time_reversal: Option<bool>,
}

impl Default for QGridParams {
    fn default() -> Self {
        QGridParams {
            nk1: 1,
            nk2: 1,
            nk3: 1,
            k1: 0,
            k2: 0,
            k3: 0,
            time_reversal: None,
        }
    }
}

/// 半数远离零取整（与 GFortran 的 `NINT` 及 Rust `f64::round()` 一致）。
fn nint(x: f64) -> i32 {
    x.round() as i32
}

/// 测试用：暴露 `nint`。
#[cfg(test)]
pub(crate) fn nint_test(x: f64) -> i32 {
    nint(x)
}

/// 晶体坐标 -> 笛卡尔坐标，复刻 `cryst_to_cart(iflag=1)`：`out(k) = Σ_v bg(v,k) * crystal(v)`。
fn cryst_to_cart(bg: &[[f64; 3]; 3], crystal: &[f64; 3]) -> [f64; 3] {
    let mut out = [0.0_f64; 3];
    for c in 0..3 {
        out[c] = bg[0][c] * crystal[0] + bg[1][c] * crystal[1] + bg[2][c] * crystal[2];
    }
    out
}

pub fn generate_q_points(info: &QeGridInfo, p: &QGridParams) -> Vec<QPoint> {
    let nrot = info.rotations.len();
    let (nk1, nk2, nk3) = (p.nk1, p.nk2, p.nk3);
    let nkr = (nk1 * nk2 * nk3) as usize;
    // xkg 使用 1-based 索引，下标 1..=nkr。
    let mut xkg = vec![[0.0_f64; 3]; nkr + 1];

    for i in 1..=nk1 {
        for j in 1..=nk2 {
            for k in 1..=nk3 {
                let n = ((k - 1) + (j - 1) * nk3 + (i - 1) * nk2 * nk3 + 1) as usize;
                xkg[n][0] = (i - 1) as f64 / nk1 as f64 + p.k1 as f64 / 2.0 / nk1 as f64;
                xkg[n][1] = (j - 1) as f64 / nk2 as f64 + p.k2 as f64 / 2.0 / nk2 as f64;
                xkg[n][2] = (k - 1) as f64 / nk3 as f64 + p.k3 as f64 / 2.0 / nk3 as f64;
            }
        }
    }

    let mut equiv = (0..=(nkr as i32)).collect::<Vec<i32>>();
    let mut wkk = vec![0.0_f64; nkr + 1];
    let eps = 1.0e-5_f64;

    for nk in 1..=nkr as i32 {
        if equiv[nk as usize] != nk {
            continue;
        }
        wkk[nk as usize] = 1.0;
        for ns in 0..nrot {
            let s = &info.rotations[ns];
            // t_rev(ns) 对非磁性体系恒为 0，不做符号翻转；以下只在 time_reversal 时加入 -k 分支。
            let mut xkr = [0.0_f64; 3];
            for ii in 0..3 {
                xkr[ii] = s[ii][0] as f64 * xkg[nk as usize][0]
                    + s[ii][1] as f64 * xkg[nk as usize][1]
                    + s[ii][2] as f64 * xkg[nk as usize][2];
                xkr[ii] -= nint(xkr[ii]) as f64;
            }
            let signs: &[f64] = if p.time_reversal.unwrap_or(info.time_reversal) {
                &[1.0, -1.0]
            } else {
                &[1.0]
            };
            for &sign in signs {
                let xx = sign * xkr[0] * nk1 as f64 - 0.5 * p.k1 as f64;
                let yy = sign * xkr[1] * nk2 as f64 - 0.5 * p.k2 as f64;
                let zz = sign * xkr[2] * nk3 as f64 - 0.5 * p.k3 as f64;
                let in_list = (xx - nint(xx) as f64).abs() <= eps
                    && (yy - nint(yy) as f64).abs() <= eps
                    && (zz - nint(zz) as f64).abs() <= eps;
                if !in_list {
                    continue;
                }
                let ii = (nint(sign * xkr[0] * nk1 as f64 - 0.5 * p.k1 as f64 + 2.0 * nk1 as f64)
                    % nk1
                    + nk1)
                    % nk1
                    + 1;
                let jj = (nint(sign * xkr[1] * nk2 as f64 - 0.5 * p.k2 as f64 + 2.0 * nk2 as f64)
                    % nk2
                    + nk2)
                    % nk2
                    + 1;
                let kk = (nint(sign * xkr[2] * nk3 as f64 - 0.5 * p.k3 as f64 + 2.0 * nk3 as f64)
                    % nk3
                    + nk3)
                    % nk3
                    + 1;
                let n = ((kk - 1) + (jj - 1) * nk3 + (ii - 1) * nk2 * nk3 + 1) as usize;
                // 仅当 n 指向另一个“未归并”的点时才归并（与 Fortran 判定一致）。
                if n > nk as usize && equiv[n] == n as i32 {
                    equiv[n] = nk;
                    wkk[nk as usize] += 1.0;
                }
            }
        }
    }

    // 收集不可约点并按 nk 顺序输出（Γ 点天然在 nk=1，已在最前）。
    let mut raw: Vec<(usize, [f64; 3], [f64; 3], f64)> = Vec::new();
    let mut fact = 0.0_f64;
    for nk in 1..=nkr as i32 {
        if equiv[nk as usize] != nk {
            continue;
        }
        let crystal = [
            xkg[nk as usize][0] - nint(xkg[nk as usize][0]) as f64,
            xkg[nk as usize][1] - nint(xkg[nk as usize][1]) as f64,
            xkg[nk as usize][2] - nint(xkg[nk as usize][2]) as f64,
        ];
        let cart = cryst_to_cart(&info.bg, &crystal);
        let w = wkk[nk as usize];
        fact += w;
        raw.push((nk as usize, crystal, cart, w));
    }

    // 保证 Γ 点（若存在于列表中）位于首位，与 `q_points.f90` 的 exist_gamma 处理一致。
    if let Some(pos) = raw.iter().position(|(_, c, _, _)| {
        c[0].abs() < 1.0e-10 && c[1].abs() < 1.0e-10 && c[2].abs() < 1.0e-10
    }) {
        if pos != 0 {
            raw.swap(0, pos);
        }
    }

    raw.into_iter()
        .enumerate()
        .map(|(idx, (_, crystal, cart, w))| QPoint {
            index: idx + 1,
            q_crystal: crystal,
            q_cart: cart,
            weight: if fact > 0.0 { w / fact } else { 0.0 },
            is_gamma: idx == 0
                && crystal[0].abs() < 1.0e-10
                && crystal[1].abs() < 1.0e-10
                && crystal[2].abs() < 1.0e-10,
        })
        .collect()
}
