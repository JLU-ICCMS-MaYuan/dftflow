use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use super::{config, store};

/// 环境证据：检查层级、程序身份、环境、版本、时间及诊断；失配后不可继续复用（数据模型 v1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentEvidence {
    pub schema_version: u32,
    pub executable_sha256: String,
    pub launcher: Vec<String>,
    pub env_script_sha256: Option<String>,
    pub host: String,
    pub observed_version: String,
    pub time: u64,
    pub status: String,
}

impl EnvironmentEvidence {
    pub fn load(path: &Path) -> Result<Self> {
        let e: Self = store::read(path)?;
        ensure!(e.schema_version == 1, "unsupported_version: 环境证据 schema_version 必须为 1");
        Ok(e)
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 探测单个可执行文件身份与运行版本；仅验证启动与版本，不视作科学阶段成功。
pub fn verify_program(path: &Path) -> Result<EnvironmentEvidence> {
    let sha256 = store::hash_file(path)?;
    let host = store::hostname();
    let out = Command::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("environment: 无法启动 {}", path.display()))?;
    let observed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let observed = if observed.is_empty() {
        String::from_utf8_lossy(&out.stderr).trim().to_string()
    } else {
        observed
    };
    let status = if out.status.success() {
        "version_reported"
    } else {
        "version_probe_failed"
    };
    Ok(EnvironmentEvidence {
        schema_version: 1,
        executable_sha256: sha256,
        launcher: vec![],
        env_script_sha256: None,
        host,
        observed_version: observed,
        time: now(),
        status: status.to_string(),
    })
}

/// 对配置的程序集合做分层环境证据（配置可解析、文件可访问、程序可启动、版本已验证）。
pub fn verify_configuration(cfg: &config::Config) -> Result<Vec<EnvironmentEvidence>> {
    let mut ev = Vec::new();
    for (n, p) in [
        ("pw", &cfg.programs.pw),
        ("ph", &cfg.programs.ph),
        ("epw", &cfg.programs.epw),
    ]
    .into_iter()
    .filter(|(_, p)| !p.path.as_os_str().is_empty())
    {
        ev.push(verify_program(&p.path).with_context(|| format!("environment: 程序 {n} 探测失败"))?);
    }
    Ok(ev)
}

/// probe 入口：选择校验 + 分配约束；科学阶段未接入时 -s 不可用，-r 需要分配记录。
pub fn probe(config_path: &Path, submit: bool, require_allocation: bool) -> Result<()> {
    if submit && require_allocation {
        bail!("cannot: probe 的 -s 与 -r 必须互斥");
    }
    if !submit && !require_allocation {
        bail!("required: probe 必须明确选择 -s 或 -r");
    }
    if require_allocation {
        let job = std::env::var("SLURM_JOB_ID").ok();
        ensure!(job.is_some(), "allocation: probe -r 需要当前 SLURM_JOB_ID");
        bail!("allocation: probe -r 需要已登记的分配记录且与 SLURM_JOB_ID 一致");
    }
    // submit 模式在 F1 需要提交探测作业，科学阶段尚未接入。
    let _ = config_path;
    bail!("not_implemented: 科学阶段尚未接入，probe -s 不可用");
}
