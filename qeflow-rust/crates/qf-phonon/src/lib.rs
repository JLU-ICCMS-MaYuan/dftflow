//! 声子 split/merge 工作流编排（对齐 `qeflow/phonon/split_workflow` 与 `split_merge`，M3）。

/// 单个工作流步骤。
#[derive(Debug, Clone)]
pub struct Step {
    pub name: String,
    pub state: qf_state::TaskState,
}

/// 构建 split 工作流步骤序列。`mode` ∈ {index, coords}（M3）。
pub fn split_workflow(_mode: &str) -> anyhow::Result<Vec<Step>> {
    anyhow::bail!("split_workflow not yet implemented (M3)")
}

/// 合并 split 产物（对齐 `split_merge.py` 规则）（M3）。
pub fn merge(_work_dir: &str, _mode: &str) -> anyhow::Result<()> {
    anyhow::bail!("merge not yet implemented (M3)")
}
