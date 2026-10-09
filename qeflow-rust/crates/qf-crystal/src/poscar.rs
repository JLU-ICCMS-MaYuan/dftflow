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
    for i in 0..total {
        let line = lines
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
