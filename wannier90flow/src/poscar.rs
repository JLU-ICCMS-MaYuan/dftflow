//! POSCAR 解析（复刻 `wannier_init.parse_poscar`：保留 Direct/Cartesian 体系）。

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Poscar {
    pub comment: String,
    /// 实空间晶格（3×3，已乘 scale，Å）。
    pub lattice: [[f64; 3]; 3],
    pub elements: Vec<String>,
    pub counts: Vec<usize>,
    /// "direct" 或 "cartesian"。
    pub coord_type: String,
    /// 原子分数/笛卡尔坐标（cartesian 已乘 scale）。
    pub positions: Vec<[f64; 3]>,
}

/// 解析 POSCAR（逐行对齐 Python 版本：剔除空行、Selective dynamics、scale 语义）。
pub fn parse_poscar(path: &str) -> Result<Poscar> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("找不到 POSCAR: {path}"))?;
    let lines: Vec<&str> = text.lines().filter(|ln| !ln.trim().is_empty()).collect();
    if lines.len() < 8 {
        anyhow::bail!("POSCAR 行数不足，无法解析。");
    }

    let comment = lines[0].trim_end().to_string();
    let scale: f64 = lines[1]
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("POSCAR scale 解析失败: {e}"))?;

    let mut lattice = [[0.0f64; 3]; 3];
    for (i, row) in lattice.iter_mut().enumerate() {
        let parts: Vec<&str> = lines[2 + i].split_whitespace().collect();
        for (j, tok) in parts.iter().take(3).enumerate() {
            row[j] = tok.parse::<f64>().map_err(|e| anyhow::anyhow!("晶格解析失败: {e}"))? * scale;
        }
    }

    let elements: Vec<String> = lines[5].split_whitespace().map(|s| s.to_string()).collect();
    let counts: Vec<usize> = lines[6]
        .split_whitespace()
        .map(|s| s.parse::<usize>())
        .collect::<Result<_, _>>()
        .map_err(|e| anyhow::anyhow!("原子计数解析失败: {e}"))?;

    let mut coord_line = lines[7].trim().to_lowercase();
    let mut start_line = 8;
    if coord_line.starts_with('s') {
        // Selective dynamics
        coord_line = lines[8].trim().to_lowercase();
        start_line = 9;
    }

    let total_atoms: usize = counts.iter().sum();
    let mut positions = Vec::with_capacity(total_atoms);
    for idx in 0..total_atoms {
        let line = lines
            .get(start_line + idx)
            .ok_or_else(|| anyhow::anyhow!("POSCAR 原子坐标行不足"))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut p = [0.0f64; 3];
        for (j, tok) in parts.iter().take(3).enumerate() {
            p[j] = tok.parse::<f64>().map_err(|e| anyhow::anyhow!("坐标解析失败: {e}"))?;
        }
        positions.push(p);
    }

    let coord_type = if coord_line.starts_with('d') {
        "direct".to_string()
    } else {
        "cartesian".to_string()
    };
    if coord_type == "cartesian" {
        for p in positions.iter_mut() {
            for x in p.iter_mut() {
                *x *= scale;
            }
        }
    }

    Ok(Poscar {
        comment,
        lattice,
        elements,
        counts,
        coord_type,
        positions,
    })
}
