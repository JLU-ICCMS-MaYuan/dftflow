//! 对称矩阵特征值求解（Jacobi 旋转法），用于 Eliashberg 矩阵最大本征值。
//! 复刻 `scflow/sc_e.py: get_tc_e` 中对 K 矩阵 `np.linalg.eig` 的依赖，
//! 但用纯 Rust 实现以避免引入 OpenBLAS 重依赖（垂直切片阶段）。

use ndarray::Array2;

/// 返回对称矩阵的所有特征值（不保证顺序）。适用于中小规模（≤ ~50×50）。
pub fn symmetric_eigenvalues(a: &Array2<f64>) -> Vec<f64> {
    let n = a.nrows();
    if n == 0 {
        return Vec::new();
    }
    let mut m = a.to_owned();
    let max_sweeps = 100;
    let tol = 1e-12;
    for _ in 0..max_sweeps {
        // 找最大非对角元
        let mut p = 0usize;
        let mut q = 1usize;
        let mut max = 0.0_f64;
        for i in 0..n {
            for j in (i + 1)..n {
                let v = m[[i, j]].abs();
                if v > max {
                    max = v;
                    p = i;
                    q = j;
                }
            }
        }
        if max < tol {
            break;
        }
        let app = m[[p, p]];
        let aqq = m[[q, q]];
        let apq = m[[p, q]];
        // tan(2θ) = 2 apq / (aqq - app)
        let phi = 0.5 * (aqq - app) / (2.0 * apq);
        let t = if phi >= 0.0 {
            1.0 / (phi + (1.0 + phi * phi).sqrt())
        } else {
            1.0 / (phi - (1.0 + phi * phi).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        // 更新第 p、q 列
        for k in 0..n {
            let mpk = m[[p, k]];
            let mqk = m[[q, k]];
            m[[p, k]] = c * mpk - s * mqk;
            m[[q, k]] = s * mpk + c * mqk;
        }
        // 更新第 p、q 行（矩阵对称）
        for k in 0..n {
            let mkp = m[[k, p]];
            let mkq = m[[k, q]];
            m[[k, p]] = c * mkp - s * mkq;
            m[[k, q]] = s * mkp + c * mkq;
        }
    }
    (0..n).map(|i| m[[i, i]]).collect()
}

/// 返回对称矩阵的最大特征值。
pub fn max_eigenvalue(a: &Array2<f64>) -> f64 {
    symmetric_eigenvalues(a)
        .into_iter()
        .max_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0)
}
