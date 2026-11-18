//! 编号工作区提交入口；分配状态写入阶段日志，作业号保存在计算目录。
use super::{
    budget,
    config::Config,
    native_jobs, parallelism, readable_state as log, slurm,
    stages::{self, Step},
    store,
};
use anyhow::{ensure, Context, Result};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

pub(super) fn check_active(cfg: &Config, own: Option<&str>) -> Result<()> {
    native_jobs::check_orphan_jobs(cfg)?;
    for step in Step::ALL {
        let doc = log::read(&cfg.work_dir, step)?;
        if doc.get("阶段分配", "提交标识") == own && own.is_some() {
            continue;
        }
        let state = doc.get("阶段分配", "状态").unwrap_or("未开始");
        if !matches!(state, "提交中" | "提交未知" | "已提交" | "运行中") {
            continue;
        }
        let job = log::jobs(&cfg.work_dir, step)?.get("阶段分配").copied();
        let job = job
            .context("scheduler_unknown: 阶段提交尚无可确认作业号；请核对 Slurm 后补齐任务号表")?;
        if std::env::var("SLURM_JOB_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            == Some(job)
        {
            continue;
        }
        let accounting = slurm::accounting(job)?;
        ensure!(
            accounting.terminal,
            "scheduler_unknown: 阶段 {} 作业 {job} 仍活动，禁止覆盖",
            step.name()
        );
        budget::with_ledger(
            &cfg.budget.ledger,
            &cfg.budget.campaign_id,
            budget::reconcile,
        )?;
        log::update(&cfg.work_dir, step, |d| {
            d.set(
                "阶段分配",
                "状态",
                if accounting.state == "COMPLETED" {
                    "已退出，待检查科学结果"
                } else {
                    "失败"
                },
            );
            Ok(())
        })?;
        crate::delivery::decay::recover_finished(cfg, job)?;
    }
    Ok(())
}
pub fn report(cfg: &Config) -> Result<serde_json::Value> {
    let mut steps = serde_json::Map::new();
    let mut complete = true;
    for step in Step::ALL {
        let reusable = stages::reusable(cfg, step)?;
        complete &= reusable;
        steps.insert(step.name().into(),serde_json::json!({"状态":log::read(&cfg.work_dir,step)?.get("阶段","状态"),"可复用":reusable}));
    }
    Ok(
        serde_json::json!({"schema_version":1,"operation_status":"ok","workflow_status":if complete {"complete"} else {"incomplete"},"workflow_complete":complete,"steps":steps,"native_task":native_jobs::status(cfg)?,"native_methods":native_jobs::groups_status(cfg)?}),
    )
}
pub fn status(cfg: &Config) -> Result<()> {
    for step in Step::ALL {
        let doc = log::read(&cfg.work_dir, step)?;
        println!(
            "{}：{}；科学结果{}",
            log::stage_name(step, log::mode(cfg)),
            doc.get("阶段", "状态").unwrap_or("未开始"),
            if stages::reusable(cfg, step)? {
                "可复用"
            } else {
                "尚需检查或计算"
            }
        );
        for (name, job) in log::jobs(&cfg.work_dir, step)? {
            println!("  {name}：Slurm {job}");
        }
    }
    Ok(())
}
pub fn start_stage(config: &Path, target: Step, resume: bool) -> Result<()> {
    prepare_stage(config, target, resume, true)
}
pub fn start_phonon(config: &Path, resume: bool) -> Result<()> {
    start_stage(config, Step::Phonon, resume)
}
pub(crate) fn prepare_stage(config: &Path, target: Step, resume: bool, submit: bool) -> Result<()> {
    if target > Step::Coarse {
        return super::native_workspace::advance(config, target, submit, resume);
    }
    let cfg = super::readable_state::load_config(config)?;
    let operation = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    super::run_state::ready(&cfg.work_dir)?;
    check_active(&cfg, None)?;
    native_jobs::check_groups(&cfg)?;
    native_jobs::reconcile(&cfg, resume)?;
    if resume {
        crate::delivery::decay::recover_pending(&cfg)?;
    }
    if cfg.budget.ledger.is_file() {
        budget::with_ledger(
            &cfg.budget.ledger,
            &cfg.budget.campaign_id,
            budget::reconcile,
        )?;
    }
    if !resume {
        for task in native_jobs::tasks(&cfg)? {
            ensure!(
                task.state != "failed"
                    || !stages::execution_steps(&cfg, target)
                        .iter()
                        .any(|step| task.directory.starts_with(step.directory(&cfg))),
                "execution_failed: {} 上次失败；修复后使用 --resume -s",
                task.directory.display()
            );
        }
    }
    let version = log::backup_config(config)?;
    log::register_config(&cfg, config, &version)?;
    let mut pending = false;
    for step in stages::execution_steps(&cfg, target) {
        let reusable = match stages::reusable(&cfg, step) {
            Ok(value) => value,
            Err(_) if resume && step == target => false,
            Err(error) => return Err(error),
        };
        if !reusable {
            parallelism::preflight(&cfg, step, false)?;
            pending = true;
        }
    }
    if !pending && !super::workspace_science::post_pending(&cfg, target)? {
        println!("阶段已完成，现有科学结果可复用");
        return Ok(());
    }
    ensure!(
        resume || log::read(&cfg.work_dir, target)?.get("阶段分配", "状态") != Some("失败"),
        "stage_failed: 上次阶段分配失败，请使用 --resume 继续未完成的任务"
    );
    let id = store::token();
    let dir = target.directory(&cfg);
    fs::create_dir_all(&dir)?;
    let q = slurm::shell_quote;
    let mut body = slurm::script_header(&cfg, &dir, &format!("epw-{}", target.name()), &format!("slurm-{}.log", target.name()), &format!("slurm-{}.log", target.name()));
    if let Some(env) = &cfg.programs.environment_script {
        body.push_str(&super::workflow::source_environment(env)?);
    }
    body.push_str(&format!(
        "exec {} --config {} --allocated-stage {} --allocation-id {}{}\n",
        q(&std::env::current_exe()?.to_string_lossy()),
        q(&version.to_string_lossy()),
        target.name(),
        q(&id),
        if resume { " --resume" } else { "" }
    ));
    let script = dir.join(format!("{}.sh", target.name()));
    store::atomic_bytes(&script, body.as_bytes())?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    log::update(&cfg.work_dir, target, |d| {
        d.remove("阶段分配");
        d.set("阶段分配", "提交标识", &id);
        d.set("阶段分配", "状态", "已准备");
        d.set("阶段分配", "配置", log::relative(&cfg.work_dir, &version));
        d.set("阶段分配", "脚本", log::relative(&cfg.work_dir, &script));
        Ok(())
    })?;
    if !submit {
        println!("脚本：{}；尚未提交", script.display());
        return Ok(());
    }
    let reservation = budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
        budget::reconcile(l)?;
        budget::reserve_workflow(l, &format!("stage:{id}"))
    })?;
    log::set_job(&cfg.work_dir, target, "阶段分配", None)?;
    log::update(&cfg.work_dir, target, |d| {
        d.set("阶段分配", "预算关联", &reservation);
        d.set("阶段分配", "状态", "提交中");
        Ok(())
    })?;
    // 状态已落盘；不能在等待提交回执时阻塞可能已经启动的分配进程。
    drop(operation);
    let result = Command::new("sbatch")
        .arg("--parsable")
        .arg(&script)
        .current_dir(&dir)
        .output()
        .context("scheduler_unknown: 无法调用 sbatch")
        .and_then(|out| {
            ensure!(
                out.status.success(),
                "scheduler_unknown: sbatch 失败：{}",
                String::from_utf8_lossy(&out.stderr)
            );
            slurm::parse_job_id(String::from_utf8_lossy(&out.stdout).trim())
        });
    let _receipt = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    let current = log::read(&cfg.work_dir, target)?;
    if current.get("阶段分配", "提交标识") != Some(id.as_str()) {
        println!("此分配已收尾，阶段已推进到新的提交");
        return Ok(());
    }
    match result {
        Ok(job) => {
            log::set_job(&cfg.work_dir, target, "阶段分配", Some(job.job_id))?;
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                if l.has_unsettled_token(&reservation) {
                    budget::attach_once(l, &reservation, job.job_id)?;
                }
                Ok(())
            })?;
            log::update(&cfg.work_dir, target, |d| {
                if d.get("阶段分配", "状态") == Some("提交中") {
                    d.set("阶段分配", "状态", "已提交");
                }
                Ok(())
            })?;
            println!(
                "Slurm 作业号：{}\n脚本：{}\n本阶段将在分配内自动推进",
                job.job_id,
                script.display()
            );
            Ok(())
        }
        Err(e) => {
            log::update(&cfg.work_dir, target, |d| {
                if d.get("阶段分配", "状态") == Some("提交中") {
                    d.set("阶段分配", "状态", "提交未知");
                }
                d.set("问题", "原因", format!("{e:#}"));
                Ok(())
            })?;
            Err(e)
        }
    }
}
pub fn allocated(config: &Path, target: Step, resume: bool, id: &str) -> Result<()> {
    let cfg = super::readable_state::load_config(config)?;
    let job: u64 = std::env::var("SLURM_JOB_ID")
        .context("allocation: 需要 Slurm 分配")?
        .parse()?;
    let operation = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    let doc = log::read(&cfg.work_dir, target)?;
    ensure!(
        doc.get("阶段分配", "提交标识") == Some(id),
        "input_conflict: 脚本不属于当前阶段分配，请使用公开命令重新生成"
    );
    ensure!(
        !matches!(doc.get("阶段分配", "状态"), Some("运行中" | "已完成")),
        "scheduler_unknown: 同一分配已经执行"
    );
    ensure!(
        log::jobs(&cfg.work_dir, target)?
            .get("阶段分配")
            .is_none_or(|j| *j == job),
        "input_conflict: 阶段脚本绑定不同作业"
    );
    let allocation = slurm::running_allocation(job)?;
    ensure!(!allocation.terminal && allocation.state == "RUNNING", "allocation: 当前分配未运行");
    parallelism::preflight(&cfg, target, true)?;
    log::set_job(&cfg.work_dir, target, "阶段分配", Some(job))?;
    if let Some(token) = doc.get("阶段分配", "预算关联") {
        budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
            budget::attach_once(l, token, job)
        })?;
    }
    log::update(&cfg.work_dir, target, |d| {
        d.set("阶段分配", "状态", "运行中");
        Ok(())
    })?;
    drop(operation);
    let result = super::native_workspace::allocated(config, target, resume, id);
    log::update(&cfg.work_dir, target, |d| {
        d.set(
            "阶段分配",
            "状态",
            if result.is_ok() {
                "已完成"
            } else {
                "失败"
            },
        );
        if let Err(e) = &result {
            d.set("问题", "原因", format!("{e:#}"));
        }
        Ok(())
    })?;
    result
}
pub fn start(config: &Path, target: Step, submit: bool) -> Result<()> {
    if submit {
        return prepare_stage(config, target, false, true);
    }
    start_allocated(config, target, false)
}
pub fn start_allocated(config: &Path, target: Step, resume: bool) -> Result<()> {
    let cfg = super::readable_state::load_config(config)?;
    check_active(&cfg, None)?;
    let generation = store::token();
    let job = std::env::var("SLURM_JOB_ID")
        .context("allocation: -r 需要 Slurm 分配")?
        .parse::<u64>()?;
    parallelism::preflight(&cfg, target, true)?;
    budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
        budget::reconcile(l)?;
        budget::register_generated(l, job)
    })?;
    super::native_workspace::allocated(config, target, resume, &generation)
}
/// 仅删除已确认不再运行的旧内容，保留阶段根 inode。
pub(super) fn archive_step(cfg: &Config, step: Step) -> Result<()> {
    check_active(cfg, None)?;
    native_jobs::check_groups(cfg)?;
    let dir = step.directory(cfg);
    fs::create_dir_all(&dir)?;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".jobs") || name.ends_with(".sh") || slurm_log(&path) {
            continue;
        }
        if step == Step::Coarse && name == "comparison-native" {
            // QE 声子比较与电子模型独立；保留其已核验现场，其他派生数据正常重建。
            for child in fs::read_dir(&path)? {
                let child = child?;
                if matches!(child.file_name().to_str(), Some("force-constants" | "frequencies")) {
                    continue;
                }
                if child.file_type()?.is_dir() {
                    fs::remove_dir_all(child.path())?;
                } else {
                    fs::remove_file(child.path())?;
                }
            }
            continue;
        }
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    for affected in Step::ALL {
        if affected == step || affected.closure().contains(&step) {
            log::update(&cfg.work_dir, affected, |d| {
                if affected == step {
                    d.sections.retain(|name, _| {
                        matches!(name.as_str(), "工作区" | "阶段" | "阶段分配" | "下一步")
                            || name.starts_with("配置快照 · ")
                            || (step == Step::Coarse
                                && matches!(name.as_str(), "子步骤 · comparison-q2r" | "子步骤 · comparison-matdyn"))
                    });
                }
                if affected == Step::Coarse && step <= Step::Phonon {
                    d.remove("子步骤 · comparison-q2r");
                    d.remove("子步骤 · comparison-matdyn");
                }
                d.set("阶段", "科学状态", "待计算");
                d.set(
                    "阶段",
                    "状态",
                    if affected == step {
                        "待计算"
                    } else {
                        "上游已更新，待计算"
                    },
                );
                Ok(())
            })?;
        }
    }
    Ok(())
}
/// 原生任务的脚本由唯一提交边界生成，不另建尝试快照。
pub fn native_script(
    _program: &super::config::Program,
    _input: &Path,
    _dir: &Path,
    _launcher: &[String],
    _stem: &str,
    _qe: bool,
    _scf: bool,
) -> Result<()> {
    Ok(())
}
pub(crate) fn slurm_log(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if let Some(stem) = name
        .strip_prefix("slurm-")
        .and_then(|s| s.strip_suffix(".log"))
    {
        return !stem.is_empty()
            && stem
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'));
    }
    let Some((job, suffix)) = name.strip_prefix("slurm-").and_then(|s| s.split_once('.')) else {
        return false;
    };
    !job.is_empty() && job.bytes().all(|c| c.is_ascii_digit()) && matches!(suffix, "out" | "err")
}

pub fn import_legacy(config: &Path, source: &Path) -> Result<()> {
    super::workspace_import::import(config, source)
}
