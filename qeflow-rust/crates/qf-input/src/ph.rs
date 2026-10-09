//! `ph.x` 输入写出（复刻 `qeflow/input_deck/ph.py`）。

use std::fs;

use anyhow::Result;
use qf_config::{Config, SectionMap};
use serde_json::json;
use serde_json::Value;

use crate::filenames::derive_filenames;
use crate::to_qe_value;

/// 由 `config.qe_phonon.ph` 构造 ph 网格配置（解析 qpoints / nq1..nq3）。
pub fn ph_grid_config(config: &Config, prefix: &str) -> SectionMap {
    let mut ph_cfg = config.qe_phonon.ph.clone();
    let (nq1, nq2, nq3) = resolve_grid(&ph_cfg, "qpoints", ("nq1", "nq2", "nq3"));
    ph_cfg.insert("nq1".to_string(), json!(nq1));
    ph_cfg.insert("nq2".to_string(), json!(nq2));
    ph_cfg.insert("nq3".to_string(), json!(nq3));
    ph_cfg.remove("qpoints");
    let names = derive_filenames(prefix);
    ph_cfg.insert("fildyn".to_string(), json!(names.fildyn));
    ph_cfg
}

/// 写出 `ph.in`（或 `split_ph.in`）。
pub fn write_ph_input(
    workspace: &str,
    config: &Config,
    prefix: &str,
    filename: &str,
    dyn0_only: bool,
) -> Result<String> {
    let mut ph_cfg = ph_grid_config(config, prefix);
    if dyn0_only {
        ph_cfg.insert("start_q".to_string(), json!(1));
        ph_cfg.insert("last_q".to_string(), json!(1));
    }
    write_inputph(workspace, filename, config, prefix, &ph_cfg)
}

/// 写出按 index 拆分的 `split_ph.in`（ldisp=true，单 q 点范围）。
pub fn write_split_ph_input_assign_index(
    q_dir: &str,
    config: &Config,
    prefix: &str,
    ph_q_index: i64,
) -> Result<String> {
    let mut ph_cfg = ph_grid_config(config, prefix);
    ph_cfg.insert("ldisp".to_string(), json!(true));
    ph_cfg.insert("start_q".to_string(), json!(ph_q_index));
    ph_cfg.insert("last_q".to_string(), json!(ph_q_index));
    write_inputph(q_dir, "split_ph.in", config, prefix, &ph_cfg)
}

/// 写出按 coords 拆分的 `split_ph.in`（ldisp=false，显式 q 点坐标）。
pub fn write_split_ph_input_assign_coords(
    q_dir: &str,
    config: &Config,
    prefix: &str,
    q_crystal: [f64; 3],
) -> Result<String> {
    let mut ph_cfg: SectionMap = config.qe_phonon.ph.clone();
    ph_cfg.insert("ldisp".to_string(), json!(false));
    let names = derive_filenames(prefix);
    ph_cfg.insert("fildyn".to_string(), json!(names.fildyn));
    ph_cfg.remove("qpoints");
    ph_cfg.remove("start_q");
    ph_cfg.remove("last_q");
    let path = write_inputph(q_dir, "split_ph.in", config, prefix, &ph_cfg)?;
    use std::io::Write;
    let mut f = fs::OpenOptions::new().append(true).open(&path)?;
    writeln!(
        f,
        "{:.15E} {:.15E} {:.15E}",
        q_crystal[0], q_crystal[1], q_crystal[2]
    )?;
    Ok(path)
}

fn write_inputph(
    workspace: &str,
    filename: &str,
    config: &Config,
    prefix: &str,
    ph_cfg: &SectionMap,
) -> Result<String> {
    let outdir = config
        .qe_phonon
        .scf
        .get("outdir")
        .and_then(|v| v.as_str())
        .unwrap_or("./out")
        .to_string();
    fs::create_dir_all(workspace)?;
    let path = format!("{workspace}/{filename}");
    let mut s = String::from("&inputph\n");
    s.push_str(&format!("  prefix = {}\n", to_qe_value(&json!(prefix))));
    s.push_str(&format!("  outdir = {}\n", to_qe_value(&json!(outdir))));
    for (k, v) in ph_cfg {
        if let Some(sv) = v.as_str() {
            if sv.is_empty() {
                continue;
            }
        }
        s.push_str(&format!("  {k} = {}\n", to_qe_value(v)));
    }
    s.push_str("/\n");
    fs::write(&path, &s)?;
    Ok(path)
}

/// 解析网格配置：优先 `vector_key`（形如 [n1,n2,n3]），否则回退 `nq1/nq2/nq3`，再否则默认 (1,1,1)。
pub fn resolve_grid(
    section: &SectionMap,
    vector_key: &str,
    fallback: (&str, &str, &str),
) -> (i64, i64, i64) {
    if let Some(Value::Array(a)) = section.get(vector_key) {
        if a.len() == 3 {
            return (
                a[0].as_i64().unwrap_or(1),
                a[1].as_i64().unwrap_or(1),
                a[2].as_i64().unwrap_or(1),
            );
        }
    }
    let v: [Option<i64>; 3] = [
        section.get(fallback.0).and_then(|v| v.as_i64()),
        section.get(fallback.1).and_then(|v| v.as_i64()),
        section.get(fallback.2).and_then(|v| v.as_i64()),
    ];
    if v[0].is_some() && v[1].is_some() && v[2].is_some() {
        return (v[0].unwrap(), v[1].unwrap(), v[2].unwrap());
    }
    (1, 1, 1)
}
