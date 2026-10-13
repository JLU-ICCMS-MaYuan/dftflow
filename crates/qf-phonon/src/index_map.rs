//! split 工作流的 q 点索引映射（对齐 `qeflow/phonon/q_index_map.py`）。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 单个 q 点条目（对齐 `q_index_map.json` 的 qpoints 项）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QIndexEntry {
    pub directory: String,
    pub index: usize,
    #[serde(default)]
    pub coords: Option<[f64; 3]>,
}

/// `q_index_map.json` 完整结构。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QIndexMap {
    pub mode: String,
    pub qpoints: Vec<QIndexEntry>,
}

/// 读取 `q_index_map.json`（缺失或损坏返回默认空结构）。
pub fn load_q_index_map(workspace: &str) -> QIndexMap {
    let p = format!("{workspace}/q_index_map.json");
    let txt = std::fs::read_to_string(&p).unwrap_or_default();
    serde_json::from_str(&txt).unwrap_or_default()
}

/// 写出 `q_index_map.json`（美化 JSON）。
pub fn save_q_index_map(workspace: &str, map: &QIndexMap) -> Result<()> {
    let p = format!("{workspace}/q_index_map.json");
    std::fs::write(&p, serde_json::to_string_pretty(map)?)?;
    Ok(())
}

/// 判断 workspace 内是否存在可用的 `q_index_map.json`（含 qpoints）。
pub fn has_q_index_map(workspace: &str) -> bool {
    let map = load_q_index_map(workspace);
    !map.qpoints.is_empty() && !map.mode.is_empty()
}

/// 解析模式字符串为内部 mode 键（coords / index）。
pub fn normalize_mode(mode: &str) -> &'static str {
    if mode.contains("coords") {
        "split_assign_coords"
    } else {
        "split_assign_index"
    }
}

#[allow(dead_code)]
fn _workspace_exists(workspace: &str) -> bool {
    Path::new(workspace).exists()
}
