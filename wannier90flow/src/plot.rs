//! wannier_plot：QE 与 Wannier90 能带对比图（复刻 `wannier90flow/wannier90_plot.py`）。
//!
//! 绘图为 plotters 内容等价重绘（非像素级）：W90 蓝色实线、QE 红色虚线、
//! 高对称点竖线、y=0 虚线；数值解析逻辑 1:1 复刻。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// 简单后缀 glob（`*_band.dat` / `*_band.labelinfo.dat`）：目录内按匹配排序取第一个。
fn pick_file(directory: &Path, suffixes: &[&str], desc: &str) -> Result<PathBuf> {
    let mut matches: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("找不到 {desc}，目录: {}", directory.display()))?
    {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if suffixes.iter().any(|s| name.ends_with(s.trim_start_matches('*'))) {
            matches.push(entry.path());
        }
    }
    matches.sort();
    matches.dedup();
    if matches.is_empty() {
        anyhow::bail!("找不到 {desc}，目录: {}", directory.display());
    }
    if matches.len() > 1 {
        println!("Warning: 找到多个{desc}，默认使用 {}", matches[0].display());
    }
    Ok(matches[0].clone())
}

/// `resolve_inputs`。
pub fn resolve_inputs(qe_dir: &str, w90_dir: &str) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let qe_dir = std::fs::canonicalize(qe_dir).unwrap_or_else(|_| PathBuf::from(qe_dir));
    let w90_dir = std::fs::canonicalize(w90_dir).unwrap_or_else(|_| PathBuf::from(w90_dir));

    let qe_band_file = pick_file(&qe_dir, &["*_band.dat"], "QE 能带数据")?;
    let w90_band_file = pick_file(&w90_dir, &["*_band.dat"], "Wannier90 能带数据")?;
    let label_path = match pick_file(&w90_dir, &["*_band.labelinfo.dat"], "高对称点信息") {
        Ok(p) => p,
        Err(_) => pick_file(&qe_dir, &["*_band.labelinfo.dat"], "高对称点信息")?,
    };
    Ok((qe_band_file, w90_band_file, label_path))
}

/// 实空间基矢 → 倒空间基矢（含 2π）。
fn get_recip_base(cell: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let cross = |u: [f64; 3], v: [f64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let vol = dot(cell[0], cross(cell[1], cell[2]));
    let fac = 2.0 * std::f64::consts::PI / vol;
    [
        scale(cross(cell[1], cell[2]), fac),
        scale(cross(cell[2], cell[0]), fac),
        scale(cross(cell[0], cell[1]), fac),
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn norm(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

/// `parse_qe_plot_file`：QE bands.x 输出（头部 nbnd/nks + 逐 k 点块）。
/// 返回 (k 路径距离, 能带矩阵 [nk][nbnd] 减费米能)。
pub fn parse_qe_plot_file(
    qe_band_file: &Path,
    b_basis: &[[f64; 3]; 3],
    fermi_qe: f64,
) -> Result<(Vec<f64>, Vec<Vec<f64>>)> {
    let text = std::fs::read_to_string(qe_band_file)?;
    let lines: Vec<&str> = text.lines().collect();
    let header = lines[0].trim();

    // nbnd = X / nks = Y
    let mut nbnd: Option<usize> = None;
    let mut nks: Option<usize> = None;
    for tok in header.split_whitespace().collect::<Vec<_>>().windows(2) {
        if tok[0] == "nbnd" || tok[0].starts_with("nbnd") && tok[0].ends_with('=') {
            // 形如 "nbnd = 8"
        }
        let _ = tok;
    }
    // 简单扫描：找 "nbnd" 与数字、"nks" 与数字
    let mut it = header.split_whitespace().peekable();
    while let Some(tok) = it.next() {
        let cleaned = tok.trim_end_matches('=');
        let mut num: Option<usize> = None;
        if cleaned == "nbnd" {
            // 跳过 '='
            for nxt in it.clone() {
                if nxt == "=" {
                    continue;
                }
                num = nxt.trim_end_matches(',').parse().ok();
                break;
            }
        } else if cleaned == "nks" {
            for nxt in it.clone() {
                if nxt == "=" {
                    continue;
                }
                num = nxt.trim_end_matches(',').parse().ok();
                break;
            }
        }
        if let Some(n) = num {
            if cleaned == "nbnd" && nbnd.is_none() {
                nbnd = Some(n);
            } else if cleaned == "nks" && nks.is_none() {
                nks = Some(n);
            }
        }
    }
    if (nbnd.is_none() || nks.is_none()) && header.split_whitespace().count() >= 6 {
        // 回退：parts[2] 与 parts[5]（去除逗号）
        let parts: Vec<&str> = header.split_whitespace().collect();
        nbnd = parts[2].replace(',', "").parse().ok();
        nks = parts[5].replace(',', "").parse().ok();
    }
    let (Some(nbnd), Some(nks)) = (nbnd, nks) else {
        anyhow::bail!("无法解析 QE 能带文件头部: {header}");
    };

    let mut qe_k_dist: Vec<f64> = Vec::with_capacity(nks);
    let mut qe_bands: Vec<Vec<f64>> = Vec::with_capacity(nks);
    let mut cur_dist = 0.0f64;
    let mut prev_k_cart: Option<[f64; 3]> = None;
    let mut idx = 1usize;
    for _ in 0..nks {
        while idx < lines.len() && lines[idx].trim().is_empty() {
            idx += 1;
        }
        if idx >= lines.len() {
            break;
        }
        let parts: Vec<f64> = lines[idx]
            .split_whitespace()
            .filter_map(|x| x.parse().ok())
            .collect();
        let k_frac = [parts[0], parts[1], parts[2]];
        let k_cart = [
            dot(k_frac, b_basis[0]),
            dot(k_frac, b_basis[1]),
            dot(k_frac, b_basis[2]),
        ];
        if let Some(prev) = prev_k_cart {
            cur_dist += norm([k_cart[0] - prev[0], k_cart[1] - prev[1], k_cart[2] - prev[2]]);
        }
        qe_k_dist.push(cur_dist);
        prev_k_cart = Some(k_cart);

        idx += 1;
        let mut ebands: Vec<f64> = Vec::with_capacity(nbnd);
        while ebands.len() < nbnd && idx < lines.len() {
            ebands.extend(lines[idx].split_whitespace().filter_map(|x| x.parse::<f64>().ok()));
            idx += 1;
        }
        qe_bands.push(ebands);
    }

    let qe_bands: Vec<Vec<f64>> = qe_bands
        .into_iter()
        .map(|row| row.into_iter().map(|e| e - fermi_qe).collect())
        .collect();
    Ok((qe_k_dist, qe_bands))
}

/// gnuplot / W90 分段解析：空行或 `#` 分段；y 减费米能。
pub fn parse_segment_file(path: &Path, fermi: f64) -> Result<Vec<(Vec<f64>, Vec<f64>)>> {
    let text = std::fs::read_to_string(path)?;
    let mut segments: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    let mut x_vals: Vec<f64> = Vec::new();
    let mut y_vals: Vec<f64> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            if !x_vals.is_empty() {
                segments.push((x_vals.clone(), y_vals.iter().map(|y| y - fermi).collect()));
                x_vals.clear();
                y_vals.clear();
            }
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        x_vals.push(parts[0].parse().context("解析 x 值失败")?);
        y_vals.push(parts[1].parse().context("解析 y 值失败")?);
    }
    if !x_vals.is_empty() {
        segments.push((x_vals, y_vals.iter().map(|y| y - fermi).collect()));
    }
    Ok(segments)
}

/// `plot_comparison`：内容等价重绘。
pub fn plot_comparison(
    qe_band_file: &Path,
    w90_band_file: &Path,
    w90_label_file: &Path,
    fermi_energy: f64,
    output_img: &Path,
) -> Result<()> {
    if !qe_band_file.exists() {
        println!("Error: 找不到 QE 数据文件 {}", qe_band_file.display());
        return Ok(());
    }
    if !w90_band_file.exists() {
        println!("Error: 找不到 Wannier90 数据文件 {}", w90_band_file.display());
        return Ok(());
    }

    // 1:1 保留 Python 中的硬编码晶胞（其 parse_qe_plot_file 依赖，实际流未使用）
    #[allow(unused_variables)]
    let cell: [[f64; 3]; 3] = [
        [-1.6848999334, 1.6848999334, 1.6848999334],
        [1.6848999334, -1.6848999334, 1.6848999334],
        [1.6848999334, 1.6848999334, -1.6848999334],
    ];

    let _ = get_recip_base; // 保留倒格矢工具（Python 中 parse_qe_plot_file 的依赖，死代码路径）
    let qe_segments: Vec<(Vec<f64>, Vec<f64>)> =
        parse_segment_file(qe_band_file, fermi_energy)?;

    let w90_segments = parse_segment_file(w90_band_file, fermi_energy)?;
    if w90_segments.is_empty() {
        anyhow::bail!(
            "Wannier90 能带文件为空或无法解析: {}",
            w90_band_file.display()
        );
    }

    // 绘图
    use plotters::prelude::*;
    let x_max = qe_segments
        .iter()
        .flat_map(|(x, _)| x.iter().copied())
        .fold(f64::NEG_INFINITY, f64::max);
    let w90_min = w90_segments
        .iter()
        .flat_map(|(_, y)| y.iter().copied())
        .fold(f64::INFINITY, f64::min);
    let w90_max = w90_segments
        .iter()
        .flat_map(|(_, y)| y.iter().copied())
        .fold(f64::NEG_INFINITY, f64::max);

    // 高对称点
    let mut ticks: Vec<(f64, String)> = Vec::new();
    if w90_label_file.exists() {
        let text = std::fs::read_to_string(w90_label_file)?;
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let label = parts[0].replace("GAMMA", "Γ");
            if let Ok(dist) = parts[2].parse::<f64>() {
                ticks.push((dist, label));
            }
        }
    }

    let root = BitMapBackend::new(output_img, (1500, 1050)).into_drawing_area();
    root.fill(&WHITE)?;
    let y_pad = (w90_max - w90_min).abs() * 0.02;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Band Structure Comparison: QE (Dots) vs Wannier90 (Lines)",
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, (w90_min - y_pad)..(w90_max * 1.2))?;
    let mut mesh = chart.configure_mesh();
    mesh.y_desc("Energy - Ef (eV)").y_labels(10);
    // 以高对称点位置为 x 刻度（闭包与数据需活到函数结束）
    let coords: Vec<f64> = ticks.iter().map(|(d, _)| *d).collect();
    let labels: Vec<String> = ticks.iter().map(|(_, l)| l.clone()).collect();
    let fmt = |v: &f64| -> String {
        if coords.is_empty() {
            return String::new();
        }
        let nearest = coords
            .iter()
            .copied()
            .min_by(|a, b| (*a - v).abs().total_cmp(&(*b - v).abs()));
        match nearest {
            Some(d) if (d - v).abs() < 1e-6 => labels
                [coords.iter().position(|&c| c == d).unwrap_or(0)]
                .clone(),
            _ => String::new(),
        }
    };
    if !ticks.is_empty() {
        mesh.x_label_formatter(&fmt);
        mesh.x_labels(coords.len());
    } else {
        mesh.x_desc("k-path");
    }
    mesh.draw()?;

    // W90 蓝色实线（底层）
    for (k, e) in &w90_segments {
        chart.draw_series(LineSeries::new(
            k.iter().zip(e.iter()).map(|(x, y)| (*x, *y)),
            BLUE.stroke_width(2),
        ))?;
    }
    // QE 红色虚线（上层）
    for (x, y) in &qe_segments {
        chart.draw_series(
            x.iter()
                .zip(y.iter())
                .map(|(x, y)| Circle::new((*x, *y), 2, RED.filled())),
        )?;
    }
    // y = 0 参考线
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(0.0, 0.0), (x_max, 0.0)],
        BLACK.mix(0.5).stroke_width(1),
    )))?;
    // 高对称点竖线
    for (d, _) in &ticks {
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(*d, w90_min - y_pad), (*d, w90_max * 1.2)],
            BLACK.mix(0.3).stroke_width(1),
        )))?;
    }
    // 图例
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;
    root.present()?;
    println!("Comparison plot saved to {}", output_img.display());
    Ok(())
}

/// wannier_plot 主流程。
pub fn run(qe: &str, w90: &str, fermi: f64, out: &str) -> Result<()> {
    let (qe_band_file, w90_band_file, label_file) = resolve_inputs(qe, w90)?;
    plot_comparison(&qe_band_file, &w90_band_file, &label_file, fermi, Path::new(out))
}
