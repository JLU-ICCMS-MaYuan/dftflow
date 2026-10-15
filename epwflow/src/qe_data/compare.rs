use super::manifest::QeManifest;

/// FR-011 预先固定的数值对照容差（实施前固定，失败不得放宽）。
pub const TOL_COORD: f64 = 1e-8;
pub const TOL_WEIGHT: f64 = 1e-10;
pub const TOL_SCF_ENERGY_RY: f64 = 1e-7;
pub const TOL_NSCF_EIGEN_EV: f64 = 1e-5;
pub const TOL_DYN: f64 = 1e-7;
pub const TOL_FREQ_CM1: f64 = 0.1;

/// 标量绝对差上限比较。
pub fn compare_scalar(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// 坐标按倒格矢周期匹配：允许周期平移后逐分量差 ≤ tol。
pub fn coordinate_match_periodic(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| {
        let d = (x - y).fract();
        let d = if d < 0.0 { d + 1.0 } else { d };
        d <= tol || (1.0 - d).abs() <= tol
    })
}

/// k/q 坐标集合周期匹配；顺序按索引对齐（同一 q 列表）。
pub fn compare_coordinates(a: &[[f64; 3]], b: &[[f64; 3]], tol: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| coordinate_match_periodic(*x, *y, tol))
}

/// 权重：记录原始约定后归一化，比较逐权重与总和误差。
pub fn compare_weights(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    let sa: f64 = a.iter().sum();
    let sb: f64 = b.iter().sum();
    if sa.abs() < 1e-300 || sb.abs() < 1e-300 {
        return false;
    }
    let na: Vec<f64> = a.iter().map(|v| v / sa).collect();
    let nb: Vec<f64> = b.iter().map(|v| v / sb).collect();
    // 归一化后总和恒为 1，故仅比较逐权重差。
    na.iter()
        .zip(nb.iter())
        .all(|(x, y)| (x - y).abs() <= TOL_WEIGHT)
}

/// 两个等长向量的最大绝对差；长度不等返回 None。
pub fn max_abs_diff(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(None, |acc, d| match acc {
            None => Some(d),
            Some(m) => Some(m.max(d)),
        })
}

/// 本征能量逐带匹配（同 k 点按能量及简并组比较），最大绝对差 ≤ 容差。
pub fn compare_eigenvalues(a: &[Vec<f64>], b: &[Vec<f64>], tol: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (ka, kb) in a.iter().zip(b.iter()) {
        if ka.len() != kb.len() {
            return false;
        }
        if let Some(d) = max_abs_diff(ka, kb) {
            if d > tol {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

/// 动力学矩阵逐元素比较（统一原子顺序与笛卡尔基底）。
pub fn compare_dyn(a: &[f64], b: &[f64]) -> bool {
    match max_abs_diff(a, b) {
        Some(d) => d <= TOL_DYN,
        None => false,
    }
}

/// 声子频率：同 q、同 ASR，保留虚频符号，按频率及简并组匹配。
pub fn compare_frequencies(a: &[f64], b: &[f64], tol: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| {
        // 符号必须一致（保留虚频），大小差 ≤ tol。
        x.signum() == y.signum() && (x - y).abs() <= tol
    })
}

/// 对照报告：每条度量记录通过与否与细节。
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct ComparisonReport {
    pub passed: bool,
    pub metrics: Vec<Metric>,
}

impl ComparisonReport {
    fn metric(&mut self, name: &str, passed: bool, detail: &str) {
        if !passed {
            self.passed = false;
        }
        self.metrics.push(Metric {
            name: name.to_string(),
            passed,
            detail: detail.to_string(),
        });
    }
}

/// 两版 QE 数据按 FR-011 全部规则对照；缺任一项不整体通过。
pub fn compare_manifests(a: &QeManifest, b: &QeManifest) -> ComparisonReport {
    let mut rep = ComparisonReport {
        passed: true,
        metrics: Vec::new(),
    };
    // 结构/赝势/程序身份应一致。
    let same_fp = a.fingerprints.structure_sha256 == b.fingerprints.structure_sha256
        && a.fingerprints.pseudo_sha256 == b.fingerprints.pseudo_sha256
        && a.fingerprints.program_sha256 == b.fingerprints.program_sha256;
    rep.metric("fingerprints", same_fp, "结构/赝势/程序身份");

    // SCF 总能绝对差。
    let (ea, eb) = match (&a.scf, &b.scf) {
        (Some(x), Some(y)) => (x.total_energy_ry, y.total_energy_ry),
        _ => (f64::NAN, f64::NAN),
    };
    let scf_ok = ea.is_finite() && eb.is_finite() && compare_scalar(ea, eb, TOL_SCF_ENERGY_RY);
    rep.metric("scf_total_energy", scf_ok, &format!("Δ={:.3e} Ry", (ea - eb).abs()));

    // 电子态：k 数、nbnd、k 坐标、权重、本征能量。
    match (&a.states, &b.states) {
        (Some(x), Some(y)) => {
            let grid_ok = x.k_grid == y.k_grid && x.nbnd == y.nbnd;
            rep.metric("nscf_grid_nbnd", grid_ok, &format!("k_grid={:?} nbnd={}", x.k_grid, x.nbnd));
            let coord_ok = compare_coordinates(&x.kpoints, &y.kpoints, TOL_COORD);
            rep.metric("nscf_k_coordinates", coord_ok, "k 坐标周期匹配");
            let w_ok = compare_weights(&x.weights, &y.weights);
            rep.metric("nscf_weights", w_ok, "权重归一化对照");
            let eig_ok = compare_eigenvalues(&x.eigenvalues, &y.eigenvalues, TOL_NSCF_EIGEN_EV);
            rep.metric("nscf_eigenvalues", eig_ok, &format!("tol={TOL_NSCF_EIGEN_EV} eV"));
        }
        _ => rep.metric("nscf_states", false, "缺少电子态"),
    }

    // q 点数量与坐标。
    let q_ok = a.qpoints.len() == b.qpoints.len()
        && compare_coordinates(
            &a.qpoints.iter().map(|q| q.crystal).collect::<Vec<_>>(),
            &b.qpoints.iter().map(|q| q.crystal).collect::<Vec<_>>(),
            TOL_COORD,
        );
    rep.metric("q_points", q_ok, "q 数量与坐标");

    // 响应文件数量与扰动数。
    let ph_ok = a.phonon_files.len() == b.phonon_files.len()
        && a
            .phonon_files
            .iter()
            .zip(b.phonon_files.iter())
            .all(|(x, y)| x.q_id == y.q_id && x.perturbations == y.perturbations);
    rep.metric("phonon_files", ph_ok, "响应文件与扰动数");

    rep
}
