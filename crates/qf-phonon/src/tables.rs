//! 声子后处理表格（对齐 `qeflow/phonon/phonon_tables.py`）。
//! 从 matdyn 产物构建声子谱 / DOS 的 CSV 行，并写出到 `phband/` 与 `phdos/`。

use anyhow::Result;
use std::path::Path;

use qf_io::{read_band_frequencies, read_labelinfo, read_numeric_rows};

use crate::archive::{PHBAND_DIR, PHDOS_DIR};

/// 从频率文件推断模式数（qe_plot 头 nbnd 或首行列数）。
pub fn detect_mode_count(text: &str) -> usize {
    let header = regex::Regex::new(r"(?i)&plot\s+nbnd\s*=\s*(\d+)").unwrap();
    for line in text.lines() {
        if let Some(c) = header.captures(line) {
            if let Some(m) = c.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    return n;
                }
            }
        }
    }
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let cols = qf_io::read_numeric_rows(line).first().map(|r| r.len());
        if let Some(n) = cols {
            if n > 0 {
                return n;
            }
        }
    }
    1
}

fn lerp(left: f64, right: f64, f: f64) -> f64 {
    left + (right - left) * f
}

/// 在 labels 之间线性插值出 `count` 个路径点（含坐标与距离）。
fn interpolate_band_points(labels: &[qf_io::LabelInfo], count: usize) -> Vec<qf_io::LabelInfo> {
    let mut points = Vec::with_capacity(count);
    for point_index in 1..=count {
        let (mut left, mut right) = (labels[0].clone(), labels[labels.len() - 1].clone());
        for w in 0..labels.len() - 1 {
            if labels[w].point_index <= point_index && point_index <= labels[w + 1].point_index {
                left = labels[w].clone();
                right = labels[w + 1].clone();
                break;
            }
        }
        let left_index = left.point_index as f64;
        let right_index = right.point_index as f64;
        let fraction = if (right_index - left_index).abs() < 1e-12 {
            0.0
        } else {
            (point_index as f64 - left_index) / (right_index - left_index)
        };
        points.push(qf_io::LabelInfo {
            label: String::new(),
            point_index,
            distance: lerp(left.distance, right.distance, fraction),
            kx: lerp(left.kx, right.kx, fraction),
            ky: lerp(left.ky, right.ky, fraction),
            kz: lerp(left.kz, right.kz, fraction),
        });
    }
    points
}

fn atom_names(elements: &[String], counts: &[usize]) -> Vec<String> {
    let mut names = Vec::new();
    for (e, c) in elements.iter().zip(counts.iter()) {
        for i in 1..=*c {
            names.push(format!("{e}_{i}"));
        }
    }
    names
}

/// 构建声子谱 CSV 行（band_rows + label_rows）。
pub fn build_phonon_band_tables(
    workspace: &str,
    prefix: &str,
    elements: &[String],
    counts: &[usize],
) -> Result<(Vec<Vec<String>>, Vec<Vec<String>>)> {
    let names = qf_input::filenames::derive_filenames(prefix);
    let label_path = format!("{workspace}/{PHBAND_DIR}/{prefix}_band.labelinfo.dat");
    let label_path = if Path::new(&label_path).exists() {
        label_path
    } else {
        format!("{workspace}/{prefix}_band.labelinfo.dat")
    };
    let freq_path = format!("{workspace}/{PHBAND_DIR}/{}", names.flfrq);
    let freq_path = if Path::new(&freq_path).exists() {
        freq_path
    } else {
        format!("{workspace}/{}", names.flfrq)
    };
    let labels = read_labelinfo(&std::fs::read_to_string(&label_path)?);
    if labels.is_empty() {
        anyhow::bail!("高对称点 labelinfo 为空。");
    }
    let freq_text = std::fs::read_to_string(&freq_path)?;
    let modes = detect_mode_count(&freq_text);
    let frequencies = read_band_frequencies(&freq_text, modes)?;
    let points = interpolate_band_points(&labels, frequencies.len());
    if points.len() != frequencies.len() {
        anyhow::bail!("声子谱路径点数量与频率数量不一致");
    }
    let mut band_header = vec![
        "point_index".to_string(),
        "distance".to_string(),
        "kx".to_string(),
        "ky".to_string(),
        "kz".to_string(),
    ];
    for m in 1..=modes {
        band_header.push(format!("mode_{m}_cm-1"));
    }
    let mut band_rows = Vec::new();
    for (point, modes_row) in points.iter().zip(frequencies.iter()) {
        let mut row = vec![
            point.point_index.to_string(),
            format!("{:.6}", point.distance),
            format!("{:.8}", point.kx),
            format!("{:.8}", point.ky),
            format!("{:.8}", point.kz),
        ];
        for v in modes_row {
            row.push(format!("{v:.6}"));
        }
        band_rows.push(row);
    }
    let label_header = vec![
        "label".to_string(),
        "point_index".to_string(),
        "distance".to_string(),
        "kx".to_string(),
        "ky".to_string(),
        "kz".to_string(),
    ];
    let label_rows: Vec<Vec<String>> = labels
        .iter()
        .map(|l| {
            vec![
                l.label.clone(),
                l.point_index.to_string(),
                format!("{:.6}", l.distance),
                format!("{:.8}", l.kx),
                format!("{:.8}", l.ky),
                format!("{:.8}", l.kz),
            ]
        })
        .collect();
    let mut band_all = vec![band_header];
    band_all.extend(band_rows);
    let mut label_all = vec![label_header];
    label_all.extend(label_rows);
    Ok((band_all, label_all))
}

/// 构建声子 DOS CSV 行（atoms_rows + elements_rows + element 表头）。
pub fn build_phonon_dos_tables(
    workspace: &str,
    prefix: &str,
    elements: &[String],
    counts: &[usize],
) -> Result<(Vec<Vec<String>>, Vec<Vec<String>>, Vec<String>)> {
    let names = qf_input::filenames::derive_filenames(prefix);
    let dos_path = format!("{workspace}/{PHDOS_DIR}/{}", names.fldos);
    let dos_path = if Path::new(&dos_path).exists() {
        dos_path
    } else {
        format!("{workspace}/{}", names.fldos)
    };
    let numeric_rows = read_numeric_rows(&std::fs::read_to_string(&dos_path)?);
    if numeric_rows.is_empty() || numeric_rows[0].len() < 2 {
        anyhow::bail!("声子 DOS 文件至少需要 frequency 与 TDOS 两列。");
    }
    let atom_names = atom_names(elements, counts);
    let max_atom_cols = numeric_rows[0].len() - 2;
    let selected = &atom_names[..atom_names.len().min(max_atom_cols)];
    let atoms_header: Vec<String> = {
        let mut h = vec!["frequency_cm-1".to_string(), "TDOS".to_string()];
        h.extend(selected.iter().cloned());
        h
    };
    let element_names: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        for e in elements {
            if !v.contains(e) {
                v.push(e.clone());
            }
        }
        v
    };
    let elements_header: Vec<String> = {
        let mut h = vec!["frequency_cm-1".to_string(), "TDOS".to_string()];
        h.extend(element_names.iter().cloned());
        h
    };
    let mut atoms_rows = Vec::new();
    let mut elements_rows = Vec::new();
    for values in &numeric_rows {
        let mut a_row = vec![format!("{:.6}", values[0]), format!("{:.6}", values[1])];
        let atom_values = &values[2..2 + selected.len()];
        for val in atom_values {
            a_row.push(format!("{val:.6}"));
        }
        atoms_rows.push(a_row);

        let mut e_totals: Vec<f64> = vec![0.0; element_names.len()];
        for (name, val) in selected.iter().zip(atom_values.iter()) {
            let symbol = name.split('_').next().unwrap_or(name);
            if let Some(idx) = element_names.iter().position(|e| e == symbol) {
                e_totals[idx] += *val;
            }
        }
        let mut e_row = vec![format!("{:.6}", values[0]), format!("{:.6}", values[1])];
        for v in &e_totals {
            e_row.push(format!("{v:.6}"));
        }
        elements_rows.push(e_row);
    }
    let mut atoms_all = vec![atoms_header];
    atoms_all.extend(atoms_rows);
    let mut elements_all = vec![elements_header];
    elements_all.extend(elements_rows);
    Ok((atoms_all, elements_all, element_names))
}
