//! VASP `DOSCAR` 解析（对齐 `vaspflow` 后处理读取总 DOS 与分波 DOS）。
//!
//! 格式：前两行注释，第三行为头 `NIONS NIMAGES NISPIN NEDOS E(fermi) ...`，
//! 之后 `NEDOS` 行总 DOS（非自旋 3 列；自旋极化 5 列），
//! 若开启 `LORBIT` 再按离子给出分波 DOS 块。

use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct IonDos {
    pub ion_index: usize,
    /// 每个离子分波 DOS 原始数据行，按自旋分块（非自旋为 1 块）。
    pub blocks: Vec<Vec<Vec<f64>>>,
}

#[derive(Debug, Clone)]
pub struct DosData {
    pub nions: usize,
    pub nspin: usize,
    pub nedos: usize,
    pub efermi: f64,
    pub energies: Vec<f64>,
    /// 总 DOS，按自旋：`tdos[spin][nedos]`。
    pub tdos: Vec<Vec<f64>>,
    /// 积分 DOS，按自旋：`itdos[spin][nedos]`。
    pub itdos: Vec<Vec<f64>>,
    pub partial: Vec<IonDos>,
}

fn float_block(text: &str, start: usize, nedos: usize) -> (usize, Vec<Vec<f64>>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut rows = Vec::with_capacity(nedos);
    let mut i = start;
    while rows.len() < nedos && i < lines.len() {
        let p: Vec<f64> = lines[i].split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if !p.is_empty() {
            rows.push(p);
        }
        i += 1;
    }
    (i, rows)
}

/// 解析 `DOSCAR` 文本（纯函数）。
pub fn parse_doscar(text: &str) -> Result<DosData> {
    let lines: Vec<&str> = text.lines().collect();
    let mut header_idx = None;
    for (i, line) in lines.iter().enumerate() {
        let nums: Vec<f64> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        // 头行至少含 NIONS/NIMAGES/NISPIN/NEDOS/E(fermi)。
        if nums.len() >= 5 {
            header_idx = Some(i);
            break;
        }
    }
    let hi = header_idx.ok_or_else(|| anyhow::anyhow!("DOSCAR 找不到整数头行"))?;
    let nums: Vec<f64> = lines[hi]
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    let nions = nums[0] as usize;
    let nspin = if nums[2] as usize == 0 { 1 } else { nums[2] as usize };
    let nedos = nums[3] as usize;
    let efermi = nums[4];
    if nedos == 0 {
        bail!("DOSCAR NEDOS 为 0");
    }

    // 总 DOS 块。
    let (mut idx, total) = float_block(text, hi + 1, nedos);
    let mut energies = Vec::with_capacity(nedos);
    let mut tdos = vec![Vec::with_capacity(nedos); nspin];
    let mut itdos = vec![Vec::with_capacity(nedos); nspin];
    for row in &total {
        energies.push(row[0]);
        if nspin == 1 {
            if row.len() >= 3 {
                tdos[0].push(row[1]);
                itdos[0].push(row[2]);
            }
        } else {
            if row.len() >= 5 {
                tdos[0].push(row[1]);
                tdos[1].push(row[2]);
                itdos[0].push(row[3]);
                itdos[1].push(row[4]);
            }
        }
    }

    // 分波 DOS（若剩余数据足够）。
    let mut partial = Vec::new();
    while idx < lines.len() {
        let head: Vec<f64> = lines[idx]
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        let is_ion_header = lines[idx]
            .to_uppercase()
            .contains("ION")
            || (head.len() >= 3 && head[0] >= 1.0 && head[1] >= 0.0);
        if !is_ion_header {
            idx += 1;
            continue;
        }
        let ion = head[0] as usize;
        idx += 1;
        let mut blocks = Vec::new();
        // 自旋极化：每个离子有 nspin 个数据块。
        for _ in 0..nspin {
            if idx >= lines.len() {
                break;
            }
            // 自旋块前可能还有一行表头（"ion i" 或空），跳过纯文本行。
            let next: Vec<f64> = lines[idx]
                .split_whitespace()
                .filter_map(|s| s.parse::<f64>().ok())
                .collect();
            if next.is_empty() || lines[idx].to_uppercase().contains("ION") {
                idx += 1;
            }
            let (nidx, blk) = float_block(text, idx, nedos);
            idx = nidx;
            if blk.len() == nedos {
                blocks.push(blk);
            }
        }
        if !blocks.is_empty() {
            partial.push(IonDos {
                ion_index: ion,
                blocks,
            });
        }
    }

    Ok(DosData {
        nions,
        nspin,
        nedos,
        efermi,
        energies,
        tdos,
        itdos,
        partial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_total_dos_nonspin() {
        let mut txt = String::new();
        txt.push_str("system\ncomment\n   2   0   1  3  2.5\n");
        txt.push_str("  -1.0   0.1   0.01\n");
        txt.push_str("   0.0   0.5   0.05\n");
        txt.push_str("   1.0   0.2   0.02\n");
        let d = parse_doscar(&txt).unwrap();
        assert_eq!(d.nspin, 1);
        assert_eq!(d.nedos, 3);
        assert_eq!(d.efermi, 2.5);
        assert_eq!(d.energies, vec![-1.0, 0.0, 1.0]);
        assert_eq!(d.tdos[0], vec![0.1, 0.5, 0.2]);
        assert_eq!(d.itdos[0].len(), 3);
    }

    #[test]
    fn parses_total_dos_spin() {
        let mut txt = String::new();
        txt.push_str("system\ncomment\n   1   1   2  2  1.0\n");
        txt.push_str("  -1.0   0.3   0.1   0.03   0.01\n");
        txt.push_str("   1.0   0.4   0.2   0.04   0.02\n");
        let d = parse_doscar(&txt).unwrap();
        assert_eq!(d.nspin, 2);
        assert_eq!(d.tdos[0], vec![0.3, 0.4]);
        assert_eq!(d.tdos[1], vec![0.1, 0.2]);
    }
}
