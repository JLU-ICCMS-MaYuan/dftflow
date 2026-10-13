//! `matdyn.x` 输入写出（复刻 `qeflow/input_deck/matdyn.py` 与 `input_deck/kpath.py`）。

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use qf_config::Config;
use serde_json::json;
use serde_json::Value;

use crate::filenames::derive_filenames;
use crate::ph::resolve_grid;
use crate::to_qe_value;

const KPATH_TOL: f64 = 1.0e-6;

/// `run_vaspkit_for_kpath`：KPATH.in 缺失时用 vaspkit 3→303 静默生成。
pub fn run_vaspkit_for_kpath(work_dir: &str) -> Result<()> {
    let output = Command::new("vaspkit")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .current_dir(work_dir)
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(b"3\n303\n");
            }
            child.wait_with_output()
        })
        .context("运行 vaspkit 生成 KPATH.in 失败（请确认 vaspkit 在 PATH 中）")?;
    if !output.status.success() {
        bail!("vaspkit 生成 KPATH.in 失败（cwd={work_dir}）");
    }
    Ok(())
}

struct KPathPoint {
    coords: [f64; 3],
    label: String,
}

/// `parse_kpath_segments`：每行 `x y z label`，相邻段共享端点时合并。
fn parse_kpath_segments(path: &str) -> Result<Vec<Vec<KPathPoint>>> {
    let text = fs::read_to_string(path)?;
    let mut points: Vec<KPathPoint> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }
        let Ok(kx) = parts[0].parse::<f64>() else { continue };
        let Ok(ky) = parts[1].parse::<f64>() else { continue };
        let Ok(kz) = parts[2].parse::<f64>() else { continue };
        points.push(KPathPoint {
            coords: [kx, ky, kz],
            label: parts[3].to_string(),
        });
    }
    if points.len() % 2 != 0 {
        bail!("KPATH.in 格式异常，点数为奇数: {}", points.len());
    }
    let mut segments: Vec<Vec<KPathPoint>> = Vec::new();
    for pair in points.chunks(2) {
        let (k1, k2) = (&pair[0], &pair[1]);
        match segments.last_mut() {
            Some(last) => {
                let prev_end = last.last().unwrap().coords;
                if (prev_end[0] - k1.coords[0]).abs() < KPATH_TOL
                    && (prev_end[1] - k1.coords[1]).abs() < KPATH_TOL
                    && (prev_end[2] - k1.coords[2]).abs() < KPATH_TOL
                {
                    last.push(KPathPoint {
                        coords: k2.coords,
                        label: k2.label.clone(),
                    });
                } else {
                    segments.push(vec![
                        KPathPoint {
                            coords: k1.coords,
                            label: k1.label.clone(),
                        },
                        KPathPoint {
                            coords: k2.coords,
                            label: k2.label.clone(),
                        },
                    ]);
                }
            }
            None => {
                segments.push(vec![
                    KPathPoint {
                        coords: k1.coords,
                        label: k1.label.clone(),
                    },
                    KPathPoint {
                        coords: k2.coords,
                        label: k2.label.clone(),
                    },
                ]);
            }
        }
    }
    Ok(segments)
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// `compute_reciprocal_lattice`：2π × inv(lattice).T。
fn reciprocal_lattice(lattice: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3]> {
    let volume = det3(lattice);
    if volume.abs() < 1.0e-12 {
        bail!("晶胞体积过小，无法计算倒格子。");
    }
    let inv = inv3(lattice);
    // 2π * inv.T：行变列。
    let mut recip = [[0.0_f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            recip[i][j] = std::f64::consts::PI * 2.0 * inv[j][i];
        }
    }
    Ok(recip)
}

fn inv3(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let d = det3(m);
    let mut inv = [[0.0_f64; 3]; 3];
    let cof = |a: f64, b: f64, c: f64, e: f64| (a * e - b * c) / d;
    inv[0][0] = cof(m[1][1], m[1][2], m[2][1], m[2][2]);
    inv[0][1] = -cof(m[0][1], m[0][2], m[2][1], m[2][2]);
    inv[0][2] = cof(m[0][1], m[0][2], m[1][1], m[1][2]);
    inv[1][0] = -cof(m[1][0], m[1][2], m[2][0], m[2][2]);
    inv[1][1] = cof(m[0][0], m[0][2], m[2][0], m[2][2]);
    inv[1][2] = -cof(m[0][0], m[0][2], m[1][0], m[1][2]);
    inv[2][0] = cof(m[1][0], m[1][1], m[2][0], m[2][1]);
    inv[2][1] = -cof(m[0][0], m[0][1], m[2][0], m[2][1]);
    inv[2][2] = cof(m[0][0], m[0][1], m[1][0], m[1][1]);
    inv
}

/// `write_kpath_labels`：写 `<prefix>_band.labelinfo.dat`（label / 索引 / 累计距离 / 坐标）。
fn write_kpath_labels(
    work_dir: &str,
    prefix: &str,
    segments: &[Vec<KPathPoint>],
    lattice: [[f64; 3]; 3],
    weights: &[i64],
) -> Result<String> {
    let recip_lat = reciprocal_lattice(lattice)?;
    let mut label_points: Vec<(&KPathPoint, bool)> = Vec::new();
    for (seg_idx, segment) in segments.iter().enumerate() {
        for (pt_idx, point) in segment.iter().enumerate() {
            label_points.push((point, seg_idx > 0 && pt_idx == 0));
        }
    }
    if label_points.len() != weights.len() {
        bail!("高对称点数量与权重数量不一致。");
    }
    let mut indices = vec![1usize];
    for i in 1..weights.len() {
        indices.push(indices[i - 1] + weights[i - 1] as usize);
    }
    let mut total_dist = 0.0_f64;
    let mut last_cart: Option<[f64; 3]> = None;
    let label_path = format!("{work_dir}/{prefix}_band.labelinfo.dat");
    let mut out = String::new();
    for ((point, is_jump), idx) in label_points.iter().zip(&indices) {
        let c = point.coords;
        let cart = [
            c[0] * recip_lat[0][0] + c[1] * recip_lat[1][0] + c[2] * recip_lat[2][0],
            c[0] * recip_lat[0][1] + c[1] * recip_lat[1][1] + c[2] * recip_lat[2][1],
            c[0] * recip_lat[0][2] + c[1] * recip_lat[1][2] + c[2] * recip_lat[2][2],
        ];
        if let Some(last) = last_cart {
            if !*is_jump {
                let d = (cart[0] - last[0]).powi(2)
                    + (cart[1] - last[1]).powi(2)
                    + (cart[2] - last[2]).powi(2);
                total_dist += d.sqrt();
            }
        }
        out.push_str(&format!(
            "{:<8} {:>12} {:>16.10} {:>16.10} {:>16.10} {:>16.10}\n",
            point.label, idx, total_dist, c[0], c[1], c[2]
        ));
        last_cart = Some(cart);
    }
    fs::write(&label_path, &out)?;
    Ok(label_path)
}

/// `build_matdyn_band_path`：解析 KPATH.in → (coords, label, weight) 列表 + labelinfo 路径。
fn build_matdyn_band_path(
    work_dir: &str,
    prefix: &str,
    lattice: [[f64; 3]; 3],
    kpath_file: &str,
    kpath_points: usize,
) -> Result<(Vec<([f64; 3], String, i64)>, String)> {
    let kpath_path = if Path::new(kpath_file).is_absolute() {
        kpath_file.to_string()
    } else {
        format!("{work_dir}/{kpath_file}")
    };
    if !Path::new(&kpath_path).exists() {
        bail!("找不到 KPATH.in: {kpath_path}");
    }
    let segments = parse_kpath_segments(&kpath_path)?;
    if segments.is_empty() {
        bail!("KPATH.in 未解析到任何高对称点。");
    }
    let mut kpoint_lines: Vec<([f64; 3], String, i64)> = Vec::new();
    let n_seg = segments.len();
    for (seg_idx, segment) in segments.iter().enumerate() {
        let last_pt = segment.len() - 1;
        for (pt_idx, point) in segment.iter().enumerate() {
            let is_last_point = pt_idx == last_pt;
            let is_jump = is_last_point && seg_idx < n_seg - 1;
            let is_final_point = is_last_point && seg_idx == n_seg - 1;
            let weight: i64 = if is_jump || is_final_point {
                1
            } else {
                kpath_points as i64
            };
            kpoint_lines.push((point.coords, point.label.clone(), weight));
        }
    }
    let weights: Vec<i64> = kpoint_lines.iter().map(|(_, _, w)| *w).collect();
    let label_path = write_kpath_labels(work_dir, prefix, &segments, lattice, &weights)?;
    Ok((kpoint_lines, label_path))
}

/// 写出声子谱 `matdyn_phband.in`（含 `q_in_cryst_coord = .true.`）与
/// `<prefix>_band.labelinfo.dat`。KPATH.in 缺失时自动调用 vaspkit 生成。
pub fn write_matdyn_phband_input(
    workspace: &str,
    config: &Config,
    prefix: &str,
    lattice: [[f64; 3]; 3],
) -> Result<String> {
    let band_cfg = config.qe_phonon.phband.clone();
    let names = derive_filenames(prefix);
    let kpath_file = band_cfg
        .get("kpath_file")
        .and_then(|v| v.as_str())
        .unwrap_or("KPATH.in")
        .to_string();
    let kpath_path = if Path::new(&kpath_file).is_absolute() {
        kpath_file.clone()
    } else {
        format!("{workspace}/{kpath_file}")
    };
    if !Path::new(&kpath_path).exists() {
        run_vaspkit_for_kpath(workspace)?;
    }
    let kpath_points = band_cfg
        .get("kpath_points")
        .and_then(|v| v.as_i64())
        .unwrap_or(100) as usize;
    let (path_points, _label_path) =
        build_matdyn_band_path(workspace, prefix, lattice, &kpath_file, kpath_points)?;

    fs::create_dir_all(workspace)?;
    let path = format!("{workspace}/matdyn_phband.in");
    let mut s = String::from("&input\n");
    let zasr = config
        .qe_phonon
        .q2r
        .get("zasr")
        .and_then(|v| v.as_str())
        .unwrap_or("simple");
    s.push_str(&format!("  asr = {}\n", to_qe_value(&json!(zasr))));
    s.push_str(&format!("  flfrc = {}\n", to_qe_value(&json!(names.flfrc))));
    s.push_str(&format!("  flfrq = {}\n", to_qe_value(&json!(names.flfrq))));
    s.push_str("  q_in_cryst_coord = .true.\n");
    if band_cfg
        .get("q_in_band_form")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
    {
        s.push_str("  q_in_band_form = .true.\n");
    }
    s.push_str("/\n");
    s.push_str(&format!("{}\n", path_points.len()));
    for (coords, _label, weight) in &path_points {
        s.push_str(&format!(
            "{:>16.10} {:>16.10} {:>16.10} {}\n",
            coords[0], coords[1], coords[2], weight
        ));
    }
    fs::write(&path, &s)?;
    Ok(path)
}

/// 写出声子 DOS `matdyn_phdos.in`。
pub fn write_matdyn_phdos_input(
    workspace: &str,
    config: &Config,
    prefix: &str,
) -> Result<String> {
    let dos_cfg = config.qe_phonon.phdos.clone();
    let names = derive_filenames(prefix);
    let (nk1, nk2, nk3) = resolve_grid(&dos_cfg, "qpoints", ("nk1", "nk2", "nk3"));

    fs::create_dir_all(workspace)?;
    let path = format!("{workspace}/matdyn_phdos.in");
    let mut s = String::from("&input\n");
    let zasr = config
        .qe_phonon
        .q2r
        .get("zasr")
        .and_then(|v| v.as_str())
        .unwrap_or("simple");
    s.push_str(&format!("  asr = {}\n", to_qe_value(&json!(zasr))));
    s.push_str(&format!("  flfrc = {}\n", to_qe_value(&json!(names.flfrc))));
    s.push_str(&format!("  fldos = {}\n", to_qe_value(&json!(names.fldos))));
    let dos_val = dos_cfg
        .get("dos")
        .cloned()
        .unwrap_or(Value::Bool(true));
    s.push_str(&format!("  dos = {}\n", to_qe_value(&dos_val)));
    s.push_str(&format!(
        "  ndos = {}\n",
        to_qe_value(&json!(dos_cfg.get("ndos").and_then(|v| v.as_i64()).unwrap_or(200)))
    ));
    s.push_str(&format!("  nk1 = {nk1}\n"));
    s.push_str(&format!("  nk2 = {nk2}\n"));
    s.push_str(&format!("  nk3 = {nk3}\n"));
    s.push_str("/\n");
    fs::write(&path, &s)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kpath_segments_merge_shared_endpoints() {
        let dir = std::env::temp_dir().join(format!("qf_matdyn_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("KPATH.in");
        fs::write(
            &path,
            "0.0 0.0 0.0 G\n0.5 0.0 0.0 X\n0.5 0.0 0.0 X\n0.5 0.5 0.0 M\n",
        )
        .unwrap();
        let segs = parse_kpath_segments(path.to_str().unwrap()).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reciprocal_matches_cross_formula() {
        let lat = [[4.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 4.0]];
        let recip = reciprocal_lattice(lat).unwrap();
        let want = std::f64::consts::PI / 2.0;
        assert!((recip[0][0] - want).abs() < 1e-12);
    }
}
