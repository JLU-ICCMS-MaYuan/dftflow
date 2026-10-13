//! POTCAR 拼接（复刻 `vaspflow` setup 中按结构元素顺序拼接多个元素 POTCAR 的行为）。
//!
//! 元素文件约定：依次尝试 `{potcar_dir}/{Element}`、`{potcar_dir}/POTCAR_{Element}`、
//! `{potcar_dir}/{Element}_POTCAR`（对齐 Python `os.path.join(potcar_dir, el)`）。

use anyhow::{Context, Result};
use std::path::Path;

/// 按结构中的**去重且保序**元素列表拼接 POTCAR 到 `out_path`。
/// `elements` 为有序元素符号列表（如 `["Si", "O"]`），每个元素只拼一次。
pub fn concat_potcar(potcar_dir: &str, elements: &[String], out_path: &str) -> Result<String> {
    let dir = Path::new(potcar_dir);
    if !dir.is_dir() {
        anyhow::bail!("potcar_dir 不存在或不是目录: {potcar_dir}");
    }
    let mut content = String::new();
    for element in elements {
        let candidates = [
            dir.join(element),
            dir.join(format!("POTCAR_{element}")),
            dir.join(format!("{element}_POTCAR")),
        ];
        let found = candidates
            .iter()
            .find(|p| p.is_file())
            .ok_or_else(|| anyhow::anyhow!("未找到元素 {element} 的 POTCAR（potcar_dir={potcar_dir}）"))?;
        let text = std::fs::read_to_string(found)
            .with_context(|| format!("read {}", found.display()))?;
        content.push_str(&text);
        if !content.ends_with('\n') {
            content.push('\n');
        }
    }
    std::fs::write(out_path, content).with_context(|| format!("write POTCAR {out_path}"))?;
    Ok(out_path.to_string())
}

/// 从结构提取去重且保序的元素列表。
pub fn distinct_elements(structure: &qf_crystal::Structure) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut order = Vec::new();
    for (sym, count) in structure.elements.iter().zip(structure.counts.iter()) {
        if seen.insert(sym.clone()) {
            order.push(sym.clone());
        }
        let _ = count;
    }
    order
}
