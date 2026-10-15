use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use super::store;

/// 阶段状态：未准备/就绪/提交未决/已提交/运行/待验证/成功/失败/未知/阻断。
/// 成功须同时满足执行结束、数值校验与必需产物有效；JOB DONE 或零退出码不能单独作为数值成功。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Unprepared,
    Ready,
    SubmissionPending,
    Submitted,
    Running,
    Validating,
    Succeeded,
    Failed,
    Unknown,
    Blocked,
}

impl StageState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StageState::Succeeded | StageState::Failed | StageState::Unknown | StageState::Blocked
        )
    }
}

/// 阶段及尝试：阶段身份、依赖、输入指纹、尝试身份、资源、作业身份、状态、产物与失败原因。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageAttempt {
    pub schema_version: u32,
    pub stage_id: String,
    pub attempt_id: String,
    pub dependencies: Vec<String>,
    pub input_digest: String,
    pub state: StageState,
    pub job_id: Option<u64>,
    pub cluster: Option<String>,
    pub validator: Option<String>,
    pub artifacts: Vec<PathBuf>,
    pub error: Option<String>,
}

impl StageAttempt {
    pub fn new(stage_id: &str, dependencies: Vec<String>, input_digest: &str) -> Self {
        StageAttempt {
            schema_version: 1,
            stage_id: stage_id.to_string(),
            attempt_id: store::token(),
            dependencies,
            input_digest: input_digest.to_string(),
            state: StageState::Ready,
            job_id: None,
            cluster: None,
            validator: None,
            artifacts: vec![],
            error: None,
        }
    }
    pub fn stage_dir(work: &Path) -> PathBuf {
        work.join(".epwflow").join("stages")
    }
    pub fn path(work: &Path, stage_id: &str) -> PathBuf {
        Self::stage_dir(work).join(format!("{stage_id}.json"))
    }
    pub fn load(work: &Path, stage_id: &str) -> Result<Self> {
        let p = Self::path(work, stage_id);
        ensure!(p.exists(), "data_invalid: 阶段 {stage_id} 尚无记录");
        let a: Self = store::read(&p)?;
        ensure!(a.schema_version == 1, "unsupported_version: 阶段记录 schema_version 必须为 1");
        Ok(a)
    }
    pub fn save(&self, work: &Path) -> Result<()> {
        let dir = Self::stage_dir(work);
        fs::create_dir_all(&dir)?;
        store::write(&Self::path(work, &self.stage_id), self)?;
        Ok(())
    }
    /// 状态转移校验；不允许跳变或回退到已完成状态；相同状态为幂等无操作（支持续算）。
    pub fn advance(&mut self, next: StageState) -> Result<()> {
        if self.state == next {
            return Ok(());
        }
        use StageState::*;
        let ok = matches!(
            (self.state, next),
            (Unprepared, Ready)
                | (Ready, SubmissionPending)
                | (SubmissionPending, Submitted)
                | (SubmissionPending, Blocked)
                | (SubmissionPending, Failed)
                | (SubmissionPending, Unknown)
                | (Submitted, Running)
                | (Submitted, Failed)
                | (Submitted, Unknown)
                | (Running, Validating)
                | (Running, Failed)
                | (Running, Unknown)
                | (Validating, Succeeded)
                | (Validating, Failed)
        );
        ensure!(ok, "data_invalid: 阶段 {} 不允许 {:?} -> {:?}", self.stage_id, self.state, next);
        self.state = next;
        Ok(())
    }
}

/// 依赖无环校验；依赖必须全部已成功，否则阻断下游。
pub fn dependencies_satisfied(
    stage_id: &str,
    deps: &[String],
    states: &BTreeMap<String, StageState>,
) -> Result<()> {
    for d in deps {
        match states.get(d) {
            Some(StageState::Succeeded) => {}
            Some(s) => bail!("data_invalid: 阶段 {stage_id} 依赖 {d} 状态为 {s:?}，未成功"),
            None => bail!("data_invalid: 阶段 {stage_id} 依赖 {d} 不存在"),
        }
    }
    Ok(())
}
