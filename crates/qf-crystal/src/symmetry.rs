//! 晶体对称性与物性辅助：spglib 对称化、原子质量、体积与密度。
//! 对齐 Python `pymatgen` 的 `SpacegroupAnalyzer` / `CifWriter` / `IStructure.density`。

use anyhow::{bail, Result};
use spglib::cell::Cell;
use spglib::dataset::Dataset;

use crate::poscar::Structure;

/// 标准原子量（IUPAC 2021，与 pymatgen 常用值一致），索引 = 原子序数。
pub const ATOMIC_MASSES: [f64; 119] = [0.0, 1.008, 4.002602, 6.94, 9.0121831, 10.811, 12.011, 14.007, 15.999, 18.998403163, 20.1797, 22.98976928, 24.305, 26.9815385, 28.085, 30.973761998, 32.06, 35.45, 39.948, 39.0983, 40.078, 44.955908, 47.867, 50.9415, 51.9961, 54.938044, 55.845, 58.933194, 58.6934, 63.546, 65.38, 69.723, 72.63, 74.921595, 78.971, 79.904, 83.798, 85.4678, 87.62, 88.90584, 91.224, 92.90637, 95.95, 98.0, 101.07, 102.9055, 106.42, 107.8682, 112.414, 114.818, 118.71, 121.76, 127.6, 126.90447, 131.293, 132.90545196, 137.327, 138.90547, 140.116, 140.90766, 144.242, 145.0, 150.36, 151.964, 157.25, 158.92535, 162.5, 164.93033, 167.259, 168.93422, 173.054, 174.9668, 178.49, 180.94788, 183.84, 186.207, 190.23, 192.217, 195.084, 196.966569, 200.592, 204.38, 207.2, 208.9804, 209.0, 210.0, 222.0, 223.0, 226.0, 227.0, 232.0377, 231.03588, 238.02891, 237.0, 244.0, 243.0, 247.0, 247.0, 251.0, 252.0, 257.0, 258.0, 259.0, 262.0, 267.0, 268.0, 269.0, 270.0, 269.0, 278.0, 281.0, 282.0, 285.0, 286.0, 289.0, 290.0, 293.0, 294.0, 294.0];

pub fn atomic_mass(symbol: &str) -> Option<f64> {
    let index = crate::elements::symbol_to_atomic_number(symbol)?;
    ATOMIC_MASSES.get(index).copied().filter(|m| *m > 0.0)
}

/// 晶胞体积（Å³）。
pub fn volume(structure: &Structure) -> f64 {
    let l = &structure.lattice;
    l[0][0] * (l[1][1] * l[2][2] - l[1][2] * l[2][1])
        - l[0][1] * (l[1][0] * l[2][2] - l[1][2] * l[2][0])
        + l[0][2] * (l[1][0] * l[2][1] - l[1][1] * l[2][0])
}

/// 密度（g/cm³），复刻 pymatgen `IStructure.density`：mass[amu]/volume[Å³] × 1.66053906660。
pub fn density(structure: &Structure) -> Result<f64> {
    let mut mass = 0.0_f64;
    for (sym, count) in structure.elements.iter().zip(&structure.counts) {
        let m = atomic_mass(sym).ok_or_else(|| anyhow::anyhow!("未知元素质量: {sym}"))?;
        mass += m * (*count as f64);
    }
    let vol = volume(structure);
    if vol.abs() < 1e-12 {
        bail!("晶胞体积为零，无法计算密度");
    }
    Ok(mass / vol * 1.6605390666050)
}

/// spglib 对称性分析结果。
#[derive(Debug, Clone)]
pub struct SymmetryInfo {
    pub number: i32,
    /// Hermann-Mauguin 简写（如 "P6_3/mmc"）。
    pub symbol: String,
    /// 理想化标准胞晶格（Å）。
    pub std_lattice: [[f64; 3]; 3],
    /// 理想化标准胞原子类型（元素符号）与分数坐标。
    pub std_atoms: Vec<(String, [f64; 3])>,
    /// 对称操作（旋转 + 平移），分数坐标。
    pub rotations: Vec<[[i32; 3]; 3]>,
    pub translations: Vec<[f64; 3]>,
}

fn spglib_cell(structure: &Structure) -> (Cell, [[f64; 3]; 3]) {
    // 类型编号从 1 开始，按元素首次出现顺序编号；elements 是"物种 + counts"，
    // 需展开为逐原子类型。
    let mut type_ids: Vec<(String, i32)> = Vec::new();
    let mut types: Vec<i32> = Vec::with_capacity(structure.positions.len());
    for (element, count) in structure.elements.iter().zip(&structure.counts) {
        let next = type_ids.len() as i32 + 1;
        let id = type_ids
            .iter()
            .find(|(e, _)| *e == *element)
            .map(|(_, id)| *id)
            .unwrap_or(next);
        if id == next {
            type_ids.push((element.clone(), id));
        }
        for _ in 0..*count {
            types.push(id);
        }
    }
    // POSCAR 坐标可能是笛卡尔，spglib 需要分数坐标。
    let is_direct = structure.coord_type.to_lowercase().starts_with('d');
    let positions: Vec<[f64; 3]> = if is_direct {
        structure.positions.clone()
    } else {
        crate::poscar::cartesian_to_fractional(&structure.lattice, &structure.positions)
    };
    // `spglib` crate 0.1 按列主序传晶格（spglib Fortran 风格约定），这里转置。
    let mut transposed = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            transposed[i][j] = structure.lattice[j][i];
        }
    }
    (Cell::new(transposed, &positions, &types), structure.lattice)
}

/// spglib 对称性分析（复刻 `SpacegroupAnalyzer(struct, symprec=...)`）。
pub fn analyze_symmetry(structure: &Structure, symprec: f64) -> Result<SymmetryInfo> {
    let (mut cell, lattice) = spglib_cell(structure);
    let dataset = Dataset::new(&mut cell, symprec);
    if dataset.spacegroup_number <= 0 || dataset.n_atoms == 0 {
        bail!("spglib 未能识别空间群（symprec={symprec}）");
    }
    // 理想化标准胞的类型编号 -> 元素符号映射。
    let mut type_elements: Vec<String> = Vec::new();
    for element in &structure.elements {
        if !type_elements.contains(element) {
            type_elements.push(element.clone());
        }
    }
    let std_atoms: Vec<(String, [f64; 3])> = dataset
        .std_types
        .iter()
        .zip(dataset.std_positions.iter())
        .map(|(t, p)| {
            let symbol = type_elements
                .get((*t - 1) as usize)
                .cloned()
                .unwrap_or_default();
            (symbol, *p)
        })
        .collect();
    // std_lattice 与输入同为列主序，转置回"行 = 晶格矢量"。
    let mut std_lattice = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            std_lattice[i][j] = dataset.std_lattice[j][i];
        }
    }
    let _ = lattice;
    Ok(SymmetryInfo {
        number: dataset.spacegroup_number,
        symbol: dataset.international_symbol,
        std_lattice,
        std_atoms,
        rotations: dataset.rotations.to_vec(),
        translations: dataset.translations.to_vec(),
    })
}

fn cell_params(lattice: &[[f64; 3]; 3]) -> (f64, f64, f64, f64, f64, f64) {
    let norm = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let angle = |a: [f64; 3], b: [f64; 3]| {
        let m = norm(a) * norm(b);
        if m > 0.0 {
            (dot(a, b) / m).acos().to_degrees()
        } else {
            0.0
        }
    };
    (
        norm(lattice[0]),
        norm(lattice[1]),
        norm(lattice[2]),
        angle(lattice[1], lattice[2]),
        angle(lattice[0], lattice[2]),
        angle(lattice[0], lattice[1]),
    )
}

/// 写出含空间群信息的对称化 CIF（等价于 Python `CifWriter(symmetrized, symprec=tol)` 的信息量：
/// 晶胞参数、空间群名/编号、对称操作、不等价原子位点）。
pub fn write_cif_symmetrized(info: &SymmetryInfo, formula: &str) -> String {
    let (a, b, c, alpha, beta, gamma) = cell_params(&info.std_lattice);
    let mut s = String::new();
    s.push_str(&format!(
        "# generated by scflow (Rust), spglib space group {} ({})\n",
        info.number, info.symbol
    ));
    s.push_str(&format!("data_{}_{}\n", formula, info.number));
    s.push_str(&format!("_symmetry_space_group_name_H-M '{sym}'\n", sym = info.symbol));
    s.push_str(&format!("_symmetry_Int_Tables_number {}\n", info.number));
    s.push_str(&format!("_cell_length_a {a:.6}\n"));
    s.push_str(&format!("_cell_length_b {b:.6}\n"));
    s.push_str(&format!("_cell_length_c {c:.6}\n"));
    s.push_str(&format!("_cell_angle_alpha {alpha:.6}\n"));
    s.push_str(&format!("_cell_angle_beta {beta:.6}\n"));
    s.push_str(&format!("_cell_angle_gamma {gamma:.6}\n"));
    s.push_str("loop_\n_symmetry_equiv_pos_as_xyz\n");
    for (rot, trans) in info.rotations.iter().zip(&info.translations) {
        let parts: Vec<String> = (0..3)
            .map(|r| {
                let mut term = String::new();
                for (cn, cv) in [('x', rot[r][0]), ('y', rot[r][1]), ('z', rot[r][2])] {
                    if cv == 0 {
                        continue;
                    }
                    if !term.is_empty() && cv > 0 {
                        term.push('+');
                    }
                    if cv.abs() == 1 {
                        if cv < 0 {
                            term.push('-');
                        }
                        term.push(cv.abs() as u8 as char);
                        let _ = cv.abs();
                        term.pop();
                        term.push(cn);
                    } else {
                        term.push_str(&format!("{cv}{cn}"));
                    }
                }
                if term.is_empty() {
                    term.push('0');
                }
                let t = trans[r];
                if t.abs() > 1e-9 {
                    format!("{term}+{t:.8}")
                } else {
                    term
                }
            })
            .collect();
        s.push_str(&format!("'{}'\n", parts.join(", ")));
    }
    s.push_str("loop_\n_atom_site_label\n_atom_site_type_symbol\n_atom_site_fract_x\n_atom_site_fract_y\n_atom_site_fract_z\n");
    let mut counters: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (symbol, p) in &info.std_atoms {
        let counter = counters.entry(symbol.clone()).or_insert(0);
        *counter += 1;
        s.push_str(&format!(
            "{symbol}{counter} {symbol} {:.8} {:.8} {:.8}\n",
            p[0], p[1], p[2]
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poscar::Structure;

    fn hcp_mg() -> Structure {
        // 六方 Mg（P6_3/mmc，194）近似结构。
        Structure {
            formula: "Mg".to_string(),
            elements: vec!["Mg".to_string(), "Mg".to_string()],
            counts: vec![2],
            lattice: [
                [3.2, 0.0, 0.0],
                [-1.6, 2.77128, 0.0],
                [0.0, 0.0, 5.2],
            ],
            positions: vec![
                [1.0 / 3.0, 2.0 / 3.0, 0.25],
                [2.0 / 3.0, 1.0 / 3.0, 0.75],
            ],
            coord_type: "direct".to_string(),
        }
    }

    #[test]
    fn detects_hcp_spacegroup() {
        let info = analyze_symmetry(&hcp_mg(), 0.2).unwrap();
        assert_eq!(info.number, 194);
        assert!(info.std_atoms.len() >= 2);
    }

    #[test]
    fn density_matches_pymatgen() {
        let mut s = hcp_mg();
        s.elements = vec!["Mg".into(), "B".into()];
        s.counts = vec![1, 1];
        let d = density(&s).unwrap();
        // (24.305+10.811)/(3.2*2.77128*5.2)*1.6605390666050 ≈ 1.26447
        assert!((d - 1.264_469).abs() < 1e-4, "density={d}");
    }
}
