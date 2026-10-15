use anyhow::{ensure, Context, Result};
use regex::Regex;
use std::path::Path;
use super::manifest::{Artifact, PhResponse, QPoint};
use super::super::foundation::store;
use std::collections::BTreeMap;

/// 已归集的 PH 响应集：验证通过的响应、总 q 数与产物清单。
#[derive(Debug, Clone)]
pub struct CollectedPh {
    pub responses: Vec<PhResponse>,
    pub total_q: usize,
    pub artifacts: Vec<Artifact>,
}

/// 从 dyn 文件解析声子模数量（omega(n) 行），同时校验非空与有限。
pub fn parse_dyn_mode_count(path: &Path) -> Result<usize> {
    ensure!(path.exists(), "data_invalid: dyn 文件缺失 {}", path.display());
    let len = std::fs::metadata(path)
        .with_context(|| format!("data_invalid: 无法读取 {}", path.display()))?
        .len();
    ensure!(len > 0, "data_invalid: dyn 文件为空 {}", path.display());
    let txt = std::fs::read_to_string(path)
        .with_context(|| format!("data_invalid: 无法读取 {}", path.display()))?;
    let re = Regex::new(r"omega\(\s*(\d+)\s*\)").expect("collect_ph: 正则编译失败");
    let n = re.captures_iter(&txt).count();
    ensure!(n > 0, "data_invalid: dyn 未解析到任何 omega 模行 {}", path.display());
    Ok(n)
}

fn copy_file(src: &Path, dst: &Path) -> Result<u64> {
    ensure!(src.exists(), "data_invalid: 源文件缺失 {}", src.display());
    if let Some(p) = dst.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::copy(src, dst).with_context(|| format!("data_invalid: 复制 {} 失败", src.display()))
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let target = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &target)?;
        } else {
            std::fs::copy(&e.path(), &target)?;
        }
    }
    Ok(())
}

/// 根据各阶段实际 outdir 归集响应数据，校验完整性并构建 EPW 消费布局（FR-007）。
/// 每个响应势必须与实际产生它的扰动模式配对；缺失/空/截断文件不得放行。
pub fn collect(work: &Path, qpoints: &[QPoint], responses: &[PhResponse]) -> Result<CollectedPh> {
    ensure!(!responses.is_empty(), "data_invalid: 无 PH 响应可归集");
    ensure!(
        responses.len() == qpoints.len(),
        "data_invalid: 响应数 {} 与 q 点数 {} 不一致",
        responses.len(),
        qpoints.len()
    );
    let epw_dir = work.join("qe-data");
    std::fs::create_dir_all(&epw_dir)?;
    let mut artifacts = Vec::new();
    let mut out_responses = Vec::new();
    for r in responses {
        // 校验每个响应与物理 q 配对且全局编号有效。
        ensure!(
            r.q_id >= 1 && (r.q_id as usize) <= qpoints.len(),
            "data_invalid: 响应 q_id {} 超出 q 点范围",
            r.q_id
        );
        let mut out = r.clone();
        let roles: Vec<(&str, std::path::PathBuf)> = vec![
            ("dyn", r.dyn_file.clone()),
            ("patterns", r.patterns.clone()),
            ("control_metadata", r.control_metadata.clone()),
        ];
        for (role, p) in &roles {
            ensure!(
                p.exists() && std::fs::metadata(p)?.len() > 0,
                "data_invalid: 响应角色 {role} 缺失或为空: {}",
                p.display()
            );
            let size = std::fs::metadata(p)?.len();
            let sha = store::hash_file(p)?;
            let dest = epw_dir.join(format!("{role}_q{}.bin", r.q_id));
            copy_file(p, &dest)?;
            artifacts.push(Artifact {
                role: format!("{role}_q{}", r.q_id),
                relative_path: dest.strip_prefix(work).unwrap_or(&dest).to_path_buf(),
                byte_size: size,
                sha256: sha,
                producer_stage: "phonon".to_string(),
                input_digest: String::new(),
                format_version: "1".to_string(),
            });
        }
        if let Some(dv) = &r.dvscf {
            ensure!(
                dv.exists() && std::fs::metadata(dv)?.len() > 0,
                "data_invalid: dvscf 缺失或为空: {}",
                dv.display()
            );
            let size = std::fs::metadata(dv)?.len();
            let sha = store::hash_file(dv)?;
            let dest = epw_dir.join(format!("dvscf_q{}.bin", r.q_id));
            copy_file(dv, &dest)?;
            artifacts.push(Artifact {
                role: format!("dvscf_q{}", r.q_id),
                relative_path: dest.strip_prefix(work).unwrap_or(&dest).to_path_buf(),
                byte_size: size,
                sha256: sha,
                producer_stage: "phonon".to_string(),
                input_digest: String::new(),
                format_version: "1".to_string(),
            });
        }
        let modes = parse_dyn_mode_count(&r.dyn_file)?;
        if out.perturbations == 0 {
            out.perturbations = modes;
        } else {
            ensure!(
                out.perturbations == modes,
                "data_invalid: q={} 扰动数 {} 与 dyn 模数 {} 不一致",
                r.q_id,
                out.perturbations,
                modes
            );
        }
        // 共享 phsave 目录：逐 q 的 patterns.N 汇聚到统一布局。
        let phsave_src = r.patterns.parent().context("data_invalid: patterns 缺父目录")?;
        copy_dir(phsave_src, &epw_dir.join("phsave"))?;
        out_responses.push(out);
    }
    Ok(CollectedPh {
        responses: out_responses,
        total_q: responses.len(),
        artifacts,
    })
}

/// 把 q 点映射落盘为可定位 JSON（FR-008 的 q_index_map.json）。
pub fn write_q_index_map(work: &Path, qpoints: &[QPoint]) -> Result<()> {
    let path = work.join("qe-data").join("q_index_map.json");
    let map: BTreeMap<usize, serde_json::Value> = qpoints
        .iter()
        .map(|q| {
            (
                q.global_index,
                serde_json::json!({
                    "local_index": q.local_index,
                    "directory": q.directory,
                    "crystal": q.crystal,
                    "cartesian": q.cartesian,
                }),
            )
        })
        .collect();
    store::write(&path, &map)?;
    Ok(())
}
