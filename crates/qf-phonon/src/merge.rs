//! split 结果回收合并（对齐 `qeflow/phonon/split_merge.py`）。
//! 从各 `q_XXXX/` 子目录收集 `prefix.dyn*`、`tmp/_ph0/{prefix}.phsave` 与 q 点目录到 workspace，
//! 并要求各自的 `split_ph.out` 已完成（JOB DONE）。

use anyhow::Result;
use regex::Regex;
use std::path::Path;

use crate::index_map::QIndexMap;

/// merge 结果（对齐 `split_merge.MergeResult`）。
#[derive(Debug, Clone, Default)]
pub struct MergeResult {
    pub success: bool,
    pub errors: Vec<String>,
    pub dyn_summaries: Vec<(usize, Vec<f64>)>,
}

fn resolve_ph_root(workdir: &str) -> String {
    let tmp_ph0 = format!("{workdir}/tmp/_ph0");
    let tmp = format!("{workdir}/tmp");
    if Path::new(&tmp_ph0).is_dir() || Path::new(&tmp).is_dir() {
        tmp_ph0
    } else {
        format!("{workdir}/_ph0")
    }
}

fn copy_file(src: &str, dst: &str) -> bool {
    if !Path::new(src).exists() {
        return false;
    }
    if let Some(parent) = Path::new(dst).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::copy(src, dst).is_ok()
}

fn copy_tree(src: &str, dst: &str) -> bool {
    if !Path::new(src).is_dir() {
        return false;
    }
    if let Some(parent) = Path::new(dst).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let status = std::process::Command::new("cp")
        .args(["-r", src, dst])
        .status();
    matches!(status, Ok(s) if s.success())
}

fn extract_lowest_frequencies(dyn_path: &str, limit: usize) -> Vec<f64> {
    let text = std::fs::read_to_string(dyn_path).unwrap_or_default();
    let re = Regex::new(r"(?i)freq\s*\(\s*\d+\s*\)\s*=\s*([-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?)").unwrap();
    let mut values: Vec<f64> = re
        .captures_iter(&text)
        .filter_map(|c| c.get(1))
        .filter_map(|m| m.as_str().parse().ok())
        .collect();
    if values.is_empty() {
        let num_re = Regex::new(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?").unwrap();
        values = num_re
            .find_iter(&text)
            .filter_map(|m| m.as_str().parse().ok())
            .collect();
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values.truncate(limit);
    values
}

fn find_dyn_source(q_dir: &str, prefix: &str, q_index: usize, coords_mode: bool, errors: &mut Vec<String>) -> Option<String> {
    let candidates: Vec<String> = if coords_mode {
        vec![
            format!("{q_dir}/{prefix}.dyn"),
            format!("{q_dir}/{prefix}.dyn{q_index}"),
        ]
    } else {
        vec![format!("{q_dir}/{prefix}.dyn{q_index}"), format!("{q_dir}/{prefix}.dyn")]
    };
    let existing: Vec<&String> = candidates.iter().filter(|c| Path::new(c).exists()).collect();
    match existing.len() {
        0 => {
            errors.push(format!("{q_dir}: 缺少候选 dyn 源文件"));
            None
        }
        1 => Some(existing[0].clone()),
        _ => {
            errors.push(format!("{q_dir}: 存在多个候选 dyn 源文件 -> {}", existing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
            None
        }
    }
}

fn find_q_phdir_source(q_dir: &str, prefix: &str, q_index: usize, coords_mode: bool, errors: &mut Vec<String>) -> Option<String> {
    let q_ph_root = resolve_ph_root(q_dir);
    let exact = format!("{q_ph_root}/{prefix}.q_{q_index}");
    if Path::new(&exact).is_dir() {
        return Some(exact);
    }
    if coords_mode {
        return None;
    }
    if q_index == 1 {
        if Path::new(&format!("{q_ph_root}/{prefix}.dvscf1")).exists()
            || Path::new(&format!("{q_ph_root}/dvscf")).exists()
        {
            return Some(q_ph_root);
        }
        errors.push(format!("{q_dir} q_{q_index}: 缺少 tmp/_ph0/{prefix}.dvscf1、_ph0/dvscf 或 _ph0/{prefix}.q_1"));
        return None;
    }
    errors.push(format!("{q_dir} q_{q_index}: 缺少 tmp/_ph0/{prefix}.q_{q_index} 目录"));
    None
}

fn find_phsave_source(q_dir: &str, prefix: &str, coords_mode: bool) -> Option<String> {
    let q_ph_root = resolve_ph_root(q_dir);
    let phsave = format!("{q_ph_root}/{prefix}.phsave");
    if !Path::new(&phsave).is_dir() {
        return None;
    }
    let pattern_name = if coords_mode { "patterns.1.xml" } else { "patterns.1.xml" };
    if !Path::new(&format!("{phsave}/{pattern_name}")).exists() {
        return None;
    }
    Some(phsave)
}

/// 合并 split 结果（对齐 `merge_split_results`）。`map` 为已加载的 `q_index_map.json`。
pub fn merge_split_results(workspace: &str, prefix: &str, map: &QIndexMap) -> Result<MergeResult> {
    let coords_mode = map.mode.contains("coords");
    let merged_ph_root = resolve_ph_root(workspace);
    if map.qpoints.is_empty() {
        return Ok(MergeResult {
            errors: vec!["缺少 q_index_map.json 或其中不包含 qpoints".into()],
            ..Default::default()
        });
    }
    let mut errors = Vec::new();
    let mut summaries = Vec::new();
    for entry in &map.qpoints {
        let q_index = entry.index;
        let q_dir = format!("{workspace}/{}", entry.directory);
        let split_out = format!("{q_dir}/split_ph.out");
        let ph_out = qf_io::read_ph_out(&split_out)?;
        if !ph_out.finished {
            errors.push(format!("{}: split_ph.out 未完成，禁止 merge", entry.directory));
            continue;
        }
        let split_in = format!("{q_dir}/split_ph.in");
        if Path::new(&split_in).exists() {
            copy_file(&split_in, &format!("{workspace}/split_ph.in.{q_index}"));
        } else {
            errors.push(format!("{}: 缺少 split_ph.in", entry.directory));
        }
        copy_file(&split_out, &format!("{workspace}/split_ph.out.{q_index}"));

        if let Some(dyn_source) = find_dyn_source(&q_dir, prefix, q_index, coords_mode, &mut errors) {
            let dyn_target = format!("{workspace}/{prefix}.dyn{q_index}");
            if copy_file(&dyn_source, &dyn_target) {
                summaries.push((q_index, extract_lowest_frequencies(&dyn_target, 5)));
            }
        }
        if let Some(q_phdir_source) = find_q_phdir_source(&q_dir, prefix, q_index, coords_mode, &mut errors) {
            if coords_mode {
                for filename in [format!("{prefix}.dvscf1"), "dvscf".to_string()] {
                    let src = format!("{q_phdir_source}/{filename}");
                    if Path::new(&src).exists() {
                        copy_file(&src, &format!("{merged_ph_root}/{filename}"));
                    }
                }
            } else {
                let dst = format!("{merged_ph_root}/{prefix}.q_{q_index}");
                copy_tree(&q_phdir_source, &dst);
            }
        }
        if let Some(phsave_source) = find_phsave_source(&q_dir, prefix, coords_mode) {
            let dst = format!("{merged_ph_root}/{prefix}.phsave");
            copy_tree(&phsave_source, &dst);
        }
    }
    let success = errors.is_empty();
    Ok(MergeResult {
        success,
        errors,
        dyn_summaries: summaries,
    })
}
