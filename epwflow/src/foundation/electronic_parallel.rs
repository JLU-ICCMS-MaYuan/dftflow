//! 第三步的两个独立 Slurm 分支；共享日志只在短暂更新时加锁。
use super::{config::Config, native_jobs, readable_state as log, slurm, stages::{self, Step}, store};
use anyhow::{ensure, Context, Result};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

const BRANCHES: [&str; 2] = ["qe_eband", "qe_eledos"];
fn label(branch: &str) -> String { format!("Electronic branch · {branch}") }

/// 所有公开写入口都要先确认两个分支没有活动或未知提交。
pub(super) fn check_active(cfg: &Config) -> Result<()> {
    let doc = log::read(&cfg.work_dir, Step::Electronic)?;
    let jobs = log::jobs(&cfg.work_dir, Step::Electronic)?;
    for branch in BRANCHES {
        let label = label(branch);
        let status = doc.get(&label, "Status").unwrap_or("not started");
        if !matches!(status, "submitting" | "submission unknown" | "submitted" | "running") { continue; }
        let job = jobs.get(&label).context("scheduler_unknown: electronic branch submission receipt missing; inspect Slurm before retrying")?;
        let accounting = slurm::accounting(*job)?;
        ensure!(accounting.terminal,
            "scheduler_unknown: electronic branch {branch} job {job} remains {}; duplicate submission prohibited", accounting.state);
        log::update(&cfg.work_dir, Step::Electronic, |d| {
            d.set(&label, "Status", "exited; validation pending");
            Ok(())
        })?;
    }
    Ok(())
}

/// 已持工作区锁；两个 sbatch 连续提交，均不等待科学执行结束。
pub(super) fn prepare(cfg: &Config, version: &Path, resume: bool, submit: bool) -> Result<Vec<u64>> {
    let previous = log::read(&cfg.work_dir, Step::Electronic)?;
    let batch = previous.get("Stage", "Batch").map(str::to_owned).unwrap_or_else(store::token);
    log::update(&cfg.work_dir, Step::Electronic, |d| {
        d.set("Stage", "Batch", &batch);
        d.set("Stage", "Calculation configuration", log::relative(&cfg.work_dir, version));
        d.set("Stage", "Status", "advancing");
        d.remove("Problem");
        Ok(())
    })?;
    let mut jobs = Vec::new();
    for branch in BRANCHES {
        super::workspace_electronic::repair_checkpoints(cfg, branch, resume)?;
        if super::workspace_electronic::branch_completed(cfg, branch)? {
            log::report_reuse(cfg, Step::Electronic, Some(branch));
            continue;
        }
        let label = label(branch);
        let dir = Step::Electronic.directory(cfg).join(branch);
        fs::create_dir_all(&dir)?;
        let id = store::token();
        let script = dir.join(format!("{branch}.sh"));
        let q = slurm::shell_quote;
        let mut body = slurm::script_header(cfg, &dir, branch,
            &format!("slurm-{branch}.log"), &format!("slurm-{branch}.log"));
        if let Some(env) = &cfg.programs.environment_script {
            body.push_str(&super::workflow::source_environment(env)?);
        }
        body.push_str(&format!("exec {} --config {} --electronic-branch {branch} --allocation-id {}{}\n",
            q(&std::env::current_exe()?.to_string_lossy()), q(&version.to_string_lossy()), q(&id),
            if resume { " --resume" } else { "" }));
        store::atomic_bytes(&script, body.as_bytes())?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
        log::set_job(&cfg.work_dir, Step::Electronic, &label, None)?;
        log::update(&cfg.work_dir, Step::Electronic, |d| {
            d.remove(&label);
            d.set(&label, "Submission ID", &id);
            d.set(&label, "Status", if submit { "submitting" } else { "prepared" });
            d.set(&label, "Configuration", log::relative(&cfg.work_dir, version));
            d.set(&label, "Script", log::relative(&cfg.work_dir, &script));
            Ok(())
        })?;
        println!("script: {}; branch: {branch}", script.display());
        if !submit { continue; }
        let result = Command::new("sbatch").arg("--parsable").arg(&script).current_dir(&dir).output()
            .context("scheduler_unknown: cannot invoke sbatch")
            .and_then(|out| {
                ensure!(out.status.success(), "scheduler_unknown: electronic branch sbatch failed: {}", String::from_utf8_lossy(&out.stderr));
                slurm::parse_job_id(String::from_utf8_lossy(&out.stdout).trim())
            });
        match result {
            Ok(job) => {
                log::set_job(&cfg.work_dir, Step::Electronic, &label, Some(job.job_id))?;
                log::update(&cfg.work_dir, Step::Electronic, |d| {
                    if d.get(&label, "Status") == Some("submitting") { d.set(&label, "Status", "submitted"); }
                    Ok(())
                })?;
                jobs.push(job.job_id);
                println!("branch: {branch}; job_id: {}", job.job_id);
            }
            Err(e) => {
                log::update(&cfg.work_dir, Step::Electronic, |d| {
                    if d.get(&label, "Status") == Some("submitting") { d.set(&label, "Status", "submission unknown"); }
                    d.set(&label, "Failure", format!("{e:#}"));
                    Ok(())
                })?;
                return Err(e);
            }
        }
    }
    Ok(jobs)
}

/// 分支入口只允许已经登记的脚本和当前 Slurm 分配执行。
pub(crate) fn allocated(config: &Path, branch: &str, resume: bool, id: &str) -> Result<()> {
    ensure!(BRANCHES.contains(&branch), "configuration: unknown electronic branch {branch}");
    let cfg = log::load_config(config)?;
    let _config = log::ExecutionConfig::enter(&cfg.work_dir, config);
    let dir = Step::Electronic.directory(&cfg).join(branch);
    let _branch = log::DirectoryGuard::acquire(&dir)?;
    let label = label(branch);
    let job: u64 = std::env::var("SLURM_JOB_ID").context("allocation: electronic branch requires Slurm")?.parse()?;
    let allocation = slurm::running_allocation(job)?;
    ensure!(!allocation.terminal && allocation.state == "RUNNING", "allocation: electronic branch allocation is not running");
    super::parallelism::preflight(&cfg, Step::Electronic, true)?;
    let _state = log::DirectoryGuard::acquire(&cfg.work_dir.join("epw-state"))?;
    let doc = log::read(&cfg.work_dir, Step::Electronic)?;
    ensure!(doc.get(&label, "Submission ID") == Some(id)
        && matches!(doc.get(&label, "Status"), Some("submitting" | "submitted")),
        "input_conflict: electronic branch script is stale or already executed");
    let bound = log::resolve(&cfg.work_dir, doc.get(&label, "Configuration").context("allocation: branch configuration missing")?);
    ensure!(fs::canonicalize(&bound)? == fs::canonicalize(config)?, "input_conflict: electronic branch configuration mismatch");
    ensure!(log::jobs(&cfg.work_dir, Step::Electronic)?.get(&label).is_none_or(|j| *j == job),
        "input_conflict: electronic branch job identity mismatch");
    let generation = format!("{}-{branch}", doc.get("Stage", "Batch").context("allocation: electronic batch missing")?);
    log::set_job(&cfg.work_dir, Step::Electronic, &label, Some(job))?;
    log::update(&cfg.work_dir, Step::Electronic, |d| { d.set(&label, "Status", "running"); Ok(()) })?;
    drop(_state);
    let _reuse = super::electronic_reuse::Scope::enter(&cfg)?;
    let result = native_jobs::scoped_allocated(&cfg, &generation, resume,
        || super::workspace_electronic::run_leaf(&cfg, branch));
    if result.is_ok() { native_jobs::accept(&cfg, &generation)?; }
    else { native_jobs::reject(&cfg, &generation)?; }
    log::update(&cfg.work_dir, Step::Electronic, |d| {
        d.set(&label, "Status", if result.is_ok() { "completed" } else { "failed" });
        if let Err(e) = &result { d.set(&label, "Failure", format!("{e:#}")); }
        Ok(())
    })?;
    result
}

/// afterany 只负责唤醒；真实科学成功必须由产物核验确认。
pub(super) fn ready_to_finish(cfg: &Config) -> Result<()> {
    let doc = log::read(&cfg.work_dir, Step::Electronic)?;
    if BRANCHES.iter().all(|b| matches!(doc.get(&label(b), "Status"), None | Some("prepared"))) { return Ok(()); }
    let mut incomplete = Vec::new();
    for branch in BRANCHES {
        match super::workspace_electronic::branch_completed(cfg, branch) {
            Ok(true) => (),
            Ok(false) => incomplete.push(branch.to_owned()),
            Err(e) => incomplete.push(format!("{branch}: {e:#}")),
        }
    }
    if !incomplete.is_empty() {
        // 保留核心能带／总 DOS 的科学交付，投影失败仍是分支失败。
        if super::workspace_electronic::completed(cfg).unwrap_or(false) { stages::finish(cfg, Step::Electronic)?; }
        log::update(&cfg.work_dir, Step::Electronic, |d| {
            d.set("Stage", "Status", "failed");
            d.set("Problem", "Reason", format!("Electronic branches incomplete: {}", incomplete.join("; ")));
            Ok(())
        })?;
        anyhow::bail!("execution_failed: electronic branches incomplete: {}; successful results retained; fix the cause and use --resume -s", incomplete.join("; "));
    }
    Ok(())
}
