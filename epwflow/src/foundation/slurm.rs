use anyhow::{bail, ensure, Context, Result};
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
    body.push_str("set -euo pipefail\n");
    let line = {
        let mut v = launcher.to_vec();
        v.extend_from_slice(command);
        v.iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ")
    };
    body.push_str(&line);
    body.push('\n');
    fs::write(&script, body)
        .with_context(|| format!("execution_failed: 无法写脚本 {}", script.display()))?;

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
    parse_job_id(&txt)
}

/// 本地执行：不经过 Slurm，用于测试阶段与登录节点受限运行；路径/参数列表处理，不经 shell 求值。
pub fn run_local(command: &[String], work: &Path) -> Result<ExitStatus> {
    ensure!(!command.is_empty(), "execution_failed: 空命令");
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
        .args(["-j", &job_id.to_string(), "--noheader", "-o", "State"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .map(|l| l.trim().to_string())
        .find(|l| !l.is_empty())
}

/// 仅接受单行正整数及可选集群名；不得接受警告文本、0或多字段。
pub fn parse_job_id(text: &str) -> Result<SubmittedJob> {
    let re = regex::Regex::new(r"^([1-9][0-9]*)(?:;([A-Za-z0-9_.-]+))?$")?;
    let captures = re
        .captures(text.trim())
        .context("scheduler_unknown: sbatch 作业号格式无效")?;
    Ok(SubmittedJob {
        job_id: captures[1].parse()?,
        cluster: captures.get(2).map(|v| v.as_str().to_owned()),
    })
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[derive(Debug, Clone)]
pub struct Accounting {
    pub state: String,
    pub terminal: bool,
    pub node_seconds: u64,
    pub nodes: u64,
}

#[derive(Debug)]
struct RunningNodesPending;
impl std::fmt::Display for RunningNodesPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scheduler_unknown: RUNNING 的节点数尚未记账")
    }
}
impl std::error::Error for RunningNodesPending {}

/// 启动前有限等待 RUNNING 的节点记账；未知记录和终态不重试，不修改预算预留。
pub fn running_allocation(job_id: u64) -> Result<Accounting> {
    running_with(
        10,
        || accounting(job_id),
        || std::thread::sleep(std::time::Duration::from_secs(1)),
    )
}

fn running_with(
    tries: usize,
    mut query: impl FnMut() -> Result<Accounting>,
    mut pause: impl FnMut(),
) -> Result<Accounting> {
    for i in 0..tries {
        match query() {
            Ok(a) if a.state == "RUNNING" && a.nodes > 0 => return Ok(a),
            Ok(a) if a.state == "RUNNING" && a.nodes == 0 => (),
            Err(e) if e.downcast_ref::<RunningNodesPending>().is_some() => (),
            Ok(_) => bail!("allocation: 当前分配未运行或已终止"),
            Err(e) => return Err(e),
        }
        if i + 1 < tries {
            pause();
        }
    }
    bail!("scheduler_unknown: 等待 RUNNING 节点记账超时；禁止启动科学程序")
}

/// 只采用 allocation 行，排除 batch/extern 等 step，未知状态保持预留。
pub fn accounting(job_id: u64) -> Result<Accounting> {
    ensure!(job_id > 0, "scheduler_unknown: 无效作业号");
    let out = Command::new("sacct")
        .args([
            "--noheader",
            "--parsable2",
            "-j",
            &job_id.to_string(),
            "--format=JobIDRaw,State,ElapsedRaw,AllocNodes",
        ])
        .output()
        .context("scheduler_unknown: 无法查询 sacct")?;
    ensure!(
        out.status.success(),
        "scheduler_unknown: sacct 失败：{}",
        String::from_utf8_lossy(&out.stderr)
    );
    parse_accounting(job_id, &String::from_utf8(out.stdout)?)
}

pub fn parse_accounting(job_id: u64, text: &str) -> Result<Accounting> {
    let id = job_id.to_string();
    let rows: Vec<Vec<&str>> = text
        .lines()
        .map(|l| l.trim().split('|').collect::<Vec<_>>())
        .filter(|r| r.first().is_some_and(|s| *s == id))
        .collect();
    ensure!(
        rows.len() == 1 && rows[0].len() >= 4,
        "scheduler_unknown: allocation 记录缺失或不唯一"
    );
    let r = &rows[0];
    let state = r[1].split_whitespace().next().unwrap_or("");
    let terminal = matches!(
        state,
        "COMPLETED"
            | "FAILED"
            | "CANCELLED"
            | "TIMEOUT"
            | "OUT_OF_MEMORY"
            | "NODE_FAIL"
            | "PREEMPTED"
            | "BOOT_FAIL"
            | "DEADLINE"
            | "REVOKED"
    );
    ensure!(
        terminal
            || matches!(
                state,
                "PENDING"
                    | "RUNNING"
                    | "COMPLETING"
                    | "CONFIGURING"
                    | "SUSPENDED"
                    | "REQUEUED"
                    | "REQUEUE_FED"
                    | "RESIZING"
            ),
        "scheduler_unknown: 无法判定作业状态 {state}"
    );
    let elapsed: u64 = r[2]
        .parse()
        .context("scheduler_unknown: ElapsedRaw 非整数")?;
    let nodes: u64 = r[3]
        .parse()
        .context("scheduler_unknown: AllocNodes 非整数")?;
    if state == "RUNNING" && nodes == 0 && elapsed > 0 {
        return Err(RunningNodesPending.into());
    }
    ensure!(
        elapsed == 0 || nodes > 0,
        "scheduler_unknown: 已占用时间但节点数为零"
    );
    Ok(Accounting {
        state: state.to_owned(),
        terminal,
        nodes,
        node_seconds: elapsed
            .checked_mul(nodes)
            .context("scheduler_unknown: 节点时间溢出")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_waits_only_for_running_node_accounting() {
        let mut rows = ["12|RUNNING|1|0|", "12|RUNNING|2|1|"].into_iter();
        let mut sleeps = 0;
        let a = running_with(
            3,
            || parse_accounting(12, rows.next().unwrap()),
            || sleeps += 1,
        )
        .unwrap();
        assert_eq!(a.nodes, 1);
        assert_eq!(a.node_seconds, 2);
        assert_eq!(sleeps, 1);
        // 结算仍拒绝零节点；启动等待不制造有效节点时间。
        assert!(parse_accounting(12, "12|RUNNING|1|0|").is_err());
        let mut calls = 0;
        assert!(running_with(
            3,
            || {
                calls += 1;
                parse_accounting(12, "12|RUNNING|1|0|")
            },
            || ()
        )
        .is_err());
        assert_eq!(calls, 3);
        for row in [
            "12|FAILED|2|1|",
            "12|PENDING|0|0|",
            "12|MYSTERY|1|1|",
            "12|COMPLETED|2|0|",
            "",
            "12|RUNNING|2|1|\n12|RUNNING|2|1|",
        ] {
            let mut calls = 0;
            assert!(running_with(
                3,
                || {
                    calls += 1;
                    parse_accounting(12, row)
                },
                || panic!("未知或终态不得等待")
            )
            .is_err());
            assert_eq!(calls, 1, "{row}");
        }
    }
}
