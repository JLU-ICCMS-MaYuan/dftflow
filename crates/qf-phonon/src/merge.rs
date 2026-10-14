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
    fn copy(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry=entry?;
            if entry.file_type()?.is_dir() { copy(&entry.path(),&dst.join(entry.file_name()))?; }
            else { std::fs::copy(entry.path(),dst.join(entry.file_name()))?; }
        }
        Ok(())
    }
    copy(Path::new(src),Path::new(dst)).is_ok()
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

fn find_phsave_source(q_dir: &str, prefix: &str, coords_mode: bool, q_index: usize) -> Option<String> {
    let q_ph_root = resolve_ph_root(q_dir);
    let phsave = format!("{q_ph_root}/{prefix}.phsave");
    if !Path::new(&phsave).is_dir() {
        return None;
    }
    let pattern_name = format!("patterns.{}.xml", if coords_mode {1} else {q_index});
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
        if let Some(phsave_source) = find_phsave_source(&q_dir, prefix, coords_mode, q_index) {
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

/// Issue 0006 的严格合并：依据配置 outdir，先验证再归集。
pub fn merge_split_results_with_outdir(workspace: &str, prefix: &str, map: &QIndexMap, outdir: &str) -> Result<MergeResult> {
    use anyhow::{ensure, Context};
    ensure!(!map.qpoints.is_empty(), "q 映射为空");
    let coords = map.mode == "split_assign_coords";
    let mut prepared = Vec::new();
    let q_re=Regex::new(r"q\s*=\s*\(\s*([^)]*)\)")?;
    for entry in &map.qpoints {
        let dir=Path::new(workspace).join(&entry.directory);
        let log=std::fs::read_to_string(dir.join("split_ph.out"))?;
        ensure!(log.contains("JOB DONE") && !log.to_lowercase().contains("error in routine"),"q {} 未正常完成",entry.index);
        let mut errors=Vec::new();
        let dyn_src=find_dyn_source(dir.to_str().unwrap(),prefix,entry.index,coords,&mut errors).context("缺少或歧义 dyn 文件")?;
        if let Some(expected)=entry.q_cart {
            let dyn_text=std::fs::read_to_string(&dyn_src)?;
            let cap=q_re.captures(&dyn_text).context("dyn 缺少 q 坐标")?;
            let got:Vec<f64>=cap[1].split_whitespace().map(str::parse).collect::<std::result::Result<_,_>>()?;
            ensure!(got.len()==3 && got.iter().zip(expected).all(|(a,b)|(a-b).abs()<1e-7),"q {} 动力学矩阵坐标不匹配",entry.index);
        }
        let phroot=dir.join(outdir).join("_ph0");
        let phsave=phroot.join(format!("{prefix}.phsave"));
        let local_index=if coords {1} else {entry.index};
        ensure!(phsave.join(format!("patterns.{local_index}.xml")).is_file(),"q {} 缺少 patterns",entry.index);
        prepared.push((entry,dir,dyn_src,phroot,phsave));
    }
    let dest_root=Path::new(workspace).join(outdir).join("_ph0");
    let dest_save=dest_root.join(format!("{prefix}.phsave"));
    std::fs::create_dir_all(&dest_save)?;
    let mut summaries=Vec::new();
    for (entry,dir,dyn_src,phroot,phsave) in prepared {
        let dyn_dst=Path::new(workspace).join(format!("{prefix}.dyn{}",entry.index));
        std::fs::copy(&dyn_src,&dyn_dst)?;
        summaries.push((entry.index,extract_lowest_frequencies(dyn_dst.to_str().unwrap(),5)));
        for name in ["split_ph.in","split_ph.out"] {std::fs::copy(dir.join(name),Path::new(workspace).join(format!("{name}.{}",entry.index)))?;}
        let qroot=phroot.join(format!("{prefix}.q_{}",entry.index));
        let src=if qroot.is_dir() {qroot} else {phroot.clone()};
        let dst=dest_root.join(format!("{prefix}.q_{}",entry.index));
        std::fs::create_dir_all(&dst)?;
        // 保留各 q 独立潜势；不把整个 _ph0 再复制到自身子目录。
        for e in std::fs::read_dir(&src)? {
            let e=e?;if e.file_type()?.is_file() && (e.file_name().to_string_lossy().contains("dvscf")) {std::fs::copy(e.path(),dst.join(e.file_name()))?;}
        }
        let local=if coords {1} else {entry.index};
        std::fs::copy(phsave.join(format!("patterns.{local}.xml")),dest_save.join(format!("patterns.{}.xml",entry.index)))?;
    }
    Ok(MergeResult {success:true,errors:Vec::new(),dyn_summaries:summaries})
}
