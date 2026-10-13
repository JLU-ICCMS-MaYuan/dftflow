//! VASP 纯后处理（`vasp_process_eledos` / `vasp_process_eband` / `vasp_process_cohp` /
//! `vasp_process_locpot` / `vasp_process_charge`）。
//!
//! 复刻对应 Python 脚本的 `--skip-vaspkit` 语义：直接读取 vaspkit / VASP / LOBSTER 生成的
//! 列文件与体积场文件，产出 1:1 数值 CSV 与 plotters PNG。外部调用由运行阶段负责。

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ===== 轨道定义（对齐 ORBITAL_GROUPS / PBAND_COMPONENT_INDEX） =====

pub const P_COMPONENTS: &[&str] = &["py", "pz", "px"];
pub const D_COMPONENTS: &[&str] = &["dxy", "dyz", "dz2", "dxz", "dx2-y2"];
pub const F_COMPONENTS: &[&str] = &["fy3x2", "fxyz", "fyz2", "fz3", "fxz2", "fzx2", "fx3"];

pub fn orbital_group(orbital: &str) -> Option<Vec<String>> {
    let o = orbital.to_lowercase();
    let comps: &[&str] = match o.as_str() {
        "s" => &["s"],
        "p" => P_COMPONENTS,
        "d" => D_COMPONENTS,
        "f" => F_COMPONENTS,
        "py" | "pz" | "px" | "dxy" | "dyz" | "dz2" | "dxz" | "dx2-y2" | "fy3x2" | "fxyz"
        | "fyz2" | "fz3" | "fxz2" | "fzx2" | "fx3" => &[o.as_str()],
        _ => return None,
    };
    Some(comps.iter().map(|s| s.to_string()).collect())
}

/// 支持的轨道集。
pub fn is_supported_orbital(o: &str) -> bool {
    orbital_group(o).is_some()
}

/// 解析 `元素:轨道,轨道` 列表为有序映射。
pub fn parse_element_specs(items: &[String]) -> Result<Vec<(String, Vec<String>)>> {
    let mut specs: Vec<(String, Vec<String>)> = Vec::new();
    for item in items {
        let (element, orbitals_text) = item
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("无效输入格式: {item}，应为 元素:轨道,轨道"))?;
        let mut orbitals = Vec::new();
        for orb in orbitals_text.split(',') {
            let orb = orb.trim().to_lowercase();
            if orb.is_empty() {
                continue;
            }
            if !is_supported_orbital(&orb) {
                bail!("不支持的轨道类型: {orb}");
            }
            if !orbitals.contains(&orb) {
                orbitals.push(orb);
            }
        }
        if orbitals.is_empty() {
            bail!("元素 {element} 未指定有效轨道");
        }
        specs.push((element.trim().to_string(), orbitals));
    }
    if specs.is_empty() {
        bail!("至少需要指定一个元素轨道组合");
    }
    Ok(specs)
}

fn normalize_header(name: &str) -> String {
    name.trim()
        .trim_start_matches('#')
        .replace(['(', ')'], "")
        .to_lowercase()
}

/// 读取 vaspkit 生成的 `.dat` 表（首行非数值为表头，其余为数据）。
pub fn read_dat_table(path: &str) -> Result<HashMap<String, Vec<f64>>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        bail!("{path} 为空");
    }
    let mut header: Option<Vec<String>> = None;
    let mut data_lines: Vec<Vec<f64>> = Vec::new();
    for line in &lines {
        let first = line.split_whitespace().next().unwrap_or("");
        if first.parse::<f64>().is_ok() {
            let row: Vec<f64> = line.split_whitespace().filter_map(|s| s.parse().ok()).collect();
            data_lines.push(row);
        } else if header.is_none() {
            header = Some(
                line.split_whitespace()
                    .map(normalize_header)
                    .filter(|s| !s.is_empty())
                    .collect(),
            );
        }
    }
    if data_lines.is_empty() {
        bail!("未从 {path} 解析到数值数据");
    }
    let ncols = data_lines[0].len();
    let mut header = header.unwrap_or_else(|| (0..ncols).map(|i| format!("col{i}")).collect());
    if header.len() != ncols {
        if header.len() > ncols {
            header.truncate(ncols);
        } else {
            let extra = (header.len()..ncols).map(|i| format!("col{i}"));
            header.extend(extra);
        }
    }
    let mut columns: HashMap<String, Vec<f64>> = HashMap::new();
    for (i, name) in header.iter().enumerate() {
        let col: Vec<f64> = data_lines.iter().map(|r| r[i]).collect();
        columns.insert(name.clone(), col);
    }
    Ok(columns)
}

fn find_column<'a>(columns: &'a HashMap<String, Vec<f64>>, candidates: &[&str]) -> Result<&'a Vec<f64>> {
    for c in candidates {
        if let Some(v) = columns.get(*c) {
            return Ok(v);
        }
    }
    bail!("缺少列: {}", candidates.join(", "))
}

fn sum_columns(columns: &HashMap<String, Vec<f64>>, names: &[String]) -> Result<Vec<f64>> {
    let mut total: Option<Vec<f64>> = None;
    for name in names {
        let col = columns
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("缺少轨道分量列: {name}"))?;
        match &mut total {
            Some(t) => {
                for (i, v) in col.iter().enumerate() {
                    t[i] += *v;
                }
            }
            None => total = Some(col.clone()),
        }
    }
    total.ok_or_else(|| anyhow::anyhow!("无可聚合列"))
}

// ===== eledos =====

pub fn interpolate_at_zero(energy: &[f64], values: &[f64]) -> f64 {
    if energy.is_empty() {
        return 0.0;
    }
    if energy[0] > 0.0 || energy[energy.len() - 1] < 0.0 {
        let idx = energy
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        return values[idx];
    }
    // 线性插值。
    for i in 0..energy.len() - 1 {
        if (energy[i] <= 0.0 && energy[i + 1] >= 0.0) || (energy[i] >= 0.0 && energy[i + 1] <= 0.0) {
            let e0 = energy[i];
            let e1 = energy[i + 1];
            let t = if (e1 - e0).abs() < 1e-12 {
                0.0
            } else {
                (0.0 - e0) / (e1 - e0)
            };
            return values[i] + t * (values[i + 1] - values[i]);
        }
    }
    values[0]
}

/// 运行 DOS 后处理（--skip-vaspkit 语义）：读 TDOS.dat + PDOS_*.dat → 写 CSV + at-EF CSV + PNG。
pub fn process_eledos(
    work_dir: &str,
    element_specs: &[(String, Vec<String>)],
    per_volume: bool,
    out_prefix: &str,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let poscar_path = Path::new(work_dir).join("POSCAR");
    let volume = read_poscar_volume(&poscar_path.to_string_lossy()).unwrap_or(1.0);

    let mut merged: Vec<(String, Vec<f64>)> = Vec::new();
    merged.push(("Energy(eV)".to_string(), Vec::new()));
    let mut energy_grid: Option<Vec<f64>> = None;

    for (element, orbitals) in element_specs {
        let pdos_path = Path::new(work_dir)
            .join(format!("PDOS_{element}.dat"))
            .to_string_lossy()
            .into_owned();
        let columns = read_dat_table(&pdos_path)?;
        let energy = find_column(&columns, &["energy", "energyev", "energyev"])?.clone();
        if let Some(ref e) = energy_grid {
            if e.len() != energy.len() || e.iter().zip(&energy).any(|(a, b)| (a - b).abs() > 1e-8) {
                bail!("能量网格不一致: {pdos_path}");
            }
        } else {
            energy_grid = Some(energy.clone());
            merged[0].1 = energy.clone();
        }
        for orbital in orbitals {
            let comps = orbital_group(orbital).unwrap();
            let mut vals = sum_columns(&columns, &comps)?;
            if per_volume {
                for v in vals.iter_mut() {
                    *v /= volume;
                }
            }
            merged.push((format!("{element}_{orbital}"), vals));
        }
        let tot = find_column(&columns, &["tot", "total"])?.clone();
        let tot = if per_volume {
            tot.iter().map(|v| v / volume).collect()
        } else {
            tot
        };
        merged.push((element.clone(), tot));
    }

    // TDOS
    let tdos_path = Path::new(work_dir).join("TDOS.dat").to_string_lossy().into_owned();
    let tdos_cols = read_dat_table(&tdos_path)?;
    let tdos_energy = find_column(&tdos_cols, &["energy", "energyev", "energyev"])?.clone();
    let tdos = {
        let c = ["tot", "dos", "tdos"]
            .iter()
            .find_map(|k| tdos_cols.get(*k))
            .or_else(|| {
                tdos_cols
                    .iter()
                    .find(|(k, _)| !["energy", "energyev", "int", "idos", "itdos"].contains(&k.as_str()))
                    .map(|(_, v)| v)
            });
        c.ok_or_else(|| anyhow::anyhow!("未能从 {tdos_path} 识别 TDOS 列"))?
            .clone()
    };
    if let Some(ref e) = energy_grid {
        if e.len() != tdos_energy.len()
            || e.iter()
                .zip(&tdos_energy)
                .any(|(a, b)| (a - b).abs() > 1e-8)
        {
            bail!("能量网格不一致: {tdos_path}");
        }
        merged[0].1 = tdos_energy.clone();
    } else {
        merged[0].1 = tdos_energy.clone();
    }
    let tdos = if per_volume {
        tdos.iter().map(|v| v / volume).collect()
    } else {
        tdos
    };
    merged.push(("TDOS".to_string(), tdos));

    let energy = &merged[0].1;
    let nrows = energy.len();
    let headers: Vec<String> = merged.iter().map(|(k, _)| k.clone()).collect();

    // merged CSV
    let suffix = if per_volume { "_per_volume" } else { "" };
    let merged_csv = Path::new(work_dir).join(format!("{out_prefix}{suffix}.csv"));
    write_csv(&merged_csv, &headers, &merged, nrows)?;

    // at-EF CSV
    let at_ef_csv = Path::new(work_dir).join(format!("{out_prefix}_at_ef{suffix}.csv"));
    let mut ef_headers = vec!["Energy(eV)".to_string()];
    ef_headers.extend(headers.iter().filter(|h| *h != "Energy(eV)").cloned());
    let mut ef_row = vec![0.0f64];
    for (_, vals) in merged.iter().skip(1) {
        ef_row.push(interpolate_at_zero(energy, vals));
    }
    {
        let mut text = String::new();
        text.push_str(&ef_headers.join(","));
        text.push('\n');
        text.push_str(&ef_row.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(","));
        text.push('\n');
        std::fs::write(&at_ef_csv, text).with_context(|| format!("write {}", at_ef_csv.display()))?;
    }

    // PNG
    let plot_png = Path::new(work_dir).join(format!("{out_prefix}{suffix}.png"));
    plot_dos(&merged_csv, &plot_png)?;

    let tdos_at_ef = interpolate_at_zero(energy, &merged.last().unwrap().1);
    println!("TDOS at E_F: {tdos_at_ef:.6}");
    Ok((merged_csv, at_ef_csv, plot_png))
}

fn read_poscar_volume(poscar_path: &str) -> Option<f64> {
    let text = std::fs::read_to_string(poscar_path).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 5 {
        return None;
    }
    let scale: f64 = lines[1].split_whitespace().next()?.parse().ok()?;
    let parse_vec = |l: &str| -> Option<[f64; 3]> {
        let p: Vec<f64> = l.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if p.len() < 3 {
            None
        } else {
            Some([p[0] * scale, p[1] * scale, p[2] * scale])
        }
    };
    let a1 = parse_vec(lines[2])?;
    let a2 = parse_vec(lines[3])?;
    let a3 = parse_vec(lines[4])?;
    let cross = [
        a2[1] * a3[2] - a2[2] * a3[1],
        a2[2] * a3[0] - a2[0] * a3[2],
        a2[0] * a3[1] - a2[1] * a3[0],
    ];
    let vol = (a1[0] * cross[0] + a1[1] * cross[1] + a1[2] * cross[2]).abs();
    Some(vol)
}

fn write_csv(path: &Path, headers: &[String], columns: &[(String, Vec<f64>)], nrows: usize) -> Result<()> {
    let mut text = String::new();
    text.push_str(&headers.join(","));
    text.push('\n');
    for r in 0..nrows {
        let row: Vec<String> = columns.iter().map(|(_, v)| format!("{}", v[r])).collect();
        text.push_str(&row.join(","));
        text.push('\n');
    }
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn plot_dos(csv_path: &Path, png_path: &Path) -> Result<()> {
    use plotters::prelude::*;
    let text = std::fs::read_to_string(csv_path).with_context(|| format!("read {}", csv_path.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let headers: Vec<&str> = lines[0].split(',').collect();
    let energy_idx = headers.iter().position(|h| *h == "Energy(eV)").unwrap_or(0);
    let mut series: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
    for (hi, h) in headers.iter().enumerate() {
        if *h == "Energy(eV)" {
            continue;
        }
        let mut pts = Vec::new();
        for line in lines.iter().skip(1) {
            let p: Vec<f64> = line.split(',').filter_map(|s| s.parse().ok()).collect();
            if p.len() > hi {
                pts.push((p[energy_idx], p[hi]));
            }
        }
        series.push((h.to_string(), pts));
    }
    let root = BitMapBackend::new(png_path.to_str().unwrap(), (800, 500)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut all_y = vec![0.0f64];
    for (_, pts) in &series {
        for (_, y) in pts {
            all_y.push(*y);
        }
    }
    let x_min = -10.0f64;
    let x_max = 10.0f64;
    let y_max = all_y.iter().cloned().fold(0.0f64, f64::max) * 1.2;
    let mut chart = ChartBuilder::on(&root)
        .caption("DOS", ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(x_min..x_max, 0.0..y_max.max(1.0))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart.configure_mesh().x_desc("Energy - E_F (eV)").y_desc("DOS").draw().map_err(|e| anyhow::anyhow!("{e}"))?;
    for (label, pts) in &series {
        chart
            .draw_series(LineSeries::new(pts.clone(), &BLUE))
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .label(label.clone())
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
    }
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .draw()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

// ===== eband =====

/// 解析 BAND.dat / PBAND_*.dat 的 block 结构（以 `# band-index N` 分隔）。
fn parse_band_blocks(path: &str) -> Result<Vec<Vec<Vec<f64>>>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let mut blocks: Vec<Vec<Vec<f64>>> = Vec::new();
    let mut current: Vec<Vec<f64>> = Vec::new();
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            let lower = s.to_lowercase();
            if lower.starts_with("# band-index") && !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            continue;
        }
        let p: Vec<f64> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        if p.len() >= 2 {
            current.push(p);
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    if blocks.is_empty() {
        bail!("未从 {path} 解析到 band 数据");
    }
    Ok(blocks)
}

/// 运行能带后处理（--skip-vaspkit 语义）：读 BAND.dat + PBAND_*.dat → 写 fatband CSV + PNG。
pub fn process_eband(
    work_dir: &str,
    element_specs: &[(String, Vec<String>)],
    fermi: Option<f64>,
    out_prefix: &str,
) -> Result<(PathBuf, PathBuf)> {
    let band_path = Path::new(work_dir).join("BAND.dat").to_string_lossy().into_owned();
    let band_blocks = parse_band_blocks(&band_path)?;

    // Fermi：显式 > FERMI_ENERGY > OUTCAR。
    let fermi = match fermi {
        Some(f) => f,
        None => {
            let fe = Path::new(work_dir).join("FERMI_ENERGY");
            if let Ok(t) = std::fs::read_to_string(&fe) {
                t.lines()
                    .filter_map(|l| l.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()))
                    .next()
                    .unwrap_or(0.0)
            } else {
                extract_fermi_from_outcar(&Path::new(work_dir).join("OUTCAR").to_string_lossy())
                    .unwrap_or(0.0)
            }
        }
    };

    // 读取各元素 PBAND。
    let mut pband_blocks: HashMap<String, Vec<Vec<Vec<f64>>>> = HashMap::new();
    for (element, _orbitals) in element_specs {
        let p = Path::new(work_dir)
            .join(format!("PBAND_{element}.dat"))
            .to_string_lossy()
            .into_owned();
        pband_blocks.insert(element.clone(), parse_band_blocks(&p)?);
    }

    // 校验 block 数一致。
    for (element, blocks) in &pband_blocks {
        if blocks.len() != band_blocks.len() {
            bail!("PBAND_{element}.dat 的 band 数与 BAND.dat 不一致");
        }
    }

    // 构建 CSV。
    let mut headers = vec![
        "band_index".to_string(),
        "kpoint_index".to_string(),
        "kpath_dist".to_string(),
        "energy_ev".to_string(),
    ];
    for (element, orbitals) in element_specs {
        for orbital in orbitals {
            headers.push(format!("weight_{element}_{orbital}"));
        }
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    let pband_component_index = |orbital: &str| -> Option<usize> {
        let m: HashMap<&str, usize> = [
            ("s", 2), ("py", 3), ("pz", 4), ("px", 5), ("dxy", 6), ("dyz", 7),
            ("dz2", 8), ("dxz", 9), ("dx2-y2", 10), ("fy3x2", 11), ("fxyz", 12),
            ("fyz2", 13), ("fz3", 14), ("fxz2", 15), ("fzx2", 16), ("fx3", 17),
        ]
        .into_iter()
        .collect();
        m.get(orbital).copied()
    };

    for (band_idx, band_block) in band_blocks.iter().enumerate() {
        for (point_idx, row) in band_block.iter().enumerate() {
            let kpath = row[0];
            let energy = row[1] - fermi;
            let mut out_row = vec![
                (band_idx + 1).to_string(),
                (point_idx + 1).to_string(),
                format!("{kpath}"),
                format!("{energy}"),
            ];
            for (element, orbitals) in element_specs {
                let pblocks = &pband_blocks[element];
                let prow = &pblocks[band_idx][point_idx];
                for orbital in orbitals {
                    let idx = pband_component_index(orbital).unwrap_or(0);
                    let w = prow.get(idx).copied().unwrap_or(0.0);
                    out_row.push(format!("{w}"));
                }
            }
            rows.push(out_row);
        }
    }

    let csv_path = Path::new(work_dir).join(format!("{out_prefix}.csv"));
    let mut text = String::new();
    text.push_str(&headers.join(","));
    text.push('\n');
    for r in &rows {
        text.push_str(&r.join(","));
        text.push('\n');
    }
    std::fs::write(&csv_path, text).with_context(|| format!("write {}", csv_path.display()))?;

    let png_path = Path::new(work_dir).join(format!("{out_prefix}.png"));
    plot_band(&csv_path, &png_path)?;
    Ok((csv_path, png_path))
}

fn extract_fermi_from_outcar(path: &str) -> Option<f64> {
    let text = std::fs::read_to_string(path).ok()?;
    let re = regex::Regex::new(r"E-fermi\s*:\s*([-\d.]+)").ok()?;
    re.captures(&text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn plot_band(csv_path: &Path, png_path: &Path) -> Result<()> {
    use plotters::prelude::*;
    let text = std::fs::read_to_string(csv_path).with_context(|| format!("read {}", csv_path.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let headers: Vec<&str> = lines[0].split(',').collect();
    let kpath_idx = headers.iter().position(|h| *h == "kpath_dist").unwrap_or(0);
    let energy_idx = headers
        .iter()
        .position(|h| *h == "energy_ev")
        .unwrap_or(1);
    // 按 band_index 分组成线。
    let mut bands: HashMap<usize, Vec<(f64, f64)>> = HashMap::new();
    for line in lines.iter().skip(1) {
        let p: Vec<f64> = line.split(',').filter_map(|s| s.parse().ok()).collect();
        if p.len() <= energy_idx {
            continue;
        }
        let b = p[0] as usize;
        bands.entry(b).or_default().push((p[kpath_idx], p[energy_idx]));
    }
    let root = BitMapBackend::new(png_path.to_str().unwrap(), (800, 500)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut x_all = Vec::new();
    let mut y_all = Vec::new();
    for pts in bands.values() {
        for (x, y) in pts {
            x_all.push(*x);
            y_all.push(*y);
        }
    }
    let (x_min, x_max) = bounds(&x_all, -1.0, 1.0);
    let (y_min, y_max) = bounds(&y_all, -5.0, 5.0);
    let mut chart = ChartBuilder::on(&root)
        .caption("Band", ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart.configure_mesh().x_desc("K-path").y_desc("Energy - E_F (eV)").draw().map_err(|e| anyhow::anyhow!("{e}"))?;
    for (_, pts) in &bands {
        chart
            .draw_series(LineSeries::new(pts.clone(), &RGBColor(0x80, 0x80, 0x80)))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    chart
        .draw_series(LineSeries::new(vec![(x_min, 0.0), (x_max, 0.0)], &BLACK))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn bounds(v: &[f64], dmin: f64, dmax: f64) -> (f64, f64) {
    if v.is_empty() {
        (dmin, dmax)
    } else {
        let lo = v.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (lo, hi)
    }
}

// ===== charge_diff =====

/// `vasp_charge_diff`：读取两份 VASP 电荷密度（PARCHG/CHGCAR），写出差分 `A - B`。
fn parse_vasp_charge_file(path: &str) -> Result<(Vec<String>, Vec<String>, (usize, usize, usize), Vec<f64>)> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    if lines.len() < 10 {
        bail!("文件内容过短，无法解析: {path}");
    }
    let is_int_tokens = |tokens: &[&str]| -> bool {
        !tokens.is_empty() && tokens.iter().all(|t| t.parse::<i64>().is_ok())
    };
    let line5_tokens: Vec<&str> = lines[5].split_whitespace().collect();
    let (counts_line_index, coord_line_index) = if is_int_tokens(&line5_tokens) {
        (5usize, 6usize)
    } else {
        (6usize, 7usize)
    };
    let mut coord_line_index = coord_line_index;
    let atom_count_line = lines[counts_line_index].trim().to_string();
    let mut coord_line = lines[coord_line_index].trim().to_string();
    if coord_line.to_lowercase().starts_with("selective") {
        coord_line_index += 1;
        coord_line = lines[coord_line_index].trim().to_string();
    }
    let coords_start = coord_line_index + 1;
    let atom_counts: Vec<usize> = atom_count_line
        .split_whitespace()
        .map(|x| x.parse())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("无法解析原子数行: {atom_count_line}"))?;
    let total_atoms: usize = atom_counts.iter().sum();
    let coord_end = coords_start + total_atoms;
    if coord_end >= lines.len() {
        bail!("坐标区长度不足: {path}");
    }
    let coords_block: Vec<String> = lines[coords_start..coord_end].to_vec();
    let mut grid_line_index = coord_end;
    let mut grid_line = lines[grid_line_index].trim().to_string();
    while grid_line.is_empty() && grid_line_index < lines.len() {
        grid_line_index += 1;
        grid_line = lines[grid_line_index].trim().to_string();
    }
    let grid_parts: Vec<&str> = grid_line.split_whitespace().collect();
    if grid_parts.len() != 3 {
        bail!("网格行格式异常: {grid_line}");
    }
    let grid: (usize, usize, usize) = (
        grid_parts[0].parse().map_err(|_| anyhow::anyhow!("网格行无法解析为整数: {grid_line}"))?,
        grid_parts[1].parse().map_err(|_| anyhow::anyhow!("网格行无法解析为整数: {grid_line}"))?,
        grid_parts[2].parse().map_err(|_| anyhow::anyhow!("网格行无法解析为整数: {grid_line}"))?,
    );
    let expected = grid.0 * grid.1 * grid.2;
    let mut data_lines: Vec<String> = Vec::new();
    for line in &lines[grid_line_index + 1..] {
        if line.trim().is_empty() {
            continue;
        }
        data_lines.extend(line.split_whitespace().map(|s| s.to_string()));
    }
    if data_lines.len() < expected {
        bail!("电荷密度数据不足: {} < {expected}", data_lines.len());
    }
    let data: Vec<f64> = data_lines[..expected]
        .iter()
        .map(|x| x.parse())
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| anyhow::anyhow!("电荷密度数值解析失败: {e}"))?;
    let header_block: Vec<String> = lines[..coords_start].to_vec();
    Ok((header_block, coords_block, grid, data))
}

fn write_vasp_charge_diff(
    path: &str,
    header: &[String],
    coords: &[String],
    grid: (usize, usize, usize),
    data: &[f64],
) -> Result<()> {
    let mut out = String::new();
    for line in header {
        out.push_str(line.trim_end_matches('\n'));
        out.push('\n');
    }
    for line in coords {
        out.push_str(line.trim_end_matches('\n'));
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&format!("  {} {} {}\n", grid.0, grid.1, grid.2));
    for (i, value) in data.iter().enumerate() {
        out.push_str(&format!("{value:12.4}"));
        if (i + 1) % 10 == 0 {
            out.push('\n');
        }
    }
    if data.len() % 10 != 0 {
        out.push('\n');
    }
    std::fs::write(path, out).with_context(|| format!("write {path}"))?;
    Ok(())
}

/// `vasp_charge_diff` 主逻辑：A - B → 输出文件（默认 A_minus_B.vasp）。
pub fn charge_diff(a_path: &str, b_path: &str, output: &str) -> Result<String> {
    let (a_header, a_coords, a_grid, a_data) = parse_vasp_charge_file(a_path)?;
    let (b_header, b_coords, b_grid, b_data) = parse_vasp_charge_file(b_path)?;
    if a_header != b_header {
        bail!("A 与 B 的头信息不一致，无法相减");
    }
    if a_coords != b_coords {
        bail!("A 与 B 的原子坐标不一致，无法相减");
    }
    if a_grid != b_grid {
        bail!("A 与 B 的网格不一致，无法相减");
    }
    let diff: Vec<f64> = a_data.iter().zip(&b_data).map(|(a, b)| a - b).collect();
    write_vasp_charge_diff(output, &a_header, &a_coords, a_grid, &diff)?;
    Ok(output.to_string())
}

// ===== idos_computed =====

/// `vasp_idos_computed`：指定能量区间内的 TDOS/PDOS 积分（复刻 `eledos/vasp_idos_computed.py`）。
///
/// 返回 (csv 路径)。能量坐标按 `E - E_F`（OUTCAR 费米能同步进 DOSCAR 后由 vaspkit 生成数据）。
#[allow(clippy::too_many_arguments)]
pub fn idos_computed(
    work_dir: &str,
    elements_text: &[String],
    low: f64,
    up: f64,
    scf_outcar: &str,
    doscar: &str,
    per_volume: bool,
    skip_vaspkit: bool,
    out_prefix: &str,
) -> Result<String> {
    if low >= up {
        bail!("要求 low < up。");
    }
    let specs = parse_element_specs(elements_text)?;
    // Python 用 dict 聚合：同元素后写覆盖先写，保持首次插入顺序。
    let mut merged_specs: Vec<(String, Vec<String>)> = Vec::new();
    for (element, orbitals) in specs {
        if let Some(entry) = merged_specs.iter_mut().find(|(e, _)| *e == element) {
            entry.1 = orbitals;
        } else {
            merged_specs.push((element, orbitals));
        }
    }

    let scf_outcar_path = Path::new(work_dir).join(scf_outcar).to_string_lossy().into_owned();
    let doscar_path = Path::new(work_dir).join(doscar).to_string_lossy().into_owned();
    let fermi = extract_fermi_from_outcar(&scf_outcar_path)
        .ok_or_else(|| anyhow::anyhow!("未能从 {scf_outcar_path} 解析 E-fermi。"))?;
    let final_fermi = replace_doscar_fermi(&doscar_path, fermi)?;
    println!("已将 DOSCAR 的费米能级更新为 {final_fermi:.10} eV");
    if !skip_vaspkit {
        println!("正在调用 vaspkit 生成 TDOS/PDOS 数据...");
        run_vaspkit_stdin(work_dir, "11\n113\n")?;
        run_vaspkit_stdin(work_dir, "11\n111\n")?;
    }

    let poscar_path = Path::new(work_dir).join("POSCAR").to_string_lossy().into_owned();
    let volume = read_poscar_volume(&poscar_path)
        .ok_or_else(|| anyhow::anyhow!("POSCAR 内容不完整，无法计算体积。"))?;

    let mut energy_ref: Option<Vec<f64>> = None;
    // data 保持插入顺序：(key, values)
    let mut data: Vec<(String, Vec<f64>)> = Vec::new();
    for (element, orbitals) in &merged_specs {
        let pdos_path = Path::new(work_dir)
            .join(format!("PDOS_{element}.dat"))
            .to_string_lossy()
            .into_owned();
        let columns = read_dat_table(&pdos_path)?;
        let energy = find_column(&columns, &["energy", "energyev", "energyev"])?.clone();
        match &energy_ref {
            None => energy_ref = Some(energy),
            Some(e) => ensure_same_grid(e, &energy, &pdos_path)?,
        }
        for orbital in orbitals {
            let comps = orbital_group(orbital)
                .ok_or_else(|| anyhow::anyhow!("未知轨道类型: {orbital}"))?;
            let mut values = sum_columns(&columns, &comps)?;
            if per_volume {
                for v in values.iter_mut() {
                    *v /= volume;
                }
            }
            data.push((format!("{element}_{orbital}"), values));
        }
    }
    let tdos_path = Path::new(work_dir).join("TDOS.dat").to_string_lossy().into_owned();
    let tdos_cols = read_dat_table(&tdos_path)?;
    let tdos_energy = find_column(&tdos_cols, &["energy", "energyev", "energyev"])?.clone();
    match &energy_ref {
        None => energy_ref = Some(tdos_energy.clone()),
        Some(e) => ensure_same_grid(e, &tdos_energy, &tdos_path)?,
    }
    let mut tdos_values: Vec<f64> = {
        let c = ["tot", "dos", "tdos"]
            .iter()
            .find_map(|k| tdos_cols.get(*k))
            .or_else(|| {
                tdos_cols
                    .iter()
                    .find(|(k, _)| !["energy", "energyev", "int", "idos", "itdos"].contains(&k.as_str()))
                    .map(|(_, v)| v)
            })
            .ok_or_else(|| anyhow::anyhow!("未能从 {tdos_path} 识别 TDOS 列。"))?;
        c.clone()
    };
    if per_volume {
        for v in tdos_values.iter_mut() {
            *v /= volume;
        }
    }
    let energy = energy_ref.context("能量网格为空")?;

    // 积分
    let tdos_idos = integrate_range_with_interpolation(&energy, &tdos_values, low, up)?;
    let mut results: Vec<(String, f64)> = vec![
        ("low_eV".to_string(), low),
        ("up_eV".to_string(), up),
        ("TDOS_IDOS".to_string(), tdos_idos),
    ];
    let mut selected_sum = 0.0_f64;
    for (key, values) in &data {
        let idos = integrate_range_with_interpolation(&energy, values, low, up)?;
        results.push((format!("{key}_IDOS"), idos));
        selected_sum += idos;
    }
    results.push(("sum_selected_PDOS_IDOS".to_string(), selected_sum));
    results.push((
        "difference_TDOS_minus_sum_selected".to_string(),
        tdos_idos - selected_sum,
    ));

    let csv_path = Path::new(work_dir).join(format!("{out_prefix}.csv"));
    let mut text = String::new();
    text.push_str(&results.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>().join(","));
    text.push('\n');
    text.push_str(
        &results
            .iter()
            .map(|(_, v)| format!("{v}"))
            .collect::<Vec<_>>()
            .join(","),
    );
    text.push('\n');
    std::fs::write(&csv_path, &text).with_context(|| format!("write {}", csv_path.display()))?;

    println!("晶胞体积: {volume:.6} Å^3");
    println!("积分区间: [{low:.6}, {up:.6}] eV");
    println!("TDOS_IDOS: {tdos_idos:.6}");
    for (key, value) in &results {
        if key.ends_with("_IDOS") && key != "TDOS_IDOS" {
            println!("{key}: {value:.6}");
        }
    }
    println!("sum_selected_PDOS_IDOS: {selected_sum:.6}");
    println!("difference_TDOS_minus_sum_selected: {:.6}", tdos_idos - selected_sum);
    println!("已生成: {}", csv_path.display());
    Ok(csv_path.to_string_lossy().into_owned())
}

fn ensure_same_grid(reference: &[f64], current: &[f64], path: &str) -> Result<()> {
    if reference.len() != current.len()
        || reference
            .iter()
            .zip(current)
            .any(|(a, b)| (a - b).abs() > 1e-8)
    {
        bail!("能量网格不一致: {path}");
    }
    Ok(())
}

fn integrate_range_with_interpolation(
    energy: &[f64],
    values: &[f64],
    low: f64,
    up: f64,
) -> Result<f64> {
    if low >= up {
        bail!("要求 low < up。");
    }
    if energy.len() != values.len() {
        bail!("能量数组与数值数组长度不一致。");
    }
    if energy.len() < 2 {
        bail!("至少需要两个能量点才能做积分。");
    }
    if low < energy[0] || up > energy[energy.len() - 1] {
        bail!(
            "积分区间 [{low}, {up}] 超出能量网格范围 [{}, {}]。",
            energy[0],
            energy[energy.len() - 1]
        );
    }
    let interp = |x: f64| -> f64 {
        // np.interp：单调网格线性插值（边界已由调用方校验）。
        for i in 0..energy.len() - 1 {
            if (energy[i] <= x && energy[i + 1] >= x) || (energy[i] >= x && energy[i + 1] <= x) {
                let e0 = energy[i];
                let e1 = energy[i + 1];
                let t = if (e1 - e0).abs() < 1e-15 {
                    0.0
                } else {
                    (x - e0) / (e1 - e0)
                };
                return values[i] + t * (values[i + 1] - values[i]);
            }
        }
        *values.last().unwrap()
    };
    let mut selected_energy = vec![low];
    let mut selected_values = vec![interp(low)];
    for (e, v) in energy.iter().zip(values) {
        if low < *e && *e < up {
            selected_energy.push(*e);
            selected_values.push(*v);
        }
    }
    selected_energy.push(up);
    selected_values.push(interp(up));
    // np.trapz
    let mut total = 0.0_f64;
    for i in 1..selected_energy.len() {
        total += 0.5 * (selected_values[i - 1] + selected_values[i]) * (selected_energy[i] - selected_energy[i - 1]);
    }
    Ok(total)
}

fn run_vaspkit_stdin(work_dir: &str, input_text: &str) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("vaspkit")
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("运行 vaspkit 失败（请确认 vaspkit 在 PATH 中）")?;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(input_text.as_bytes());
    }
    let status = child.wait()?;
    if !status.success() {
        bail!("vaspkit 执行失败，退出码: {status}");
    }
    Ok(())
}

/// 将 DOSCAR 第 6 行第 4 个 token 替换为新费米能（保留原小数位数/格式）。
fn replace_doscar_fermi(doscar_path: &str, new_fermi: f64) -> Result<f64> {
    let text = std::fs::read_to_string(doscar_path)
        .with_context(|| format!("未找到 DOSCAR: {doscar_path}"))?;
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    // 保留行尾换行信息。
    let had_trailing_newline = text.ends_with('\n');
    if lines.len() < 6 {
        bail!("DOSCAR 行数不足，无法修改费米能级。");
    }
    let line = lines[5].clone();
    let re = regex::Regex::new(r"\S+").unwrap();
    let matches: Vec<regex::Match> = re.find_iter(&line).collect();
    if matches.len() < 4 {
        bail!("DOSCAR 第 6 行格式不正确，无法修改费米能级。");
    }
    let old_token = matches[3].as_str().to_string();
    let replacement: String;
    let mut decimal_places = 10usize;
    if old_token.to_lowercase().contains('e') {
        let parts: Vec<&str> = regex::Regex::new(r"[eE]")
            .unwrap()
            .split(&old_token)
            .collect();
        let mantissa = *parts.first().context("DOSCAR 科学计数 token 解析失败")?;
        if let Some(dot) = mantissa.find('.') {
            decimal_places = mantissa[dot + 1..].len();
        }
        let r = format!("{:.*e}", decimal_places, new_fermi);
        replacement = if old_token.contains('E') { r.to_uppercase() } else { r };
    } else if old_token.contains('.') {
        decimal_places = old_token.split('.').nth(1).map(|s| s.len()).unwrap_or(0);
        replacement = format!("{:.*}", decimal_places, new_fermi);
    } else {
        replacement = format!("{}", new_fermi.round() as i64);
    }
    let start = matches[3].start();
    let end = matches[3].end();
    lines[5] = format!("{}{}{}", &line[..start], replacement, &line[end..]);
    let mut out = lines.join("\n");
    if had_trailing_newline {
        out.push('\n');
    }
    std::fs::write(doscar_path, out)?;
    replacement.parse().map_err(|e| anyhow::anyhow!("替换后费米能解析失败: {e}"))
}

// ===== cohp =====

struct InteractionSpec {
    left_elem: String,
    left_orbs: Vec<String>,
    right_elem: String,
    right_orbs: Vec<String>,
    label: String,
}

fn parse_interaction_spec(text: &str) -> Result<InteractionSpec> {
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() != 2 {
        bail!("interaction 格式错误: {text}，应为 ElementA:orb1,orb2|ElementB:orb1,orb2");
    }
    let parse_side = |s: &str| -> Result<(String, Vec<String>)> {
        let (elem, orbs_text) = s
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("相互作用片段格式错误: {s}"))?;
        let elem = elem.trim().to_string();
        let orbs = orbs_text
            .split(',')
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty())
            .collect::<Vec<_>>();
        if elem.is_empty() || orbs.is_empty() {
            bail!("相互作用片段格式错误: {s}");
        }
        Ok((elem, orbs))
    };
    let (left_elem, left_orbs) = parse_side(parts[0])?;
    let (right_elem, right_orbs) = parse_side(parts[1])?;
    let label = format!(
        "{}:{} | {}:{}",
        left_elem,
        left_orbs.join(","),
        right_elem,
        right_orbs.join(",")
    );
    Ok(InteractionSpec {
        left_elem,
        left_orbs,
        right_elem,
        right_orbs,
        label,
    })
}

/// 解析 COHPCAR 原始相互作用标签。
///
/// 真实 LOBSTER（≥ 2.2，含 6.0）标签形如：
/// - 原子对：`No.1:B2->B3(1.7746...)`
/// - 轨道分辨：`No.1:B2[2s]->B3[2p_y](1.7746...)`
/// `Average` 无 `(...)`，返回 `None`。
///
/// 注意：pymatgen 会把轨道标签规范化为纯轨道名（`2s-2s`，丢失元素），
/// 旧实现（Python/Rust）按虚构标签 `Mg1_3s-B2_2s` 匹配，对真实文件永远失配；
/// 此处直接对原始标签解析，保留元素与轨道信息。
#[derive(Debug, Clone, PartialEq)]
struct RawBondLabel {
    left_atom: String,
    left_orb: Option<String>,
    right_atom: String,
    right_orb: Option<String>,
}

fn parse_raw_cohp_label(label: &str) -> Option<RawBondLabel> {
    let body = label.trim().rsplit_once('(')?.0;
    let (left, right) = body.split_once("->")?;
    let parse_side = |s: &str| -> (String, Option<String>) {
        let s = s.trim();
        // 去掉 `No.X:` 前缀（取最后一个 `:` 之后的部分）
        let s = s.rsplit(':').next().unwrap_or(s);
        match s.split_once('[') {
            Some((atom, rest)) => {
                let orb = rest.trim().trim_end_matches(']').trim().to_string();
                (atom.trim().to_string(), Some(orb))
            }
            None => (s.to_string(), None),
        }
    };
    let (left_atom, left_orb) = parse_side(left);
    let (right_atom, right_orb) = parse_side(right);
    Some(RawBondLabel {
        left_atom,
        left_orb,
        right_atom,
        right_orb,
    })
}

/// 提取原子标签中的元素符号（前导字母，对齐 Python `extract_element`）。
fn atom_element(atom_label: &str) -> &str {
    let end = atom_label
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(atom_label.len());
    &atom_label[..end]
}

/// 轨道请求匹配（对齐 Python `orbital_matches`）：精确相等，或请求无 `_` 时前缀匹配 `orb_`。
fn orbital_matches(requested: &str, orbital_label: &str) -> bool {
    requested == orbital_label
        || (!requested.contains('_') && orbital_label.starts_with(&format!("{requested}_")))
}

/// 单侧匹配：元素相等且任一请求轨道命中。
fn side_matches_raw(side_elem: &str, side_orbs: &[String], atom: &str, orb: &str) -> bool {
    atom_element(atom) == side_elem
        && side_orbs
            .iter()
            .any(|o| orbital_matches(o, orb))
}

/// 运行 COHP 后处理（对齐 `cohp/vasp_plot_cohp.py`）：
/// 读 COHPCAR.lobster → `proj_cohp.dat` / `average_cohp.dat` + PNG + ICOHP 汇总。
#[allow(clippy::too_many_arguments)]
pub fn process_cohp(
    directory: &str,
    file_name: &str,
    interactions: &[String],
    lower_energy: Option<f64>,
    upper_energy: Option<f64>,
    emin: Option<f64>,
    emax: Option<f64>,
    out_prefix: Option<&str>,
) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let resolve_out = |stem_ext: &str| -> PathBuf {
        match out_prefix {
            Some(p) => Path::new(directory).join(format!("{p}_{stem_ext}")),
            None => Path::new(directory).join(stem_ext),
        }
    };
    let car_path = Path::new(directory).join(file_name).to_string_lossy().into_owned();
    let car = qf_io::vasp_cohpar::read_cohpcarpcar(&car_path)?;
    // 键数 = 原子对标签数（无 `[orb]`、含 `->` 的标签），对齐 Python `len(cohp_data)-1`；
    // 轨道分辨文件中每键含 1 条原子对 + N 条轨道标签，归一化分母是键数而非匹配轨道对数。
    let parsed_labels: Vec<Option<RawBondLabel>> =
        car.labels.iter().map(|l| parse_raw_cohp_label(l)).collect();
    let num_bonds = parsed_labels
        .iter()
        .filter(|p| {
            matches!(
                p.as_ref(),
                Some(RawBondLabel { left_orb: None, right_orb: None, .. })
            )
        })
        .count() as f64;

    let specs: Vec<InteractionSpec> = interactions
        .iter()
        .map(|s| parse_interaction_spec(s))
        .collect::<Result<_>>()?;

    // average
    let (avg_cohp, avg_icohp) = qf_io::vasp_cohpar::average_cohp(&car);
    let fermi_index = car
        .energies
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let avg_icohp_at_ef = avg_icohp[fermi_index];

    let avg_path = resolve_out("average_cohp.dat");
    let avg_headers = vec!["energy".to_string(), "COHP".to_string(), "ICOHP".to_string()];
    save_cohp_dat(
        &avg_path,
        &avg_headers,
        &car.energies,
        &[avg_cohp.clone(), avg_icohp.clone()],
    )?;

    // 各相互作用投影
    let mut proj_headers = vec!["energy".to_string()];
    let mut proj_series: Vec<Vec<f64>> = vec![car.energies.clone()];
    let mut icohp_at_ef_list = Vec::new();
    for spec in &specs {
        let mut total_cohp = vec![0.0f64; car.energies.len()];
        let mut total_icohp = vec![0.0f64; car.energies.len()];
        let mut matched = 0usize;
        for (k, parsed) in parsed_labels.iter().enumerate() {
            let Some(RawBondLabel { left_atom, left_orb: Some(lo), right_atom, right_orb: Some(ro) }) = parsed else {
                continue;
            };
            if side_matches_raw(&spec.left_elem, &spec.left_orbs, left_atom, lo)
                && side_matches_raw(&spec.right_elem, &spec.right_orbs, right_atom, ro)
            {
                for i in 0..car.energies.len() {
                    total_cohp[i] += car.cohp[k][i];
                    total_icohp[i] += car.icohp[k][i];
                }
                matched += 1;
            }
        }
        // 归一化分母 = 键数（对齐 Python `total /= num_bonds`），匹配不到时不归一
        if matched > 0 && num_bonds > 0.0 {
            for v in total_cohp.iter_mut() {
                *v /= num_bonds;
            }
            for v in total_icohp.iter_mut() {
                *v /= num_bonds;
            }
        }
        icohp_at_ef_list.push((spec.label.clone(), total_icohp[fermi_index], matched));
        proj_headers.push(format!("{}-COHP", spec.label));
        proj_headers.push(format!("{}-ICOHP", spec.label));
        proj_series.push(total_cohp);
        proj_series.push(total_icohp);
    }
    let proj_path = resolve_out("proj_cohp.dat");
    save_cohp_dat(&proj_path, &proj_headers, &car.energies, &proj_series[1..])?;

    // PNG
    let energy_range = match (emin, emax) {
        (Some(lo), Some(hi)) => Some((lo, hi)),
        _ => None,
    };
    let avg_png = resolve_out("average_cohp.png");
    plot_cohp_range(&avg_png, &car.energies, &[("COHP".to_string(), avg_cohp.clone()), ("ICOHP".to_string(), avg_icohp.clone())], energy_range)?;
    let proj_png = resolve_out("proj_cohp.png");
    plot_cohp_multi_range(&proj_png, &car.energies, &proj_headers[1..], &proj_series[1..], energy_range)?;

    println!("基于 {num_bonds} 个键分析 {car_path}");
    println!("--- ICOHP values at the Fermi level (E_Fermi = 0.000 eV) ---");
    for (label, value, matched) in &icohp_at_ef_list {
        if *matched > 0 {
            println!("  {:<20}: {:12.6}", format!("{label}-ICOHP"), value);
        } else {
            println!("  {:<20}: 未找到匹配贡献", format!("{label}-ICOHP"));
        }
    }
    println!("  {:<20}: {:12.6}", "average-ICOHP", avg_icohp_at_ef);

    // 能窗分析（对齐 Python `-low/--upper_energy` 分支）
    if let (Some(low), Some(up)) = (lower_energy, upper_energy) {
        let nearest = |x: f64| -> usize {
            car.energies
                .iter()
                .enumerate()
                .min_by(|a, b| (*a).1.abs().partial_cmp(&(*b).1.abs()).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        let _ = nearest; // 占位避免未使用告警（下方按真实目标值查找）
        let low_index = car
            .energies
            .iter()
            .enumerate()
            .min_by(|a, b| ((*a).1 - low).abs().partial_cmp(&((*b).1 - low).abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let up_index = car
            .energies
            .iter()
            .enumerate()
            .min_by(|a, b| ((*a).1 - up).abs().partial_cmp(&((*b).1 - up).abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let actual_low = car.energies[low_index];
        let actual_up = car.energies[up_index];
        println!();
        println!("--- ICOHP values in energy range [{actual_low:.3} eV, {actual_up:.3} eV] ---");
        println!(
            "{:<20} | {:>12} | {:>12} | {:>12} | {:>14}",
            "Interaction", "ICOHP @ EF", "ICOHP @ low", "ICOHP @ up", "ICOHP(up-low)"
        );
        println!("{}", "-".repeat(78));
        for (i, spec) in specs.iter().enumerate() {
            let label = format!("{}-ICOHP", spec.label);
            let col_index = proj_headers.iter().position(|h| *h == label);
            let Some(col_index) = col_index else { continue };
            let icohp_at_low = proj_series[col_index - 1][low_index];
            let icohp_at_up = proj_series[col_index - 1][up_index];
            let icohp_diff = icohp_at_up - icohp_at_low;
            println!(
                "{:<20} | {:12.6} | {:12.6} | {:12.6} | {:14.6}",
                label,
                icohp_at_ef_list[i].1,
                icohp_at_low,
                icohp_at_up,
                icohp_diff
            );
        }
        let avg_icohp_at_low = avg_icohp[low_index];
        let avg_icohp_at_up = avg_icohp[up_index];
        let avg_icohp_diff = avg_icohp_at_up - avg_icohp_at_low;
        println!(
            "{:<20} | {:12.6} | {:12.6} | {:12.6} | {:14.6}",
            "average-ICOHP", avg_icohp_at_ef, avg_icohp_at_low, avg_icohp_at_up, avg_icohp_diff
        );
        println!("{}", "-".repeat(78));
    }

    Ok((avg_path, avg_png, proj_path, proj_png))
}

/// Python `save_data`：表头 `"  ".join`，数据行 `%12.6f` 空格分隔。
fn save_cohp_dat(path: &Path, header_labels: &[String], energy: &[f64], series: &[Vec<f64>]) -> Result<()> {
    let mut text = String::new();
    text.push_str(&header_labels.iter().map(|l| l.clone()).collect::<Vec<_>>().join("  "));
    text.push('\n');
    for r in 0..energy.len() {
        let mut row = vec![format!("{:12.6}", energy[r])];
        for s in series {
            row.push(format!("{:12.6}", s[r]));
        }
        text.push_str(&row.join(" "));
        text.push('\n');
    }
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

// ===== plot_kcohp =====

/// k 解析相互作用（对齐 `cohp/orbital_interactions.py`，basis 标签为 `Mg_3s` 形式）。
struct KInteraction {
    left_elem: String,
    left_orbs: Vec<String>,
    right_elem: String,
    right_orbs: Vec<String>,
    label: String,
    filename_label: String,
}

fn sanitize_label(text: &str) -> String {
    let re = regex::Regex::new(r"[^A-Za-z0-9]+").unwrap();
    let safe = re.replace_all(text, "-").trim_matches('-').to_string();
    if safe.is_empty() { "interaction".to_string() } else { safe }
}

fn parse_k_interaction_spec(text: &str) -> Result<KInteraction> {
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() != 2 {
        bail!("interaction 格式错误: {text}。应为 ElementA:orb1,orb2|ElementB:orb1,orb2");
    }
    let parse_side = |s: &str| -> Result<(String, Vec<String>)> {
        let (elem, orbs_text) = s
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("相互作用片段格式错误: {s}。应为 Element:orb1,orb2"))?;
        let elem = elem.trim().to_string();
        let orbs: Vec<String> = orbs_text
            .split(',')
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty())
            .collect();
        if elem.is_empty() || orbs.is_empty() {
            bail!("相互作用片段格式错误: {s}。元素和轨道都不能为空。");
        }
        Ok((elem, orbs))
    };
    let (left_elem, left_orbs) = parse_side(parts[0].trim())?;
    let (right_elem, right_orbs) = parse_side(parts[1].trim())?;
    let label = format!(
        "{}:{} | {}:{}",
        left_elem,
        left_orbs.join(","),
        right_elem,
        right_orbs.join(",")
    );
    let filename_label = format!(
        "{}__{}",
        sanitize_label(&format!("{}_{}", left_elem, left_orbs.join("-"))),
        sanitize_label(&format!("{}_{}", right_elem, right_orbs.join("-")))
    );
    Ok(KInteraction {
        left_elem,
        left_orbs,
        right_elem,
        right_orbs,
        label,
        filename_label,
    })
}

fn side_matches_basis(elem: &str, orbs: &[String], basis_label: &str) -> bool {
    if !basis_label.contains('_') {
        return false;
    }
    let (atom_label, orbital_label) = basis_label.split_once('_').unwrap();
    if atom_element(atom_label) != elem {
        return false;
    }
    orbs.iter().any(|o| orbital_matches(o, orbital_label))
}

fn match_basis_indices(labels: &[String], elem: &str, orbs: &[String]) -> Vec<usize> {
    labels
        .iter()
        .enumerate()
        .filter(|(_, l)| side_matches_basis(elem, orbs, l))
        .map(|(i, _)| i)
        .collect()
}

/// 解析单个 `KspaceCOHPBand*.lobster`（`parse_single_band_cohp_file`）。
fn parse_single_band_cohp_file(
    filename: &str,
) -> Result<(Vec<f64>, Vec<Vec<Vec<f64>>>, Vec<String>)> {
    let content = std::fs::read_to_string(filename).with_context(|| format!("read {filename}"))?;
    let lines: Vec<&str> = content.lines().collect();
    let header_re = regex::Regex::new(r"kpoints\s+(\d+)").unwrap();
    let num_kpoints_decl = header_re
        .captures(lines.first().copied().unwrap_or(""))
        .and_then(|c| c[1].parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("无法从 {filename} 第一行解析 k 点数。"))?;
    let mut blocks: Vec<&str> = content.split("COHP_munujk matrix for spin").collect();
    blocks.remove(0);
    if blocks.is_empty() {
        bail!("{filename} 中没有 COHP 矩阵数据。");
    }
    let num_kpoints = num_kpoints_decl.min(blocks.len());
    let first_block_lines: Vec<&str> = blocks[0]
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if first_block_lines.len() < 3 {
        bail!("{filename} 中的第一个 COHP block 格式不完整。");
    }
    let labels: Vec<String> = first_block_lines[1]
        .split_whitespace()
        .skip(1)
        .map(|s| s.to_string())
        .collect();
    let num_orbitals = labels.len();
    let eigen_re = regex::Regex::new(r"eigenvalue\s+([-\d.]+)").unwrap();
    let mut energies = vec![0.0_f64; num_kpoints];
    let mut cohps = vec![vec![vec![0.0_f64; num_orbitals]; num_orbitals]; num_kpoints];
    for (ik, block) in blocks.iter().take(num_kpoints).enumerate() {
        let block_lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        let eigen = eigen_re
            .captures(block_lines.first().copied().unwrap_or(""))
            .and_then(|c| c[1].parse::<f64>().ok())
            .ok_or_else(|| anyhow::anyhow!("无法从 {filename} 的第 {} 个 block 读取 eigenvalue。", ik + 1))?;
        energies[ik] = eigen;
        let matrix_lines = &block_lines[2..2 + num_orbitals];
        if matrix_lines.len() != num_orbitals {
            bail!("{filename} 的第 {} 个 block 矩阵行数不完整。", ik + 1);
        }
        for (row_index, line) in matrix_lines.iter().enumerate() {
            let values: Vec<&str> = line.split_whitespace().skip(1).collect();
            if values.len() != num_orbitals {
                bail!("{filename} 的第 {} 个 block 矩阵列数不完整。", ik + 1);
            }
            for (col, v) in values.iter().enumerate() {
                cohps[ik][row_index][col] = v
                    .parse()
                    .map_err(|e| anyhow::anyhow!("COHP 矩阵数值解析失败: {e}"))?;
            }
        }
    }
    Ok((energies, cohps, labels))
}

fn kpoints_distances(
    directory: &str,
) -> Result<(Vec<f64>, Vec<(f64, String)>)> {
    let poscar = Path::new(directory).join("POSCAR").to_string_lossy().into_owned();
    let ptext = std::fs::read_to_string(&poscar).with_context(|| format!("read {poscar}"))?;
    let plines: Vec<&str> = ptext.lines().collect();
    let scale: f64 = plines[1].trim().parse().context("POSCAR scale 解析失败")?;
    let parse_vec = |line: &str| -> [f64; 3] {
        let vals: Vec<f64> = line.split_whitespace().take(3).filter_map(|s| s.parse().ok()).collect();
        [vals[0] * scale, vals[1] * scale, vals[2] * scale]
    };
    let a1 = parse_vec(plines[2]);
    let a2 = parse_vec(plines[3]);
    let a3 = parse_vec(plines[4]);
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    };
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let volume = dot(a1, cross(a2, a3));
    if volume.abs() < 1e-12 {
        bail!("POSCAR 晶胞体积接近 0，无法计算倒易晶格。");
    }
    let two_pi = 2.0 * std::f64::consts::PI;
    let recip: [[f64; 3]; 3] = [cross(a2, a3), cross(a3, a1), cross(a1, a2)].map(|v| v.map(|c| two_pi * c / volume));

    // KPATH.in 高对称点字典
    let kpath = Path::new(directory).join("KPATH.in").to_string_lossy().into_owned();
    let ktext = std::fs::read_to_string(&kpath).with_context(|| format!("read {kpath}"))?;
    let mut high_sym: Vec<([f64; 3], String)> = Vec::new();
    for raw_line in ktext.lines() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.contains('!')
            || line.contains("K-Path")
            || line.contains("Line-Mode")
            || line.contains("Reciprocal")
        {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let (Ok(x), Ok(y), Ok(z)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>(), parts[2].parse::<f64>())
        else {
            continue;
        };
        let mut label = parts.last().unwrap().to_string();
        if label.to_uppercase().contains("GAMMA") {
            label = "Γ".to_string();
        }
        high_sym.push(([x, y, z], label));
    }

    let kpoints = Path::new(directory).join("KPOINTS").to_string_lossy().into_owned();
    let kpt_text = std::fs::read_to_string(&kpoints).with_context(|| format!("read {kpoints}"))?;
    let klines: Vec<&str> = kpt_text.lines().collect();
    let parts: Vec<&str> = klines[0].split_whitespace().collect();
    if parts.len() < 7 {
        bail!("KPOINTS 第一行格式不正确，无法解析 fatband 路径信息。");
    }
    let num_ibz: usize = parts[4].parse().context("KPOINTS IBZ k 点数解析失败")?;
    let num_path: usize = parts[6].parse().context("KPOINTS 路径 k 点数解析失败")?;
    let start_line = 3 + num_ibz;
    let end_line = (start_line + num_path).min(klines.len());
    let mut path_coords: Vec<[f64; 3]> = Vec::new();
    for line in &klines[start_line..end_line] {
        let vals: Vec<&str> = line.split_whitespace().collect();
        if vals.len() < 3 {
            continue;
        }
        if let (Ok(x), Ok(y), Ok(z)) = (vals[0].parse::<f64>(), vals[1].parse::<f64>(), vals[2].parse::<f64>()) {
            path_coords.push([x, y, z]);
        }
    }
    if path_coords.is_empty() {
        bail!("KPOINTS 中未解析到高对称路径 k 点。");
    }
    let allclose = |a: &[f64; 3], b: &[f64; 3]| {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4 && (a[2] - b[2]).abs() < 1e-4
    };
    let mut distances = vec![0.0_f64];
    let mut high_sym_points: Vec<(f64, String)> = Vec::new();
    let first = path_coords[0];
    for (coord, label) in &high_sym {
        if allclose(&first, coord) {
            high_sym_points.push((0.0, label.clone()));
            break;
        }
    }
    for i in 1..path_coords.len() {
        let prev = path_coords[i - 1];
        let curr = path_coords[i];
        let cart = |k: [f64; 3]| {
            [
                dot(k, [recip[0][0], recip[1][0], recip[2][0]]),
                dot(k, [recip[0][1], recip[1][1], recip[2][1]]),
                dot(k, [recip[0][2], recip[1][2], recip[2][2]]),
            ]
        };
        let pc = cart(prev);
        let cc = cart(curr);
        let d = (cc[0] - pc[0]).powi(2) + (cc[1] - pc[1]).powi(2) + (cc[2] - pc[2]).powi(2);
        distances.push(distances[i - 1] + d.sqrt());
        for (coord, label) in &high_sym {
            if allclose(&path_coords[i], coord) {
                high_sym_points.push((distances[i - 1] + d.sqrt(), label.clone()));
                break;
            }
        }
    }
    // 去重（1e-6）
    let mut unique: Vec<(f64, String)> = Vec::new();
    let mut seen: Vec<f64> = Vec::new();
    let mut sorted_hs = high_sym_points.clone();
    sorted_hs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for (d, l) in sorted_hs {
        if !seen.iter().any(|v| (v - d).abs() < 1e-6) {
            seen.push(d);
            unique.push((d, l));
        }
    }
    Ok((distances, unique))
}

/// `vasp_plot_kcohp`：band-resolved pCOHP（KspaceCOHPBand*.lobster → .dat + .png）。
#[allow(clippy::too_many_arguments)]
pub fn plot_kcohp(
    directory: &str,
    interaction_texts: &[String],
    fermi: Option<f64>,
    emin: Option<f64>,
    emax: Option<f64>,
    vmax: f64,
    output_prefix: Option<&str>,
) -> Result<(PathBuf, PathBuf)> {
    let specs: Vec<KInteraction> = interaction_texts
        .iter()
        .map(|t| parse_k_interaction_spec(t))
        .collect::<Result<_>>()?;
    if specs.is_empty() {
        bail!("至少需要一个 --interaction，示例: --interaction \"Mg:3s,3p|B:2s,2p\"");
    }
    // 文件列表：按文件名中第一个数字排序
    let mut file_list: Vec<(usize, String)> = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("KspaceCOHPBand") && name.ends_with(".lobster") {
            let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
            let key: usize = digits.parse().unwrap_or(0);
            file_list.push((
                key,
                entry.path().to_string_lossy().into_owned(),
            ));
        }
    }
    file_list.sort_by_key(|(k, _)| *k);
    if file_list.is_empty() {
        bail!("当前目录下未找到 KspaceCOHPBand*.lobster 文件。");
    }

    let (k_distances, high_sym_points) = kpoints_distances(directory)?;
    let fermi_energy = match fermi {
        Some(f) => f,
        None => {
            let outcar = Path::new(directory).join("OUTCAR").to_string_lossy().into_owned();
            extract_fermi_from_outcar(&outcar)
                .ok_or_else(|| anyhow::anyhow!("未找到 OUTCAR，无法自动读取费米能，请使用 --fermi。"))?
        }
    };

    let mut labels: Option<Vec<String>> = None;
    let mut all_energies: Vec<Vec<f64>> = Vec::new();
    let mut pcohps_by_label: Vec<(String, Vec<Vec<f64>>)> =
        specs.iter().map(|s| (s.label.clone(), Vec::new())).collect();
    for (_, filename) in &file_list {
        let (energies, cohps, current_labels) = parse_single_band_cohp_file(filename)?;
        if labels.is_none() {
            labels = Some(current_labels);
        }
        let labels_ref = labels.as_ref().unwrap();
        let nk = k_distances.len();
        if energies.len() < nk {
            bail!(
                "{} 的 k 点数少于 KPOINTS 路径点数。",
                Path::new(filename).file_name().unwrap().to_string_lossy()
            );
        }
        all_energies.push(energies[..nk].to_vec());
        for (label, values) in pcohps_by_label.iter_mut() {
            let spec = specs.iter().find(|s| &s.label == label).unwrap();
            let left_idx = match_basis_indices(labels_ref, &spec.left_elem, &spec.left_orbs);
            let right_idx = match_basis_indices(labels_ref, &spec.right_elem, &spec.right_orbs);
            if left_idx.is_empty() || right_idx.is_empty() {
                bail!("相互作用 {} 未能匹配到 basis labels。", spec.label);
            }
            let mut pcohp = vec![0.0_f64; nk];
            for (ik, mat) in cohps.iter().take(nk).enumerate() {
                let mut acc = 0.0;
                for &i in &left_idx {
                    for &j in &right_idx {
                        acc += mat[i][j];
                    }
                }
                pcohp[ik] = acc;
            }
            values.push(pcohp);
        }
    }

    // 输出前缀
    let output_prefix = match output_prefix {
        Some(p) => p.to_string(),
        None => format!(
            "band_resolved_pcohp_{}",
            specs
                .iter()
                .map(|s| s.filename_label.clone())
                .collect::<Vec<_>>()
                .join("_and_")
        ),
    };
    let dat_path = Path::new(directory).join(format!("{output_prefix}.dat"));
    let png_path = Path::new(directory).join(format!("{output_prefix}.png"));

    // .dat：header `   `.join(f"{label:<24}")，行 `{kd:<24.10f} {e:<24.10f} {values}`
    let num_bands = all_energies.len();
    let nk = k_distances.len();
    let mut text = String::new();
    let mut header_labels = vec!["# K-Distance".to_string(), "Energy".to_string()];
    header_labels.extend(pcohps_by_label.iter().map(|(l, _)| l.clone()));
    text.push_str(
        &header_labels
            .iter()
            .map(|l| format!("{:<24}", l))
            .collect::<Vec<_>>()
            .join("   "),
    );
    text.push('\n');
    for ib in 0..num_bands {
        for ik in 0..nk {
            let mut row = format!(
                "{:<24.10} {:<24.10}",
                k_distances[ik], all_energies[ib][ik] - fermi_energy
            );
            for (_, values) in &pcohps_by_label {
                row.push_str(&format!(" {:<24.10}", values[ib][ik]));
            }
            text.push_str(&row);
            text.push('\n');
        }
        text.push('\n');
    }
    std::fs::write(&dat_path, &text).with_context(|| format!("write {}", dat_path.display()))?;

    // PNG：各相互作用的散点图（coolwarm_r 近似配色）
    plot_kcohp_png(&png_path, &k_distances, &all_energies, fermi_energy, &pcohps_by_label, &high_sym_points, emin, emax, vmax)?;
    println!("数据已保存至: {}", dat_path.display());
    println!("绘图已保存为: {}", png_path.display());
    Ok((dat_path, png_path))
}

#[allow(clippy::too_many_arguments)]
fn plot_kcohp_png(
    png_path: &Path,
    k_distances: &[f64],
    all_energies: &[Vec<f64>],
    fermi: f64,
    pcohps_by_label: &[(String, Vec<Vec<f64>>)],
    high_sym_points: &[(f64, String)],
    emin: Option<f64>,
    emax: Option<f64>,
    vmax: f64,
) -> Result<()> {
    use plotters::prelude::*;
    let n = pcohps_by_label.len().max(1);
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = ((n as f64) / cols as f64).ceil() as usize;
    let root = BitMapBackend::new(png_path.to_str().unwrap(), (900, 600)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let areas = root.split_evenly((rows, cols));
    for (area, (label, values)) in areas.into_iter().zip(pcohps_by_label) {
        let mut energies_flat: Vec<f64> = Vec::new();
        let mut xs: Vec<f64> = Vec::new();
        let mut cs: Vec<f64> = Vec::new();
        let mut band_lines: Vec<Vec<(f64, f64)>> = Vec::new();
        for (ib, e_row) in all_energies.iter().enumerate() {
            let mut band_line = Vec::new();
            for (ik, e) in e_row.iter().enumerate() {
                let shifted = e - fermi;
                band_line.push((k_distances[ik], shifted));
                energies_flat.push(shifted);
                xs.push(k_distances[ik]);
                cs.push(values[ib][ik]);
            }
            band_lines.push(band_line);
        }
        let x_min = k_distances.first().copied().unwrap_or(0.0);
        let x_max = k_distances.last().copied().unwrap_or(1.0);
        let (y_min, y_max) = match (emin, emax) {
            (Some(lo), Some(hi)) => (lo, hi),
            _ => {
                let lo = energies_flat.iter().cloned().fold(f64::INFINITY, f64::min);
                let hi = energies_flat.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                (lo, hi)
            }
        };
        let mut chart = ChartBuilder::on(&area)
            .caption(label, ("sans-serif", 16))
            .margin(8)
            .x_label_area_size(28)
            .y_label_area_size(46)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        chart.configure_mesh().disable_x_mesh().disable_y_mesh().draw().map_err(|e| anyhow::anyhow!("{e}"))?;
        // 能带灰线
        for band in &band_lines {
            chart
                .draw_series(std::iter::once(PathElement::new(
                    band.clone(),
                    RGBColor(153, 153, 153).stroke_width(1),
                )))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        // pCOHP 散点（coolwarm_r 近似：-vmax 红 → +vmax 蓝）
        let color = |v: f64| -> RGBColor {
            let t = ((v + vmax) / (2.0 * vmax)).clamp(0.0, 1.0);
            let r = (180.0 + (59.0 - 180.0) * t) as u8;
            let g = (4.0 + (76.0 - 4.0) * t) as u8;
            let b = (38.0 + (192.0 - 38.0) * t) as u8;
            RGBColor(r, g, b)
        };
        let points: Vec<(f64, f64, f64)> = xs
            .iter()
            .zip(&energies_flat)
            .zip(&cs)
            .map(|((x, y), c)| (*x, *y, *c))
            .collect();
        chart
            .draw_series(points.iter().map(|(x, y, c)| {
                Circle::new((*x, *y), 2, color(*c).filled())
            }))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![(x_min, 0.0), (x_max, 0.0)],
                BLACK.stroke_width(1),
            )))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    root.present().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn write_two_col_series(
    path: &Path,
    headers: &[String],
    energy: &[f64],
    series: &[Vec<f64>],
) -> Result<()> {
    let mut text = String::new();
    text.push_str(&headers.join("\t"));
    text.push('\n');
    for r in 0..energy.len() {
        let mut row = vec![format!("{}", energy[r])];
        for s in series {
            row.push(format!("{}", s[r]));
        }
        text.push_str(&row.join("\t"));
        text.push('\n');
    }
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn plot_cohp_range(
    png_path: &Path,
    energy: &[f64],
    series: &[(String, Vec<f64>)],
    energy_range: Option<(f64, f64)>,
) -> Result<()> {
    use plotters::prelude::*;
    let root = BitMapBackend::new(png_path.to_str().unwrap(), (800, 500)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (x_min, x_max) = bounds(energy, -10.0, 10.0);
    let mut y_all = vec![0.0f64];
    for (_, v) in series {
        y_all.extend_from_slice(v);
    }
    let (y_min, y_max) = match energy_range {
        Some((lo, hi)) => (lo, hi),
        None => bounds(&y_all, -1.0, 1.0),
    };
    let mut chart = ChartBuilder::on(&root)
        .caption("Average COHP / ICOHP", ("sans-serif", 22))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart.configure_mesh().x_desc("Energy (eV)").y_desc("COHP / ICOHP").draw().map_err(|e| anyhow::anyhow!("{e}"))?;
    for (label, v) in series {
        let color = if label.starts_with("ICO") {
            &RGBColor(0xe3, 0x29, 0x29)
        } else {
            &RGBColor(0x1f, 0x77, 0xb4)
        };
        chart
            .draw_series(LineSeries::new(energy.iter().cloned().zip(v.clone()), color))
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .label(label.clone())
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
    }
    chart
        .draw_series(LineSeries::new(vec![(0.0, y_min), (0.0, y_max)], &BLACK))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .draw()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn plot_cohp_multi_range(
    png_path: &Path,
    energy: &[f64],
    headers: &[String],
    series: &[Vec<f64>],
    energy_range: Option<(f64, f64)>,
) -> Result<()> {
    use plotters::prelude::*;
    let root = BitMapBackend::new(png_path.to_str().unwrap(), (800, 500)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (x_min, x_max) = bounds(energy, -10.0, 10.0);
    let mut y_all = vec![0.0f64];
    for v in series {
        y_all.extend_from_slice(v);
    }
    let (y_min, y_max) = match energy_range {
        Some((lo, hi)) => (lo, hi),
        None => bounds(&y_all, -1.0, 1.0),
    };
    let mut chart = ChartBuilder::on(&root)
        .caption("Projected COHP / ICOHP", ("sans-serif", 22))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart.configure_mesh().x_desc("Energy (eV)").y_desc("COHP / ICOHP").draw().map_err(|e| anyhow::anyhow!("{e}"))?;
    let palette = [
        RGBColor(0x1f, 0x77, 0xb4),
        RGBColor(0xe3, 0x29, 0x29),
        RGBColor(0x2c, 0xa0, 0x2c),
        RGBColor(0xff, 0x7f, 0x0e),
    ];
    for (i, v) in series.iter().enumerate() {
        if i % 2 == 1 {
            continue; // 仅画 COHP，避免与 ICOHP 重复
        }
        let color = palette[i / 2 % palette.len()];
        chart
            .draw_series(LineSeries::new(energy.iter().cloned().zip(v.clone()), color))
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .label(headers[i].clone())
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
    }
    chart
        .draw_series(LineSeries::new(vec![(0.0, y_min), (0.0, y_max)], &BLACK))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .draw()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

// ===== locpot / charge 体积场平面平均 =====

/// 运行 LOCPOT 后处理：读取体积场，输出 min/max 与平面平均 .dat + PNG。
/// `axis` 为平面平均方向（0=x,1=y,2=z）。
pub fn process_locpot(
    directory: &str,
    file_name: &str,
    axis: usize,
    out_prefix: &str,
) -> Result<(PathBuf, PathBuf)> {
    let path = Path::new(directory).join(file_name).to_string_lossy().into_owned();
    let vol = qf_io::vasp_locpot::read_vasp_volumetric(&path)?;
    let pa = qf_io::vasp_locpot::planar_average(&vol, axis);
    let vmin = vol.data.iter().cloned().fold(f64::INFINITY, f64::min);
    let vmax = vol.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("LOCPOT Min: {vmin:.6}");
    println!("LOCPOT Max: {vmax:.6}");

    let axis_len = vol.ngrid.0.max(vol.ngrid.1).max(vol.ngrid.2) as f64;
    let coord: Vec<f64> = (0..pa.len()).map(|i| i as f64 / pa.len() as f64 * axis_len).collect();
    let dat_path = Path::new(directory).join(format!("{out_prefix}_planar_average.dat"));
    let mut text = String::new();
    text.push_str("coordinate(Angstrom)\tV_locpot(eV)\n");
    for i in 0..pa.len() {
        text.push_str(&format!("{}\t{}\n", coord[i], pa[i]));
    }
    std::fs::write(&dat_path, text).with_context(|| format!("write {}", dat_path.display()))?;

    let png_path = Path::new(directory).join(format!("{out_prefix}_planar_average.png"));
    plot_line(&png_path, &coord, &pa, "Planar Average of LOCPOT", "Position (Å)", "V (eV)")?;
    Ok((dat_path, png_path))
}

/// 运行 CHGCAR 后处理：读取体积场，输出电荷密度平面平均 .dat + PNG。
pub fn process_charge(
    directory: &str,
    file_name: &str,
    axis: usize,
    out_prefix: &str,
) -> Result<(PathBuf, PathBuf)> {
    let path = Path::new(directory).join(file_name).to_string_lossy().into_owned();
    let vol = qf_io::vasp_locpot::read_vasp_volumetric(&path)?;
    let pa = qf_io::vasp_locpot::planar_average(&vol, axis);
    let dat_path = Path::new(directory).join(format!("{out_prefix}_planar_average.dat"));
    let axis_len = vol.ngrid.0.max(vol.ngrid.1).max(vol.ngrid.2) as f64;
    let coord: Vec<f64> = (0..pa.len()).map(|i| i as f64 / pa.len() as f64 * axis_len).collect();
    let mut text = String::new();
    text.push_str("coordinate(Angstrom)\trho(e/Angstrom^3)\n");
    for i in 0..pa.len() {
        text.push_str(&format!("{}\t{}\n", coord[i], pa[i]));
    }
    std::fs::write(&dat_path, text).with_context(|| format!("write {}", dat_path.display()))?;
    let png_path = Path::new(directory).join(format!("{out_prefix}_planar_average.png"));
    plot_line(&png_path, &coord, &pa, "Planar Average of Charge Density", "Position (Å)", "Density (e/Å³)")?;
    Ok((dat_path, png_path))
}

fn plot_line(
    png_path: &Path,
    x: &[f64],
    y: &[f64],
    caption: &str,
    x_desc: &str,
    y_desc: &str,
) -> Result<()> {
    use plotters::prelude::*;
    let root = BitMapBackend::new(png_path.to_str().unwrap(), (800, 500)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (x_min, x_max) = bounds(x, 0.0, 1.0);
    let (y_min, y_max) = bounds(y, -1.0, 1.0);
    let mut chart = ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 22))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart.configure_mesh().x_desc(x_desc).y_desc(y_desc).draw().map_err(|e| anyhow::anyhow!("{e}"))?;
    chart
        .draw_series(LineSeries::new(x.iter().cloned().zip(y.iter().cloned()), &BLACK))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
