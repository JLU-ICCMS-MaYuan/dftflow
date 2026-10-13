//! 任务状态机与续算（复刻 FireWorks 状态机 + 现状三态 + `split_resume_step`）。
//!
//! 状态落盘为 `workflow_status.json` / `task_status.json`，替代 FireWorks 的
//! MongoDB / AiiDA 的 PostgreSQL，契合"弱机器 + 单静态二进制"约束。
//!
//! 序列化键**完全对齐** Python 版 `workflow_status.json`，保证 Rust 写出的状态文件
//! 与现状可互换（可恢复语义契约兼容）：
//! ```json
//! {
//!   "scf": "finished",
//!   "ph": { "state": "finished", "dyn0": "finished", "tasks": { "01": "finished" } },
//!   "merge": "finished",
//!   "q2r": "finished",
//!   "matdyn_phband": "finished",
//!   "matdyn_phdos": "finished",
//!   "phonon_plot": "finished"
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const WORKFLOW_STATUS_FILE: &str = "workflow_status.json";
pub const TASK_STATUS_FILE: &str = "task_status.json";

/// 三态：未开始 / 已完成 / 错误。序列化字符串对齐现状 `qe_check` 返回值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    #[default]
    Unstarted,
    Finished,
    Error,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskState::Unstarted => "unstarted",
            TaskState::Finished => "finished",
            TaskState::Error => "error",
        }
    }
}

/// 声子 `ph` 子状态嵌套结构，对齐 `workflow_status.json` 的 `ph` 字段。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhStatus {
    #[serde(default)]
    pub state: TaskState,
    #[serde(default)]
    pub dyn0: TaskState,
    #[serde(default)]
    pub tasks: HashMap<String, TaskState>,
}

/// 声子工作流整体状态，字段与序列化键对齐 Python 版 `workflow_status.json`。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowStatus {
    #[serde(default)]
    pub scf: TaskState,
    #[serde(default)]
    pub ph: PhStatus,
    #[serde(default)]
    pub merge: TaskState,
    #[serde(default)]
    pub q2r: TaskState,
    #[serde(default)]
    pub matdyn_phband: TaskState,
    #[serde(default)]
    pub matdyn_phdos: TaskState,
    #[serde(default)]
    pub phonon_plot: TaskState,
}

impl WorkflowStatus {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text).with_context(|| format!("write {}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// 文件系统后端：基于 QE 输出文件探测任务状态（对齐 `qeflow/check/checks.py`）
// ---------------------------------------------------------------------------

/// 对齐 `qeflow/io_reader/common.py::ERROR_MARKERS`。
const ERROR_MARKERS: &[&str] = &[
    "%%%%%%%%%%%%%%",
    "error in routine",
    "error:",
    "stopping ...",
    "convergence not achieved",
];

fn has_error_marker(text: &str) -> bool {
    let lowered = text.to_lowercase();
    ERROR_MARKERS
        .iter()
        .any(|m| lowered.contains(&m.to_lowercase()))
}

struct Sniff {
    exists: bool,
    finished: bool,
    error: bool,
}

/// 读取输出文件，判定是否存在 / 完成（含 "JOB DONE"）/ 出错（含错误标记）。
fn sniff(path: &Path) -> Sniff {
    let exists = path.exists();
    let text = if exists {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };
    let finished = exists && text.contains("JOB DONE");
    let error = exists && has_error_marker(&text);
    Sniff {
        exists,
        finished,
        error,
    }
}

fn finalize(exists: bool, finished: bool, errored: bool) -> TaskState {
    if !exists {
        TaskState::Unstarted
    } else if finished {
        TaskState::Finished
    } else if errored {
        TaskState::Error
    } else {
        TaskState::Error
    }
}

fn check_scf(workspace: &Path) -> TaskState {
    let s = sniff(&workspace.join("scf.out"));
    finalize(s.exists, s.finished, s.error)
}

fn check_split_task(q_dir: &Path) -> TaskState {
    let scf = sniff(&q_dir.join("scf.out"));
    let ph = sniff(&q_dir.join("split_ph.out"));
    let has_any = scf.exists || ph.exists;
    if ph.finished && scf.finished {
        return TaskState::Finished;
    }
    if ph.error || scf.error {
        return TaskState::Error;
    }
    if !has_any {
        return TaskState::Unstarted;
    }
    TaskState::Error
}

// ---------------------------------------------------------------------------
// q_index_map.json 读取与 split 目录发现（对齐 `qeflow/phonon/q_index_map.py`）
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct QIndexPayload {
    #[serde(default)]
    qpoints: Vec<QIndexEntry>,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct QIndexEntry {
    #[serde(default)]
    directory: String,
    #[serde(default)]
    index: Option<usize>,
}

fn load_q_index(workspace: &Path) -> (Vec<QIndexEntry>, Option<String>) {
    let path = workspace.join("q_index_map.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return (Vec::new(), None),
    };
    match serde_json::from_str::<QIndexPayload>(&text) {
        Ok(p) => (p.qpoints, p.mode),
        Err(_) => (Vec::new(), None),
    }
}

fn list_split_directories(workspace: &Path) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(workspace) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Some(name) = p.file_name() {
                    let name = name.to_string_lossy().into_owned();
                    // 对齐 Python `list_split_directories`：仅两位数字目录。
                    if name.len() == 2 && name.chars().all(|c| c.is_ascii_digit()) {
                        v.push(name);
                    }
                }
            }
        }
    }
    v.sort();
    v
}

fn collect_split_task_states(workspace: &Path) -> HashMap<String, TaskState> {
    let (entries, _) = load_q_index(workspace);
    let mut q_names: Vec<String> = entries
        .iter()
        .map(|e| e.directory.clone())
        .filter(|s| !s.is_empty())
        .collect();
    if q_names.is_empty() {
        q_names = list_split_directories(workspace);
    }
    q_names
        .into_iter()
        .map(|q| {
            let state = check_split_task(&workspace.join(&q));
            (q, state)
        })
        .collect()
}

/// 刷新每个 q 点目录的 `task_status.json`，返回 q 目录名 -> 状态。
pub fn refresh_split_task_status(workspace: &Path) -> Result<HashMap<String, TaskState>> {
    let states = collect_split_task_states(workspace);
    for (q_name, state) in &states {
        let q_dir = workspace.join(q_name);
        if !q_dir.is_dir() {
            continue;
        }
        let payload = serde_json::json!({
            "directory": q_name,
            "state": state.as_str(),
        });
        let path = q_dir.join(TASK_STATUS_FILE);
        std::fs::write(&path, serde_json::to_string_pretty(&payload)?)
            .with_context(|| format!("write {}", path.display()))?;
    }
    Ok(states)
}

fn check_ph(workspace: &Path, prefix: &str, tasks: &HashMap<String, TaskState>) -> PhStatus {
    let dyn0_path = workspace.join(format!("{prefix}.dyn0"));
    let dyn0_ready = dyn0_path.exists()
        && std::fs::metadata(&dyn0_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false);
    let dyn0 = if dyn0_ready {
        TaskState::Finished
    } else {
        TaskState::Unstarted
    };

    let state = if tasks.is_empty() {
        // nosplit 模式：ph.out 决定状态
        let ph = sniff(&workspace.join("ph.out"));
        if dyn0_ready && ph.finished {
            TaskState::Finished
        } else if ph.error {
            TaskState::Error
        } else if ph.exists {
            TaskState::Error
        } else {
            TaskState::Unstarted
        }
    } else {
        summarize_task_states(&tasks.values().copied().collect::<Vec<_>>())
    };
    PhStatus {
        state,
        dyn0,
        tasks: tasks.clone(),
    }
}

fn merge_ph_root(workspace: &Path) -> std::path::PathBuf {
    let tmp = workspace.join("tmp");
    let tmp_ph0 = tmp.join("_ph0");
    if tmp_ph0.is_dir() || tmp.is_dir() {
        tmp_ph0
    } else {
        workspace.join("_ph0")
    }
}

fn check_merge(
    workspace: &Path,
    prefix: &str,
    entries: &[QIndexEntry],
    mode: &Option<String>,
) -> TaskState {
    if entries.is_empty() {
        return TaskState::Unstarted;
    }
    let mode = match mode {
        Some(m) if m == "split_assign_coords" || m == "split_assign_index" => m.clone(),
        _ => return TaskState::Error,
    };
    // 所有 split 任务必须 finished
    let mut task_states = Vec::new();
    for e in entries {
        if e.directory.is_empty() {
            continue;
        }
        task_states.push(check_split_task(&workspace.join(&e.directory)));
    }
    if summarize_task_states(&task_states) != TaskState::Finished {
        return TaskState::Unstarted;
    }
    let phsave = merge_ph_root(workspace).join(format!("{prefix}.phsave"));
    if !phsave.is_dir() {
        return TaskState::Unstarted;
    }
    let patterns_ready = match mode.as_str() {
        "split_assign_coords" => phsave.join("patterns.1.xml").exists(),
        "split_assign_index" => entries.iter().all(|e| match e.index {
            Some(i) => phsave.join(format!("patterns.{i}.xml")).exists(),
            None => false,
        }),
        _ => false,
    };
    if !patterns_ready {
        return TaskState::Unstarted;
    }
    if mode == "split_assign_index" {
        let mut missing = false;
        for e in entries {
            match e.index {
                Some(i) => {
                    if !workspace.join(format!("{prefix}.dyn{i}")).exists()
                        || !workspace.join(format!("split_ph.in.{i}")).exists()
                        || !workspace.join(format!("split_ph.out.{i}")).exists()
                    {
                        missing = true;
                    }
                }
                None => missing = true,
            }
        }
        if missing {
            return TaskState::Error;
        }
    }
    TaskState::Finished
}

fn check_q2r(workspace: &Path, prefix: &str) -> TaskState {
    let out = sniff(&workspace.join("q2r.out"));
    let fc = workspace.join(format!("{prefix}.fc"));
    if !out.exists {
        TaskState::Unstarted
    } else if out.finished && fc.exists() {
        TaskState::Finished
    } else {
        TaskState::Error
    }
}

fn check_matdyn(workspace: &Path, output_name: &str, product_name: &str, archived: &[String]) -> TaskState {
    let out = sniff(&workspace.join(output_name));
    let product = workspace.join(product_name).exists();
    let archive_ready = archived.iter().all(|p| workspace.join(p).exists());
    if !out.exists {
        TaskState::Unstarted
    } else if out.finished && product && archive_ready {
        TaskState::Finished
    } else {
        TaskState::Error
    }
}

fn check_phonon_plot(
    workspace: &Path,
    matdyn_phband: TaskState,
    matdyn_phdos: TaskState,
) -> TaskState {
    let required = [
        "phband/phonon_band.csv",
        "phband/phonon_band_labels.csv",
        "phband/phonon_band.png",
        "phdos/phonon_dos_atoms.csv",
        "phdos/phonon_dos_elements.csv",
        "phdos/phonon_dos.png",
    ];
    if matdyn_phband != TaskState::Finished || matdyn_phdos != TaskState::Finished {
        return TaskState::Unstarted;
    }
    let existing = required.iter().filter(|p| workspace.join(p).exists()).count();
    if existing == 0 {
        TaskState::Unstarted
    } else if existing == required.len() {
        TaskState::Finished
    } else {
        TaskState::Error
    }
}

/// 基于文件系统输出文件刷新整体工作流状态并落盘 `workflow_status.json`。
/// 对齐 `qeflow/phonon/status_machine.refresh_workflow_status`。
pub fn refresh_workflow_status(workspace: &Path, prefix: &str) -> Result<WorkflowStatus> {
    let tasks = refresh_split_task_status(workspace)?;
    let (entries, mode) = load_q_index(workspace);

    let scf = check_scf(workspace);
    let ph = check_ph(workspace, prefix, &tasks);
    let merge = check_merge(workspace, prefix, &entries, &mode);
    let q2r = check_q2r(workspace, prefix);
    let matdyn_phband = check_matdyn(
        workspace,
        "matdyn_phband.out",
        &format!("{prefix}.phband.freq"),
        &[
            format!("phband/{prefix}.phband.freq"),
            format!("phband/{prefix}_band.labelinfo.dat"),
        ],
    );
    let matdyn_phdos = check_matdyn(
        workspace,
        "matdyn_phdos.out",
        &format!("{prefix}.phdos.dat"),
        &[format!("phdos/{prefix}.phdos.dat")],
    );
    let phonon_plot = check_phonon_plot(workspace, matdyn_phband, matdyn_phdos);

    let status = WorkflowStatus {
        scf,
        ph,
        merge,
        q2r,
        matdyn_phband,
        matdyn_phdos,
        phonon_plot,
    };
    status.save(&workspace.join(WORKFLOW_STATUS_FILE))?;
    Ok(status)
}

// ---------------------------------------------------------------------------
// 续算：阶段判定（对齐 `qeflow/phonon/status_machine.split_resume_step`）
// ---------------------------------------------------------------------------

pub fn summarize_task_states(states: &[TaskState]) -> TaskState {
    let set: std::collections::HashSet<_> = states.iter().copied().collect();
    if set.is_empty() {
        TaskState::Unstarted
    } else if set.len() == 1 && set.contains(&TaskState::Finished) {
        TaskState::Finished
    } else if set.contains(&TaskState::Error) {
        TaskState::Error
    } else {
        TaskState::Unstarted
    }
}

pub fn all_split_finished(status: &WorkflowStatus) -> bool {
    if status.ph.tasks.is_empty() {
        return false;
    }
    status.ph.tasks.values().all(|&s| s == TaskState::Finished)
}

pub fn split_needs_run(status: &WorkflowStatus) -> bool {
    if status.ph.tasks.is_empty() {
        return false;
    }
    summarize_task_states(&status.ph.tasks.values().copied().collect::<Vec<_>>())
        != TaskState::Finished
}

/// 返回下一个待执行阶段 `(序号, 标签)`，对齐现状 `split_resume_step` 的 1–11 阶段模型。
/// 注意：现状实现跳过 7/9（"collect" 折叠进 matdyn 步骤），这里 1:1 对齐。
pub fn split_resume_step(status: &WorkflowStatus) -> (u8, &'static str) {
    if status.scf != TaskState::Finished {
        return (1, "运行 SCF");
    }
    if status.ph.dyn0 != TaskState::Finished {
        return (2, "运行 dyn0");
    }
    if !all_split_finished(status) {
        return (3, "提交 split q 点任务");
    }
    if status.merge != TaskState::Finished {
        return (4, "合并 split q 点结果");
    }
    if status.q2r != TaskState::Finished {
        return (5, "运行 q2r");
    }
    if status.matdyn_phband != TaskState::Finished {
        return (6, "运行 matdyn 声子谱");
    }
    if status.matdyn_phdos != TaskState::Finished {
        return (8, "运行 matdyn 声子 DOS");
    }
    if status.phonon_plot != TaskState::Finished {
        return (10, "绘图并写出 CSV");
    }
    (11, "完成声子回收")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_starts_at_one() {
        let s = WorkflowStatus::default();
        assert_eq!(split_resume_step(&s).0, 1);
    }

    #[test]
    fn resume_skips_finished() {
        let mut s = WorkflowStatus::default();
        s.scf = TaskState::Finished;
        s.ph.dyn0 = TaskState::Finished;
        s.ph.tasks.insert("01".into(), TaskState::Finished);
        s.ph.tasks.insert("02".into(), TaskState::Finished);
        s.merge = TaskState::Finished;
        assert_eq!(split_resume_step(&s).0, 5); // next is q2r
    }

    #[test]
    fn state_serialization_lowercase() {
        assert_eq!(
            serde_json::to_string(&TaskState::Finished).unwrap(),
            "\"finished\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Unstarted).unwrap(),
            "\"unstarted\""
        );
    }

    #[test]
    fn workflow_status_json_contract() {
        let s = WorkflowStatus {
            scf: TaskState::Finished,
            ph: PhStatus {
                state: TaskState::Finished,
                dyn0: TaskState::Finished,
                tasks: HashMap::from([("01".to_string(), TaskState::Finished)]),
            },
            merge: TaskState::Finished,
            q2r: TaskState::Finished,
            matdyn_phband: TaskState::Finished,
            matdyn_phdos: TaskState::Finished,
            phonon_plot: TaskState::Finished,
        };
        let text = serde_json::to_string_pretty(&s).unwrap();
        assert!(text.contains("\"scf\": \"finished\""));
        assert!(text.contains("\"matdyn_phband\""));
        assert!(text.contains("\"tasks\""));
    }

    #[test]
    fn summarize_rules() {
        assert_eq!(summarize_task_states(&[]), TaskState::Unstarted);
        assert_eq!(
            summarize_task_states(&[TaskState::Finished, TaskState::Finished]),
            TaskState::Finished
        );
        assert_eq!(
            summarize_task_states(&[TaskState::Finished, TaskState::Unstarted]),
            TaskState::Unstarted
        );
        assert_eq!(
            summarize_task_states(&[TaskState::Finished, TaskState::Error]),
            TaskState::Error
        );
    }
}
