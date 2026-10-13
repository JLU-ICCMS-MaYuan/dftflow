//! VASP `EIGENVAL` 解析（对齐 `vaspflow` 各后处理读取能带/本征值）。
//!
//! 格式：前两行为注释，第三行为整数头 `NIONS NISPIN NBANDS ...`，
//! 之后按 k 点循环：一行 `kx ky kz weight` + `NBANDS` 行 `E occ`。
//! `ISPIN=2` 时整个 k 点块重复两次（先 spin1 后 spin2）。

use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct EigenvalKpoint {
    pub coord: [f64; 3],
    pub weight: f64,
    /// 本征值，按自旋分块：`energies[spin][band]`。
    pub energies: Vec<Vec<f64>>,
}

#[derive(Debug, Clone)]
pub struct Eigenval {
    pub nions: usize,
    pub nspin: usize,
    pub nbands: usize,
    pub kpoints: Vec<EigenvalKpoint>,
}

struct RawBlock {
    coord: [f64; 3],
    weight: f64,
    energies: Vec<f64>,
}

/// 解析 `EIGENVAL` 文本（纯函数）。
pub fn parse_eigenval(text: &str) -> Result<Eigenval> {
    let lines: Vec<&str> = text.lines().collect();
    // 定位整数头行：首个含至少 3 个连续整数的行。
    let mut header_idx = None;
    for (i, line) in lines.iter().enumerate() {
        let ints: Vec<usize> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .collect();
        if ints.len() >= 3 {
            header_idx = Some(i);
            break;
        }
    }
    let hi = header_idx.ok_or_else(|| anyhow::anyhow!("EIGENVAL 找不到整数头行"))?;
    let header: Vec<usize> = lines[hi]
        .split_whitespace()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect();
    let nions = header[0];
    let nspin = if header[1] == 0 { 1 } else { header[1] };
    let nbands = header[2];
    if nbands == 0 {
        bail!("EIGENVAL NBANDS 为 0");
    }

    // 逐块读取。
    let mut blocks: Vec<RawBlock> = Vec::new();
    let mut i = hi + 1;
    while i < lines.len() {
        let kl: Vec<f64> = lines[i]
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if kl.len() < 4 {
            i += 1;
            continue;
        }
        i += 1;
        let mut energies = Vec::with_capacity(nbands);
        let mut got = 0;
        while got < nbands && i < lines.len() {
            let parts: Vec<f64> = lines[i]
                .split_whitespace()
                .filter_map(|s| s.parse::<f64>().ok())
                .collect();
            if parts.is_empty() {
                i += 1;
                continue;
            }
            energies.push(parts[0]);
            got += 1;
            i += 1;
        }
        if energies.len() != nbands {
            bail!(
                "EIGENVAL k 点块能量数量不符：期望 {nbands}，实际 {}",
                energies.len()
            );
        }
        blocks.push(RawBlock {
            coord: [kl[0], kl[1], kl[2]],
            weight: kl[3],
            energies,
        });
    }

    if nspin == 1 {
        let kpoints = blocks
            .into_iter()
            .map(|b| EigenvalKpoint {
                coord: b.coord,
                weight: b.weight,
                energies: vec![b.energies],
            })
            .collect();
        Ok(Eigenval {
            nions,
            nspin,
            nbands,
            kpoints,
        })
    } else {
        let nk = blocks.len() / 2;
        if blocks.len() != nk * 2 {
            bail!("EIGENVAL ISPIN=2 时 k 点块数量应为偶数");
        }
        let mut kpoints = Vec::with_capacity(nk);
        for k in 0..nk {
            let up = blocks[k].energies.clone();
            let down = blocks[k + nk].energies.clone();
            kpoints.push(EigenvalKpoint {
                coord: blocks[k].coord,
                weight: blocks[k].weight,
                energies: vec![up, down],
            });
        }
        Ok(Eigenval {
            nions,
            nspin,
            nbands,
            kpoints,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spin1() {
        // 头：NIONS NISPIN NBANDS = 1 1 2；3 个 k 点。
        let txt = "\
system
comment
   1   1   2
   0.0  0.0  0.0   1.0
   -5.0   1.0
   -2.0   1.0
   0.5  0.0  0.0   1.0
   -4.5   1.0
   -1.5   1.0
   0.0  0.5  0.0   1.0
   -4.0   1.0
   -1.0   1.0
";
        let e = parse_eigenval(txt).unwrap();
        assert_eq!(e.nspin, 1);
        assert_eq!(e.nbands, 2);
        assert_eq!(e.kpoints.len(), 3);
        assert_eq!(e.kpoints[0].coord, [0.0, 0.0, 0.0]);
        assert_eq!(e.kpoints[0].energies[0], vec![-5.0, -2.0]);
        assert_eq!(e.kpoints[2].energies[0], vec![-4.0, -1.0]);
    }

    #[test]
    fn parses_spin2_duplicate_blocks() {
        let txt = "\
system
comment
   1   2   1
   0.0  0.0  0.0   1.0
   -5.0   1.0
   0.0  0.0  0.0   1.0
   -3.0   1.0
";
        let e = parse_eigenval(txt).unwrap();
        assert_eq!(e.nspin, 2);
        assert_eq!(e.kpoints.len(), 1);
        assert_eq!(e.kpoints[0].energies, vec![vec![-5.0], vec![-3.0]]);
    }
}
