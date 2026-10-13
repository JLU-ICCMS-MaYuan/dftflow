//! VASP 体积场文件（`LOCPOT` / `CHGCAR` / `PARCHG`）解析。
//!
//! 文件头部与 POSCAR 一致：注释行 + 缩放因子 + 3 个格矢 + 元素行 + 计数行 +
//! （可选选择性动力学标记 `s`/`d`/`c`）+ 原子坐标；其后可能有 CHGCAR 的
//! augmentation 占位数数据，再接 `NGX NGY NGZ` 网格维度行，最后是平面（每 5 个一组）
//! 排布的网格数据，按 Fortran 顺序（x 最快，z 最慢）。`LOCPOT` 无 augmentation 段。
//!
//! 复刻 `vaspflow/locpot/vasp_process_locpot.py::read_vasp_volumetric` 的解析语义。

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub struct Volumetric {
    pub lattice: [[f64; 3]; 3],
    pub species: Vec<String>,
    pub counts: Vec<usize>,
    pub ngrid: (usize, usize, usize),
    /// 首块（总电荷密度 / 总势）网格数据，Fortran 顺序 reshape 为 (nx, ny, nz)。
    pub data: Vec<f64>,
    /// 文件包含的完整数据块数（1 = 仅总场；2 = 含自旋密度块）。
    pub n_blocks: usize,
}

fn parse_vec3(s: &str) -> Option<[f64; 3]> {
    let v: Vec<f64> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    if v.len() >= 3 {
        Some([v[0], v[1], v[2]])
    } else {
        None
    }
}

pub fn read_vasp_volumetric(path: &str) -> Result<Volumetric> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    if lines.len() < 8 {
        bail!("{path} 过短，无法解析体积场文件");
    }

    // 注释行 (0) + 缩放/格矢 (1..4 或 1 + 2..5)
    let mut i = 1;
    let first = lines[i].split_whitespace().collect::<Vec<&str>>();
    let numbers: Vec<f64> = first.iter().filter_map(|s| s.parse().ok()).collect();
    let scale;
    let lattice: [[f64; 3]; 3];
    if numbers.len() >= 3 {
        // 第一行直接是格矢（无统一缩放）
        let a = parse_vec3(lines[i]).ok_or_else(|| anyhow::anyhow!("格矢解析失败"))?;
        let b = parse_vec3(lines[i + 1]).ok_or_else(|| anyhow::anyhow!("格矢解析失败"))?;
        let c = parse_vec3(lines[i + 2]).ok_or_else(|| anyhow::anyhow!("格矢解析失败"))?;
        scale = 1.0;
        lattice = [a, b, c];
        i += 3;
    } else {
        scale = numbers[0];
        lattice = [
            parse_vec3(lines[i + 1]).context("格矢 a 解析失败")?,
            parse_vec3(lines[i + 2]).context("格矢 b 解析失败")?,
            parse_vec3(lines[i + 3]).context("格矢 c 解析失败")?,
        ];
        i += 4;
    }
    let lattice = [
        [lattice[0][0] * scale, lattice[0][1] * scale, lattice[0][2] * scale],
        [lattice[1][0] * scale, lattice[1][1] * scale, lattice[1][2] * scale],
        [lattice[2][0] * scale, lattice[2][1] * scale, lattice[2][2] * scale],
    ];

    // 元素行 / 计数行
    let tokens = lines[i].split_whitespace().collect::<Vec<&str>>();
    let all_int = tokens.iter().all(|t| t.parse::<i64>().is_ok());
    let (species, counts): (Vec<String>, Vec<usize>);
    if all_int {
        species = Vec::new();
        counts = tokens.iter().map(|t| t.parse().unwrap()).collect();
        i += 1;
    } else {
        species = tokens.iter().map(|s| s.to_string()).collect();
        i += 1;
        counts = lines[i]
            .split_whitespace()
            .map(|t| t.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .context("元素计数解析失败")?;
        i += 1;
    }

    // 可选选择性动力学标记行
    if let Some(next) = lines.get(i) {
        let low = next.trim().to_lowercase();
        if low.starts_with('s') || low.starts_with('d') || low.starts_with('c') {
            i += 1;
        }
    }

    // 原子坐标行
    let n_atoms: usize = counts.iter().sum();
    i += n_atoms;

    // 跳过空行 + 可能的 CHGCAR augmentation 段，直到出现恰好 3 个整数的网格维度行
    let mut grid_line_idx = None;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 3 && parts.iter().all(|p| p.parse::<i64>().is_ok()) {
            grid_line_idx = Some(i);
            break;
        }
        i += 1;
        if i - (i.saturating_sub(1)) > 0 && i > lines.len() {
            break;
        }
        // augmentation 段：继续跳过（不解析其内容）
    }
    let grid_line_idx =
        grid_line_idx.ok_or_else(|| anyhow::anyhow!("{path} 未找到网格维度行 NGX NGY NGZ"))?;
    let dims: Vec<usize> = lines[grid_line_idx]
        .split_whitespace()
        .map(|t| t.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .context("网格维度解析失败")?;
    let (nx, ny, nz) = (dims[0], dims[1], dims[2]);
    let ngrid = nx * ny * nz;

    // 读取网格数据（可能含多个块，每块 ngrid 个值）
    let mut values: Vec<f64> = Vec::new();
    let mut j = grid_line_idx + 1;
    while j < lines.len() && values.len() < ngrid {
        for t in lines[j].split_whitespace() {
            if let Ok(v) = t.parse::<f64>() {
                values.push(v);
            }
        }
        j += 1;
    }
    if values.len() < ngrid {
        bail!(
            "{path} 网格数据不足：读取 {}，需要 {}（网格 {}x{}x{}）",
            values.len(),
            ngrid,
            nx,
            ny,
            nz
        );
    }

    // 估算完整块数（文件中剩余数值能容纳多少个 ngrid）
    let total_available = {
        let mut extra = 0usize;
        while j < lines.len() {
            for t in lines[j].split_whitespace() {
                if t.parse::<f64>().is_ok() {
                    extra += 1;
                }
            }
            j += 1;
        }
        ngrid + extra
    };
    let n_blocks = (total_available / ngrid).max(1);

    Ok(Volumetric {
        lattice,
        species,
        counts,
        ngrid: (nx, ny, nz),
        data: values[..ngrid].to_vec(),
        n_blocks,
    })
}

/// 沿某晶格轴（0=x,1=y,2=z）做平面平均，返回 `ngrid[axis]` 个平均值（沿该轴位置）。
pub fn planar_average(v: &Volumetric, axis: usize) -> Vec<f64> {
    let (nx, ny, nz) = v.ngrid;
    let n = match axis {
        0 => nx,
        1 => ny,
        2 => nz,
        _ => panic!("axis 必须为 0/1/2"),
    };
    let mut sum = vec![0.0f64; n];
    let mut cnt = vec![0usize; n];
    for k in 0..nx {
        for l in 0..ny {
            for m in 0..nz {
                let idx = k + l * nx + m * nx * ny; // Fortran 顺序
                let p = match axis {
                    0 => k,
                    1 => l,
                    2 => m,
                    _ => unreachable!(),
                };
                sum[p] += v.data[idx];
                cnt[p] += 1;
            }
        }
    }
    sum.iter()
        .zip(cnt)
        .map(|(s, c)| if c > 0 { *s / c as f64 } else { 0.0 })
        .collect()
}
