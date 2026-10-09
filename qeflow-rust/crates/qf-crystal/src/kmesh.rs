//! k 点网格派生（复刻 `qeflow/crystal/kmesh.py`）。

/// k 网格规格：显式网格或间距（Å⁻¹ 量级的 reciprocal 间距）。
#[derive(Debug, Clone)]
pub enum KmeshSpec {
    Grid([i64; 3]),
    Spacing(f64),
}

/// 由间距（或显式网格）推导自动 k 点网格。
/// `lattice` 为 Å 单位晶格；`default_spacing` 为 reciprocal 间距（Å⁻¹）的下限（Python 用 0.04）。
pub fn get_kpoints_from_spacing(
    lattice: [[f64; 3]; 3],
    spec: &KmeshSpec,
    default_spacing: f64,
) -> [i64; 3] {
    match spec {
        KmeshSpec::Grid(g) => *g,
        KmeshSpec::Spacing(spacing) => {
            let reciprocal = reciprocal_lattice(lattice);
            let lengths = [
                vec_norm(reciprocal[0]),
                vec_norm(reciprocal[1]),
                vec_norm(reciprocal[2]),
            ];
            let sp = if *spacing > 0.0 { *spacing } else { default_spacing };
            let k = |l: f64| -> i64 { ((l / (sp * 2.0 * std::f64::consts::PI)).ceil()).max(1.0) as i64 };
            [k(lengths[0]), k(lengths[1]), k(lengths[2])]
        }
    }
}

/// 倒格子基矢（行向量），单位 Å⁻¹。`lattice` 为 Å 单位晶格。
pub fn reciprocal_lattice(lattice: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let a = lattice;
    let vol = det3(a);
    let mut b = [[0.0f64; 3]; 3];
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let cross = cross3(a[j], a[k]);
        for m in 0..3 {
            b[i][m] = 2.0 * std::f64::consts::PI * cross[m] / vol;
        }
    }
    b
}

fn vec_norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}
