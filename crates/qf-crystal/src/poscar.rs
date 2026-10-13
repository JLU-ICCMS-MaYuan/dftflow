//! POSCAR 解析（复刻 `qeflow/crystal/poscar.py`）。

use anyhow::{bail, Result};
use std::fs;

use crate::formula::get_formula;

/// 解析后的晶体结构。
#[derive(Debug, Clone)]
pub struct Structure {
    pub formula: String,
    pub elements: Vec<String>,
    pub counts: Vec<usize>,
    /// 晶格矢量（Å），`[[a1],[a2],[a3]]`。
    pub lattice: [[f64; 3]; 3],
    /// 原子坐标，长度 = sum(counts)。
    pub positions: Vec<[f64; 3]>,
    /// 坐标类型：以 'd' 开头表示 crystal（分数），否则 angstrom（笛卡尔）。
    pub coord_type: String,
}

/// 解析 VASP POSCAR 为 [`Structure`]。
pub fn parse_poscar(poscar_path: &str) -> Result<Structure> {
    let text = fs::read_to_string(poscar_path)
        .map_err(|_| anyhow::anyhow!("找不到 {poscar_path} 文件"))?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 8 {
        bail!("POSCAR 行数不足，无法解析。");
    }
    let scale: f64 = lines[1]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("无法解析 POSCAR 第二行的缩放系数"))?;

    let mut lattice = [[0.0f64; 3]; 3];
    for (i, row) in lattice.iter_mut().enumerate() {
        let vals: Vec<f64> = lines[2 + i]
            .split_whitespace()
            .map(|s| s.parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| anyhow::anyhow!("无法解析晶格矢量第 {} 行", i + 1))?;
        for (j, v) in vals.iter().enumerate().take(3) {
            row[j] = v * scale;
        }
    }

    // 元素行：第 5 行（索引 5）通常是元素符号；若其首列为数字，则元素在第 0 行（索引 0）。
    let mut elements: Vec<String> = lines[5].split_whitespace().map(|s| s.to_string()).collect();
    if elements.first().map(|s| s.chars().next().unwrap().is_ascii_digit()).unwrap_or(false) {
        elements = lines[0].split_whitespace().map(|s| s.to_string()).collect();
        if elements.is_empty() {
            bail!("无法在 POSCAR 中找到元素名称。");
        }
    }
    let counts: Vec<usize> = lines[6]
        .split_whitespace()
        .map(|s| s.parse::<usize>())
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("无法解析原子计数行"))?;

    let coord_line = lines[7].trim().to_string();
    let (start_line, coord_type) = if coord_line.to_lowercase().starts_with('s') {
        (9, lines[8].trim().to_string())
    } else {
        (8, coord_line)
    };

    let total: usize = counts.iter().sum();
    let mut positions = Vec::with_capacity(total);
    for i in 0..total {        let line = lines
            .get(start_line + i)
            .ok_or_else(|| anyhow::anyhow!("POSCAR 坐标行不足"))?;
        let vals: Vec<f64> = line
            .split_whitespace()
            .take(3)
            .map(|s| s.parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| anyhow::anyhow!("无法解析坐标行 {}", i + 1))?;
        let mut p = [vals[0], vals[1], vals[2]];
        if !coord_type.to_lowercase().starts_with('d') {
            for v in &mut p {
                *v *= scale;
            }
        }
        positions.push(p);
    }

    let formula = get_formula(&elements, &counts);
    Ok(Structure {
        formula,
        elements,
        counts,
        lattice,
        positions,
        coord_type,
    })
}

/// 笛卡尔坐标（Å）→ 分数坐标。
pub fn cartesian_to_fractional(lattice: &[[f64; 3]; 3], positions: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let det = lattice[0][0] * (lattice[1][1] * lattice[2][2] - lattice[1][2] * lattice[2][1])
        - lattice[0][1] * (lattice[1][0] * lattice[2][2] - lattice[1][2] * lattice[2][0])
        + lattice[0][2] * (lattice[1][0] * lattice[2][1] - lattice[1][1] * lattice[2][0]);
    let inv = if det.abs() > 1e-12 {
        [
            [
                (lattice[1][1] * lattice[2][2] - lattice[1][2] * lattice[2][1]) / det,
                (lattice[0][2] * lattice[2][1] - lattice[0][1] * lattice[2][2]) / det,
                (lattice[0][1] * lattice[1][2] - lattice[0][2] * lattice[1][1]) / det,
            ],
            [
                (lattice[1][2] * lattice[2][0] - lattice[1][0] * lattice[2][2]) / det,
                (lattice[0][0] * lattice[2][2] - lattice[0][2] * lattice[2][0]) / det,
                (lattice[0][2] * lattice[1][0] - lattice[0][0] * lattice[1][2]) / det,
            ],
            [
                (lattice[1][0] * lattice[2][1] - lattice[1][1] * lattice[2][0]) / det,
                (lattice[0][1] * lattice[2][0] - lattice[0][0] * lattice[2][1]) / det,
                (lattice[0][0] * lattice[1][1] - lattice[0][1] * lattice[1][0]) / det,
            ],
        ]
    } else {
        [[0.0; 3]; 3]
    };
    // 行向量约定：cart = fract · M → fract = cart · M⁻¹
    positions
        .iter()
        .map(|p| {
            [
                inv[0][0] * p[0] + inv[1][0] * p[1] + inv[2][0] * p[2],
                inv[0][1] * p[0] + inv[1][1] * p[1] + inv[2][1] * p[2],
                inv[0][2] * p[0] + inv[1][2] * p[1] + inv[2][2] * p[2],
            ]
        })
        .collect()
}

/// 写出一个最小 CIF（复刻 Python `save_structure` 的结构文本写出）。
/// 当前为原始结构（坐标系已统一为分数坐标）；对称化（spglib）为后续增强。
pub fn write_cif(structure: &Structure) -> String {
    let norm = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let angle = |a: [f64; 3], b: [f64; 3]| {
        let d = dot(a, b);
        let m = norm(a) * norm(b);
        if m > 0.0 {
            d / m
        } else {
            0.0
        }
        .acos()
        .to_degrees()
    };
    let a = norm(structure.lattice[0]);
    let b = norm(structure.lattice[1]);
    let c = norm(structure.lattice[2]);
    let alpha = angle(structure.lattice[1], structure.lattice[2]);
    let beta = angle(structure.lattice[0], structure.lattice[2]);
    let gamma = angle(structure.lattice[0], structure.lattice[1]);

    // 笛卡尔 -> 分数坐标用逆晶格
    let det = structure.lattice[0][0]
        * (structure.lattice[1][1] * structure.lattice[2][2]
            - structure.lattice[1][2] * structure.lattice[2][1])
        - structure.lattice[0][1]
            * (structure.lattice[1][0] * structure.lattice[2][2]
                - structure.lattice[1][2] * structure.lattice[2][0])
        + structure.lattice[0][2]
            * (structure.lattice[1][0] * structure.lattice[2][1]
                - structure.lattice[1][1] * structure.lattice[2][0]);
    let inv = if det.abs() > 1e-12 {
        let mut inv = [[0.0f64; 3]; 3];
        inv[0][0] = (structure.lattice[1][1] * structure.lattice[2][2]
            - structure.lattice[1][2] * structure.lattice[2][1])
            / det;
        inv[0][1] = (structure.lattice[0][2] * structure.lattice[2][1]
            - structure.lattice[0][1] * structure.lattice[2][2])
            / det;
        inv[0][2] = (structure.lattice[0][1] * structure.lattice[1][2]
            - structure.lattice[0][2] * structure.lattice[1][1])
            / det;
        inv[1][0] = (structure.lattice[1][2] * structure.lattice[2][0]
            - structure.lattice[1][0] * structure.lattice[2][2])
            / det;
        inv[1][1] = (structure.lattice[0][0] * structure.lattice[2][2]
            - structure.lattice[0][2] * structure.lattice[2][0])
            / det;
        inv[1][2] = (structure.lattice[0][2] * structure.lattice[1][0]
            - structure.lattice[0][0] * structure.lattice[1][2])
            / det;
        inv[2][0] = (structure.lattice[1][0] * structure.lattice[2][1]
            - structure.lattice[1][1] * structure.lattice[2][0])
            / det;
        inv[2][1] = (structure.lattice[0][1] * structure.lattice[2][0]
            - structure.lattice[0][0] * structure.lattice[2][1])
            / det;
        inv[2][2] = (structure.lattice[0][0] * structure.lattice[1][1]
            - structure.lattice[0][1] * structure.lattice[1][0])
            / det;
        inv
    } else {
        [[0.0; 3]; 3]
    };
    let to_fract = |p: [f64; 3]| -> [f64; 3] {
        [
            inv[0][0] * p[0] + inv[0][1] * p[1] + inv[0][2] * p[2],
            inv[1][0] * p[0] + inv[1][1] * p[1] + inv[1][2] * p[2],
            inv[2][0] * p[0] + inv[2][1] * p[1] + inv[2][2] * p[2],
        ]
    };

    let mut s = String::new();
    s.push_str(&format!("data_{}\n", structure.formula));
    s.push_str(&format!("_cell_length_a {a:.6}\n"));
    s.push_str(&format!("_cell_length_b {b:.6}\n"));
    s.push_str(&format!("_cell_length_c {c:.6}\n"));
    s.push_str(&format!("_cell_angle_alpha {alpha:.4}\n"));
    s.push_str(&format!("_cell_angle_beta {beta:.4}\n"));
    s.push_str(&format!("_cell_angle_gamma {gamma:.4}\n"));
    s.push_str(
        "loop_\n_atom_site_label\n_atom_site_type_symbol\n_atom_site_fract_x\n_atom_site_fract_y\n_atom_site_fract_z\n",
    );

    let is_direct = structure.coord_type.to_lowercase().starts_with('d');
    let mut atom_index = 0usize;
    for (sym, count) in structure.elements.iter().zip(structure.counts.iter()) {
        for k in 0..*count {
            let p = structure.positions[atom_index];
            let fract = if is_direct {
                p
            } else {
                to_fract(p)
            };
            s.push_str(&format!(
                "{sym}{k} {sym} {:.6} {:.6} {:.6}\n",
                fract[0], fract[1], fract[2]
            ));
            atom_index += 1;
        }
    }
    s
}
