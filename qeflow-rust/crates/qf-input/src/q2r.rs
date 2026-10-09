//! `q2r.x` 输入写出（复刻 `qeflow/input_deck/q2r.py`）。

use std::fs;

use anyhow::Result;
use qf_config::Config;
use serde_json::json;

use crate::filenames::derive_filenames;
use crate::to_qe_value;

/// 写出 `q2r.in`。
pub fn write_q2r_input(workspace: &str, config: &Config, prefix: &str) -> Result<String> {
    let q2r_cfg = config.qe_phonon.q2r.clone();
    let names = derive_filenames(prefix);
    fs::create_dir_all(workspace)?;
    let path = format!("{workspace}/q2r.in");
    let mut s = String::from("&input\n");
    s.push_str(&format!("  fildyn = {}\n", to_qe_value(&json!(names.fildyn))));
    s.push_str(&format!("  flfrc = {}\n", to_qe_value(&json!(names.flfrc))));
    for (k, v) in &q2r_cfg {
        s.push_str(&format!("  {k} = {}\n", to_qe_value(v)));
    }
    s.push_str("/\n");
    fs::write(&path, &s)?;
    Ok(path)
}
