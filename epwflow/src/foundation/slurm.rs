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

/// 日志使用文件名；Slurm 在执行正文前按脚本目录打开日志，不能依赖正文的 cd。
pub(super) fn log_directives(directory: &Path, output: &str, error: &str) -> String {
    format!(
        "#SBATCH --chdir={}\n#SBATCH --output={output}\n#SBATCH --error={error}\n",
        shell_quote(&directory.to_string_lossy())
    )
}

/// 资源指令完全来自用户 header；默认日志在前，用户显式设置优先。
pub fn script_header(cfg: &super::config::Config, directory: &Path, name: &str, output: &str, error: &str) -> String {
    let mut text = format!("#!/bin/bash\n#SBATCH --job-name={name}\n{}#SBATCH --open-mode=truncate\n", log_directives(directory, output, error));
    if let Some(header) = &cfg.slurm.header {
        text.push_str(header);
        if !header.ends_with('\n') { text.push('\n'); }
    }
    text
}

/// 构造 sbatch 脚本并原生提交；核对退出码与唯一正整数作业号。
pub fn submit_via_sbatch(
    launcher: &[String],
    command: &[String],
    work: &Path,
    name: &str,
    header: &str,
) -> Result<SubmittedJob> {
    let dir = work.join("epw-state").join("slurm");
    fs::create_dir_all(&dir)?;
    let script = dir.join(format!("submit-{name}.sh"));
    let mut body = String::from("#!/bin/bash\n");
    body.push_str(&format!("#SBATCH --job-name={name}\n"));
    body.push_str(&log_directives(&dir, "slurm-%j.out", "slurm-%j.err"));
    body.push_str(header);
    if !body.ends_with('\n') { body.push('\n'); }
    body.push_str(&format!("cd -- {} || exit $?\n", shell_quote(&work.to_string_lossy())));
    let line = {
        let mut v = launcher.to_vec();
        v.extend_from_slice(command);
        qf_runner::plain_scientific_argv(&v).iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ")
    };
    body.push_str(&line);
    body.push('\n');
    super::store::atomic_bytes(&script, body.as_bytes())
        .with_context(|| format!("execution_failed: Could not write script {}", script.display()))?;

    let out = Command::new("sbatch")
        .arg("--parsable")
        .arg(&script)
        .current_dir(work)
        .output()
        .with_context(|| "scheduler_unknown: cannot invoke sbatch")?;
    if !out.status.success() {
        bail!(
            "scheduler_unknown: sbatch Failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
    parse_job_id(&txt)
}

/// 本地执行：不经过 Slurm，用于测试阶段与登录节点受限运行；路径/参数列表处理，不经 shell 求值。
pub fn run_local(command: &[String], work: &Path) -> Result<ExitStatus> {
    ensure!(!command.is_empty(), "execution_failed: empty command");
    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(work)
        .status()
        .with_context(|| format!("execution_failed: cannot start {}", command[0]))?;
    Ok(status)
}

/// 查询作业终态（best-effort）；查询失败或查无记录保持未知，不自动重提。
pub fn query(job_id: u64) -> Option<String> {
    let out = Command::new("sacct")
        .args(["-j", &job_id.to_string(), "--noheader", "-o", "State"])
        .output()
        .ok()?;
    if !out.status.success() {
        return if String::from_utf8_lossy(&out.stderr).contains("Slurm accounting storage is disabled") {
            accounting(job_id).ok().map(|a| a.state)
        } else { None };
    }
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
        .context("scheduler_unknown: sbatch Job Number Format Invalid")?;
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
        write!(f, "scheduler_unknown: RUNNING Nodes not yet accounted for")
    }
}
impl std::error::Error for RunningNodesPending {}

/// 启动读取控制器实时分配；终态由独立的 sacct 查询负责。
pub fn running_allocation(job_id: u64) -> Result<Accounting> {
    running_with(
        10,
        || controller_allocation(job_id),
        || std::thread::sleep(std::time::Duration::from_secs(1)),
    )
}

fn controller_allocation(job_id: u64) -> Result<Accounting> {
    ensure!(job_id > 0, "allocation: Invalid Job Number");
    let out = Command::new("scontrol")
        .args(["show", "job", "-o", &job_id.to_string()])
        .output()
        .context("scheduler_unknown: Unable to query scontrol")?;
    ensure!(
        out.status.success(),
        "scheduler_unknown: scontrol Failed:{}",
        String::from_utf8_lossy(&out.stderr)
    );
    parse_controller(job_id, &String::from_utf8(out.stdout)?)
}

/// 仅供启动门使用，不提供运行时长，不从环境变量猜测资源。
fn parse_controller(job_id: u64, text: &str) -> Result<Accounting> {
    let rows: Vec<_> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    ensure!(rows.len() == 1, "scheduler_unknown: Control record missing or not unique");
    let mut fields = std::collections::BTreeMap::new();
    for field in rows[0].split_whitespace() {
        if let Some((key, value)) = field.split_once('=') {
            if matches!(key, "JobId" | "JobState" | "NumNodes") {
                ensure!(
                    fields.insert(key, value).is_none(),
                    "scheduler_unknown: Control field repeats"
                );
            }
        }
    }
    ensure!(
        fields.get("JobId").copied() == Some(job_id.to_string().as_str()),
        "input_conflict: Controlr operational identity conflict"
    );
    ensure!(
        fields.get("JobState") == Some(&"RUNNING"),
        "allocation: The controller 's current distribution is not running or terminated"
    );
    let nodes = fields
        .get("NumNodes")
        .context("scheduler_unknown: controller missing nodes")?
        .parse()
        .context("scheduler_unknown: Control Node Non-integer")?;
    Ok(Accounting {
        state: "RUNNING".into(),
        terminal: false,
        nodes,
        node_seconds: 0,
    })
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
            Ok(_) => bail!("allocation: Current distribution is not operational or terminated"),
            Err(e) => return Err(e),
        }
        if i + 1 < tries {
            pause();
        }
    }
    bail!("scheduler_unknown: Waiting for controller RUNNING Nodes timed out; prohibition on starting scientific programs")
}

/// 只采用 allocation 行，排除 batch/extern 等 step，未知状态拒绝继续。
pub fn accounting(job_id: u64) -> Result<Accounting> {
    ensure!(job_id > 0, "scheduler_unknown: Invalid Job Number");
    let out = Command::new("sacct")
        .args([
            "--noheader",
            "--parsable2",
            "-j",
            &job_id.to_string(),
            "--format=JobIDRaw,State,ElapsedRaw,NNodes",
        ])
        .output()
        .context("scheduler_unknown: Unable to query sacct")?;
    if !out.status.success()
        && String::from_utf8_lossy(&out.stderr).contains("Slurm accounting storage is disabled")
    {
        if let Ok(active) = controller_allocation(job_id) {
            return Ok(active);
        }
        return completion_accounting(job_id);
    }
    ensure!(
        out.status.success(),
        "scheduler_unknown: sacct Failed:{}",
        String::from_utf8_lossy(&out.stderr)
    );
    parse_accounting(job_id, &String::from_utf8(out.stdout)?)
}

/// filetxt 后端的 sacct 会以可写方式打开日志；查询私有副本，不改权限或原日志。
fn completion_accounting(job_id: u64) -> Result<Accounting> {
    let text = completion_query(job_id, "--format=JobIDRaw,State,ElapsedRaw,NNodes")?;
    let observation = parse_accounting(job_id, &text)?;
    ensure!(observation.terminal, "scheduler_unknown: There is no reliable end to the completed log");
    Ok(observation)
}

/// 原生任务终态的退出码与状态使用同一真实 Slurm 后端。
pub fn exit_code_record(job_id: u64) -> Result<String> {
    ensure!(job_id > 0, "scheduler_unknown: Invalid Job Number");
    let out = Command::new("sacct").args(["--noheader", "--parsable2", "-j",
        &job_id.to_string(), "--format=JobIDRaw,ExitCode"]).output()?;
    if !out.status.success()
        && String::from_utf8_lossy(&out.stderr).contains("Slurm accounting storage is disabled")
    {
        return completion_query(job_id, "--format=JobIDRaw,ExitCode");
    }
    ensure!(out.status.success(), "scheduler_unknown: _Other Organiser {job_id} Exit Code");
    Ok(String::from_utf8(out.stdout)?)
}

fn completion_query(job_id: u64, fields: &str) -> Result<String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let config = Command::new("scontrol").args(["show", "config"]).output()
        .context("scheduler_unknown: Unable to query Slurm Log Configuration Completed")?;
    ensure!(config.status.success(), "scheduler_unknown: Slurm Failed to complete log configuration query");
    let text = String::from_utf8(config.stdout)?;
    let value = |key: &str| -> Result<&str> {
        let values: Vec<_> = text.lines().filter_map(|line| line.split_once('='))
            .filter(|(name, _)| name.trim() == key).map(|(_, value)| value.trim()).collect();
        ensure!(values.len() == 1, "scheduler_unknown: Slurm {key} Missing or duplicated");
        Ok(values[0])
    };
    ensure!(value("JobCompType")? == "jobcomp/filetxt",
        "scheduler_unknown: accounting Not enabled and not available jobcomp/filetxt Finish Log");
    let source = Path::new(value("JobCompLoc")?);
    ensure!(source.is_absolute(), "scheduler_unknown: Slurm The completed log path is not an absolute path");
    let bytes = fs::read(source).context("scheduler_unknown: Unable to read Slurm Finish Log")?;
    let snapshot = std::env::temp_dir().join(format!("epw-jobcomp-{}.log", super::store::token()));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
        .open(&snapshot)?;
    let result = (|| {
        file.write_all(&bytes)?;
        drop(file);
        let out = Command::new("sacct").args(["--completion", "--file"])
            .arg(&snapshot).args(["--noheader", "--parsable2", "-j", &job_id.to_string(),
                fields]).output()
            .context("scheduler_unknown: Unable to query Slurm Finish Log")?;
        ensure!(out.status.success(), "scheduler_unknown: Slurm Failed to complete log query:{}",
            String::from_utf8_lossy(&out.stderr));
        Ok(String::from_utf8(out.stdout)?)
    })();
    let cleanup = fs::remove_file(&snapshot);
    cleanup.context("scheduler_unknown: Unable to delete Slurm Copy completed log query")?;
    result
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
        "scheduler_unknown: allocation Missing or not unique"
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
        "scheduler_unknown: Could not determine operational status {state}"
    );
    let elapsed: u64 = r[2]
        .parse()
        .context("scheduler_unknown: ElapsedRaw Non-integer")?;
    let nodes: u64 = r[3]
        .parse()
        .context("scheduler_unknown: AllocNodes Non-integer")?;
    if state == "RUNNING" && nodes == 0 && elapsed > 0 {
        return Err(RunningNodesPending.into());
    }
    ensure!(
        elapsed == 0 || nodes > 0,
        "scheduler_unknown: Time taken but zero nodes"
    );
    Ok(Accounting {
        state: state.to_owned(),
        terminal,
        nodes,
        node_seconds: elapsed
            .checked_mul(nodes)
            .context("scheduler_unknown: Node time spills")?,
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
                || panic!("Unknown or terminal cannot wait")
            )
            .is_err());
            assert_eq!(calls, 1, "{row}");
        }
    }
    #[test]
    fn controller_startup_is_bound_to_one_running_job_and_has_no_billing() {
        let a =
            parse_controller(12, "JobId=12 JobName=fixture JobState=RUNNING NumNodes=2").unwrap();
        assert_eq!(a.nodes, 2);
        assert_eq!(a.node_seconds, 0);
        for row in [
            "",
            "JobId=13 JobState=RUNNING NumNodes=1",
            "JobId=12 JobState=COMPLETED NumNodes=1",
            "JobId=12 JobState=PENDING NumNodes=1",
            "JobId=12 JobState=RUNNING NumNodes=bad",
            "JobId=12 JobState=RUNNING",
            "JobId=12 JobState=RUNNING NumNodes=1 JobId=12",
            "JobId=12 JobState=RUNNING NumNodes=1\nJobId=12 JobState=RUNNING NumNodes=1",
        ] {
            assert!(parse_controller(12, row).is_err(), "{row}");
        }
        let mut rows = [
            "JobId=12 JobState=RUNNING NumNodes=0",
            "JobId=12 JobState=RUNNING NumNodes=1",
        ]
        .into_iter();
        let mut waits = 0;
        let a = running_with(
            2,
            || parse_controller(12, rows.next().unwrap()),
            || waits += 1,
        )
        .unwrap();
        assert_eq!(a.nodes, 1);
        assert_eq!(waits, 1);
    }
}
