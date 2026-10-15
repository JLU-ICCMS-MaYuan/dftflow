use anyhow::{bail, Context, ensure, Result};
use std::{
    fs,
    path::Path,
    process::{Command, ExitStatus},
};

/// 已提交作业身份；用可关联的尝试身份核对调度记录。
#[derive(Debug, Clone)]
pub struct SubmittedJob {
    pub job_id: u64,
    pub cluster: Option<String>,
}

/// 构造 sbatch 脚本并原生提交；核对退出码与唯一正整数作业号。
pub fn submit_via_sbatch(
    launcher: &[String],
    command: &[String],
    work: &Path,
    name: &str,
) -> Result<SubmittedJob> {
    let dir = work.join(".epwflow").join("slurm");
    fs::create_dir_all(&dir)?;
    let script = dir.join(format!("submit-{name}.sh"));
    let mut body = String::from("#!/bin/bash\n");
    body.push_str(&format!("#SBATCH --job-name={name}\n"));
    let line = {
        let mut v = launcher.to_vec();
        v.extend_from_slice(command);
        v.join(" ")
    };
    body.push_str(&line);
    body.push('\n');
    fs::write(&script, body).with_context(|| format!("execution_failed: 无法写脚本 {}", script.display()))?;

    let out = Command::new("sbatch")
        .arg("--parsable")
        .arg(&script)
        .current_dir(work)
        .output()
        .with_context(|| "scheduler_unknown: 无法调用 sbatch")?;
    if !out.status.success() {
        bail!(
            "scheduler_unknown: sbatch 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // 形如 <jobid> 或 <jobid>;<cluster>
    let mut parts = txt.split(';');
    let job_id: u64 = parts
        .next()
        .and_then(|s| s.trim().parse().ok())
        .context("scheduler_unknown: sbatch 未返回正整数作业号")?;
    let cluster = parts.next().map(|s| s.trim().to_string());
    Ok(SubmittedJob { job_id, cluster })
}

/// 本地执行：不经过 Slurm，用于测试阶段与登录节点受限运行；路径/参数列表处理，不经 shell 求值。
pub fn run_local(command: &[String], work: &Path) -> Result<ExitStatus> {
    ensure!(
        !command.is_empty(),
        "execution_failed: 空命令"
    );
    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(work)
        .status()
        .with_context(|| format!("execution_failed: 无法启动 {}", command[0]))?;
    Ok(status)
}

/// 查询作业终态（best-effort）；查询失败或查无记录保持未知，不自动重提。
pub fn query(job_id: u64) -> Option<String> {
    let out = Command::new("sacct")
        .args([
            "-j",
            &job_id.to_string(),
            "--noheader",
            "-o",
            "State",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .map(|l| l.trim().to_string())
        .find(|l| !l.is_empty())
}
