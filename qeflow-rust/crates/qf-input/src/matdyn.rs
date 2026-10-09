//! `matdyn.x` 输入写出（复刻 `qeflow/input_deck/matdyn.py`）。

use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use qf_config::Config;
use qf_crystal::Structure;
use serde_json::json;
use serde_json::Value;

use crate::filenames::derive_filenames;
use crate::ph::resolve_grid;
use crate::to_qe_value;

/// 读取 KPATH.in 的高对称点（每行 4 字段：x y z label）。
fn read_kpath_points(path: &str) -> Result<Vec<([f64; 3], String)>> {
    let text = fs::read_to_string(path)
        .map_err(|_| anyhow::anyhow!("找不到 KPATH.in: {path}"))?;
    let mut pts = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let x = parts[0].parse::<f64>()?;
        let y = parts[1].parse::<f64>()?;
        let z = parts[2].parse::<f64>()?;
        pts.push(([x, y, z], parts[3].to_string()));
    }
    if pts.is_empty() {
        bail!("KPATH.in 未解析到任何高对称点: {path}");
    }
    Ok(pts)
}

/// 写出声子谱 `matdyn_phband.in`（含 `q_in_cryst_coord = .true.`）。
pub fn write_matdyn_phband_input(
    workspace: &str,
    config: &Config,
    _structure: &Structure,
    prefix: &str,
) -> Result<String> {
    let band_cfg = config.qe_phonon.phband.clone();
    let names = derive_filenames(prefix);
    let kpath_file = band_cfg
        .get("kpath_file")
        .and_then(|v| v.as_str())
        .unwrap_or("KPATH.in")
        .to_string();
    let kpath_path = if Path::new(&kpath_file).is_absolute() {
        kpath_file
    } else {
        format!("{workspace}/{kpath_file}")
    };
    let points = read_kpath_points(&kpath_path)?;
    let kpath_points = band_cfg
        .get("kpath_points")
        .and_then(|v| v.as_i64())
        .unwrap_or(100);

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
    s.push_str(&format!("{}\n", points.len()));
    let n = points.len();
    for (i, (coords, _label)) in points.iter().enumerate() {
        let is_seg_end = i == n - 1 || coords != &points[i + 1].0;
        let weight = if is_seg_end { 1 } else { kpath_points };
        s.push_str(&format!(
            "{:.10} {:.10} {:.10} {}\n",
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
