//! split 输入生成：枚举 full q 网格，逐 q 点写出 `NN/scf.in`、`NN/split_ph.in`、
//! `NN/run_split.sh` 并登记 `q_index_map.json`（对齐 `qeflow/phonon/split_prepare.py`
//! 与 `q_index_map.py`）。q 点来源优先 `q_points.json`（旧目录兼容，晶体坐标），
//! 缺失时回退解析 `<prefix>.dyn0`。

use anyhow::{Context, Result};
use std::path::Path;

use qf_config::Config;
use qf_crystal::Structure;

use crate::index_map::{has_q_index_map, normalize_mode, save_q_index_map, QIndexEntry, QIndexMap};

/// 单个待拆分 q 点条目（对齐 `split_prepare._load_split_q_entries`）。
#[derive(Debug, Clone)]
pub struct SplitQEntry {
    pub directory: String,
    pub index: usize,
    pub q_crystal: [f64; 3],
    pub q_cart: Option<[f64; 3]>,
}

/// 加载拆分所需 q 点条目：优先 `q_points.json`，回退 `<prefix>.dyn0`（目录名 `NN`）。
pub fn load_split_q_entries(workspace: &str, prefix: &str) -> Result<Vec<SplitQEntry>> {
    // 1) q_points.json（仅为已有计算目录保留读取兼容）
    let qp_path = format!("{workspace}/q_points.json");
    if let Ok(text) = std::fs::read_to_string(&qp_path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            let mut entries = Vec::new();
            if let Some(points) = value.get("qpoints").and_then(|v| v.as_array()) {
                for p in points {
                    let index = p.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let Some(coords) = p.get("q_crystal").and_then(|v| v.as_array()) else {
                        continue;
                    };
                    let q_crystal: [f64; 3] = [
                        coords.first().and_then(|v| v.as_f64()).unwrap_or(0.0),
                        coords.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0),
                        coords.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    ];
                    let directory = p
                        .get("directory")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{index:02}"));
                    let q_cart = p.get("q_cart").and_then(|v|v.as_array()).filter(|v|v.len()==3).and_then(|v|Some([v[0].as_f64()?,v[1].as_f64()?,v[2].as_f64()?]));
                    entries.push(SplitQEntry {
                        directory,
                        index,
                        q_crystal,
                        q_cart,
                    });
                }
            }
            if !entries.is_empty() {
                return Ok(entries);
            }
        }
    }
    // 2) <prefix>.dyn0
    let dyn0_path = format!("{workspace}/{prefix}.dyn0");
    if Path::new(&dyn0_path).exists() {
        let raw = std::fs::read_to_string(&dyn0_path)?;
        let qs = qf_io::parse_dyn0_qpoints(&raw)
            .with_context(|| format!("解析 dyn0 失败: {dyn0_path}"))?;
        return Ok(qs
            .into_iter()
            .map(|q| SplitQEntry {
                directory: format!("{:02}", q.index),
                index: q.index,
                q_crystal: q.q_crystal,
                q_cart: Some(q.q_crystal),
            })
            .collect());
    }
    Ok(Vec::new())
}

/// 生成单个 q 点目录的 `run_split.sh`（对齐 `split_prepare._write_split_run_script`）。
fn write_split_run_script(
    q_dir: &str,
    slurm_header: &str,
    pw_command: &str,
    ph_command: &str,
    workflow_name: &str,
    prefix: &str,
    q_name: &str,
) -> Result<String> {
    let mut commands: Vec<String> = vec!["set -e".to_string()];
    for (step, command) in [("split_scf", pw_command), ("split_ph", ph_command)] {
        commands.push(qf_runner::echo_qeflow(&"-".repeat(60)));
        commands.push(qf_runner::echo_qeflow(&format!("workflow: {workflow_name}")));
        commands.push(qf_runner::echo_qeflow(&format!("prefix: {prefix}")));
        commands.push(qf_runner::echo_qeflow(&format!("q_dir: {q_name}")));
        commands.push(qf_runner::echo_qeflow(&format!("step: {step}")));
        commands.push(qf_runner::echo_qeflow(&format!("work_dir: {q_dir}")));
        commands.push(qf_runner::echo_qeflow(&format!("command: {command}")));
        commands.push(format!("if ! cd \"{q_dir}\"; then"));
        commands.push(qf_runner::echo_qeflow(&format!("failed: {step}")));
        commands.push("  exit 1".to_string());
        commands.push("fi".to_string());
        commands.push(format!("if ! {command}; then"));
        commands.push(qf_runner::echo_qeflow(&format!("failed: {step}")));
        commands.push("  exit 1".to_string());
        commands.push("fi".to_string());
        commands.push(qf_runner::echo_qeflow(&format!("done: {step}")));
    }
    let mut lines: Vec<String> = vec!["#!/bin/bash".to_string()];
    let header = slurm_header.trim();
    if !header.is_empty() {
        lines.extend(header.lines().map(|l| l.to_string()));
    }
    lines.extend(commands);
    let path = format!("{q_dir}/run_split.sh");
    std::fs::write(&path, format!("{}\n", lines.join("\n")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(path)
}

/// 生成 split 输入：为每个 q 点建立 `NN/` 目录，写出 `scf.in`、`split_ph.in`、
/// `run_split.sh`；`q_index_map.json` 仅在缺失时写出（避免覆盖规范文件）。
/// 返回生成的条目列表。
pub fn generate_split_inputs(
    workspace: &str,
    config: &Config,
    prefix: &str,
    mode: &str,
    structure: &Structure,
) -> Result<Vec<SplitQEntry>> {
    let mode_key = normalize_mode(mode);
    let qpoints = load_split_q_entries(workspace, prefix)?;
    let entries: Vec<SplitQEntry> = if qpoints.is_empty() {
        let coords = {
            // dyn0/q_points 均缺失：按 nq 网格全枚举并补建条目。
            let ph = &config.qe_phonon.ph;
            let (nq1, nq2, nq3) = qf_input::ph::resolve_grid(ph, "qpoints", ("nq1", "nq2", "nq3"));
            let (nq1, nq2, nq3) = (nq1.max(1), nq2.max(1), nq3.max(1));
            let mut v = Vec::new();
            for i in 0..nq1 {
                for j in 0..nq2 {
                    for k in 0..nq3 {
                        v.push([
                            i as f64 / nq1 as f64,
                            j as f64 / nq2 as f64,
                            k as f64 / nq3 as f64,
                        ]);
                    }
                }
            }
            if v.is_empty() {
                v.push([0.0, 0.0, 0.0]);
            }
            v
        };
        entries_from_coords(&coords)
    } else {
        qpoints
    };

    let slurm_header = config.slurm.header.clone().unwrap_or_default();
    let workflow_name = Path::new(workspace)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let exec = config.qe.executable_path();
    let pw_command = qf_runner::resolve_qe_command(exec, "pw.x");
    let ph_command = qf_runner::resolve_qe_command(exec, "ph.x");

    let mut map_entries = Vec::with_capacity(entries.len());
    for entry in &entries {
        let q_dir = format!("{workspace}/{}", entry.directory);
        std::fs::create_dir_all(&q_dir)?;
        qf_input::pw::write_phonon_scf_input(&q_dir, config, structure, prefix)?;
        if mode_key == "split_assign_coords" {
            let q_cart = if let Some(cart)=entry.q_cart {cart} else {
                let od=config.qe_phonon.scf.get("outdir").and_then(|v|v.as_str()).unwrap_or("./tmp");
                let info=crate::xml::parse_qe_save_xml(&Path::new(workspace).join(od).join(format!("{prefix}.save/data-file-schema.xml")))?;
                let mut cart=[0.;3]; for i in 0..3 {for j in 0..3 {cart[i]+=entry.q_crystal[j]*info.bg[j][i];}} cart
            };
            qf_input::ph::write_split_ph_input_assign_coords(&q_dir, config, prefix, q_cart)?;
        } else {
            qf_input::ph::write_split_ph_input_assign_index(&q_dir, config, prefix, entry.index as i64)?;
        }
        write_split_run_script(
            &q_dir,
            &slurm_header,
            &pw_command,
            &ph_command,
            &workflow_name,
            prefix,
            &entry.directory,
        )?;
        map_entries.push(QIndexEntry {
            directory: entry.directory.clone(),
            index: entry.index,
            coords: Some(entry.q_crystal),
            q_cart: entry.q_cart, q_crystal: Some(entry.q_crystal),
        });
    }
    if !has_q_index_map(workspace) {
        let map = QIndexMap {
            mode: mode_key.to_string(),
            qpoints: map_entries,
        };
        save_q_index_map(workspace, &map)?;
    }
    Ok(entries)
}

fn entries_from_coords(coords: &[[f64; 3]]) -> Vec<SplitQEntry> {
    coords
        .iter()
        .enumerate()
        .map(|(i, q)| SplitQEntry {
            directory: format!("{:02}", i + 1),
            index: i + 1,
            q_crystal: *q,
            q_cart: None,
        })
        .collect()
}
