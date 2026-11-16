//! 作业外的原生执行边界：一份脚本、一次调用、一次明确的调度回执。
use super::{
    budget,
    config::Config,
    process::ExecutionReport,
    slurm,
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
pub struct Paused;
impl std::fmt::Display for Paused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("native_pending: 请在作业结束后再次执行同一公开命令 -s")
    }
}
impl std::error::Error for Paused {}
pub fn paused(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Paused>().is_some()
}
pub(crate) fn is_log(path: &Path) -> bool {
    if super::workspace::slurm_log(path) {
        return true;
    }
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("slurm-") && n.ends_with(".log"))
    {
        return true;
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("native-"))
        .and_then(|n| n.rsplit_once('-'))
        .and_then(|(_, tail)| tail.split_once('.'))
        .is_some_and(|(job, ext)| {
            !job.is_empty()
                && job.bytes().all(|c| c.is_ascii_digit())
                && matches!(ext, "out" | "err")
        })
}
pub(crate) fn empty_execution_directory(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !is_log(&entry.path()) && !entry.path().extension().is_some_and(|e| e == "jobs") {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub id: String,
    pub key: String,
    pub generation: String,
    pub directory: PathBuf,
    pub command: Vec<String>,
    pub stem: String,
    pub script: Identity,
    pub inputs: Vec<Identity>,
    pub sources: Vec<Identity>,
    pub state: String,
    pub job: Option<u64>,
    pub reservation: Option<String>,
    #[serde(default)]
    pub attached: bool,
    pub exit_status: Option<i32>,
    pub duration_secs: f64,
    pub stdout: Option<Identity>,
    pub stderr: Option<Identity>,
}

struct Session {
    cfg: Config,
    generation: String,
    submit: bool,
    resume: bool,
    consumed: Vec<(PathBuf, String)>,
    allocated: bool,
}
thread_local! { static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) }; }
pub fn enabled() -> bool {
    SESSION.with(|s| s.borrow().is_some())
}
pub fn retry_enabled() -> bool {
    SESSION.with(|s| s.borrow().as_ref().is_some_and(|s| s.resume))
}
pub fn scoped<T>(
    cfg: &Config,
    generation: &str,
    submit: bool,
    resume: bool,
    run: impl FnOnce() -> Result<T>,
) -> Result<T> {
    scoped_mode(cfg, generation, submit, resume, false, run)
}

/// 声子整阶段已登记一个分配，子调用不再提交或预留资源。
pub fn scoped_allocated<T>(
    cfg: &Config,
    generation: &str,
    resume: bool,
    run: impl FnOnce() -> Result<T>,
) -> Result<T> {
    scoped_mode(cfg, generation, false, resume, true, run)
}

fn scoped_mode<T>(
    cfg: &Config,
    generation: &str,
    submit: bool,
    resume: bool,
    allocated: bool,
    run: impl FnOnce() -> Result<T>,
) -> Result<T> {
    ensure!(!enabled(), "input_conflict: 原生执行上下文不能嵌套");
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            SESSION.with(|s| *s.borrow_mut() = None);
        }
    }
    SESSION.with(|s| {
        *s.borrow_mut() = Some(Session {
            cfg: cfg.clone(),
            generation: generation.into(),
            submit,
            resume,
            consumed: vec![],
            allocated,
        })
    });
    let _guard = Guard;
    let result = run();
    if result.is_ok() || result.as_ref().err().is_some_and(paused) {
        validate_consumed()?;
    }
    result
}
fn task_step(cfg: &Config, directory: &Path) -> Result<super::stages::Step> {
    super::stages::Step::ALL
        .into_iter()
        .find(|s| directory.starts_with(s.directory(cfg)))
        .context("configuration: 原生任务不属于八阶段计算目录")
}
fn task_label(cfg: &Config, task: &Task) -> String {
    if let Some(group) = &task.group {
        format!("方法 · {group}")
    } else {
        format!(
            "任务 · {} / {}",
            super::readable_state::relative(&cfg.work_dir, &task.directory),
            task.stem
        )
    }
}
fn state_text(state: &str) -> &str {
    match state {
        "prepared" => "已准备",
        "submission_pending" => "提交中",
        "submitted" => "已提交",
        "unknown" => "提交未知",
        "running" => "运行中",
        "awaiting_validation" => "待检查",
        "validated" => "已完成",
        "failed" => "失败",
        s => s,
    }
}
fn state_code(state: &str) -> &str {
    match state {
        "已准备" => "prepared",
        "提交中" => "submission_pending",
        "已提交" => "submitted",
        "提交未知" => "unknown",
        "运行中" => "running",
        "待检查" => "awaiting_validation",
        "已完成" => "validated",
        "失败" => "failed",
        s => s,
    }
}
fn optional_identity(path: &Path) -> Identity {
    Identity::capture(path).unwrap_or_else(|_| Identity {
        path: path.into(),
        target: path.into(),
        sha256: String::new(),
        bytes: 0,
    })
}
fn persist(cfg: &Config, task: &Task) -> Result<()> {
    use super::readable_state as log;
    let step = task_step(cfg, &task.directory)?;
    let label = task_label(cfg, task);
    log::set_job(&cfg.work_dir, step, &label, task.job)?;
    log::update(&cfg.work_dir, step, |doc| {
        if task.group.is_none() {
            doc.remove(&label);
        } else {
            for key in ["退出码", "预算关联"] {
                if let Some(fields) = doc.sections.get_mut(&label) {
                    fields.remove(key);
                }
            }
        }
        for (key, value) in [
            ("提交标识", task.id.clone()),
            ("批次", task.generation.clone()),
            ("状态", state_text(&task.state).into()),
            ("计算目录", log::relative(&cfg.work_dir, &task.directory)),
            ("脚本", log::relative(&cfg.work_dir, &task.script.path)),
            ("输出名称", task.stem.clone()),
            (
                "运行命令",
                task.command
                    .iter()
                    .map(|s| shell_words::quote(s))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            ("用时秒", format!("{:.3}", task.duration_secs)),
            ("预算已关联", task.attached.to_string()),
        ] {
            doc.set(&label, key, value);
        }
        if let Some(token) = &task.reservation {
            doc.set(&label, "预算关联", token);
        }
        if let Some(exit) = task.exit_status {
            doc.set(&label, "退出码", exit);
        }
        for (i, input) in task.inputs.iter().enumerate() {
            doc.set(
                &label,
                &format!("科学输入{}", i + 1),
                log::relative(&cfg.work_dir, &input.path),
            );
        }
        doc.set(
            &label,
            "科学输出",
            log::relative(
                &cfg.work_dir,
                &task.directory.join(format!("{}.out", task.stem)),
            ),
        );
        Ok(())
    })
}
/// 从阶段日志和任务号表重建当前任务，不读取独立任务仓库。
pub fn tasks(cfg: &Config) -> Result<Vec<Task>> {
    use super::{readable_state as log, stages::Step};
    let mut result = Vec::new();
    for step in Step::ALL {
        let doc = log::read(&cfg.work_dir, step)?;
        let jobs = log::jobs(&cfg.work_dir, step)?;
        for (name, fields) in &doc.sections {
            if !(name.starts_with("任务 · ") || name.starts_with("方法 · ")) {
                continue;
            }
            let field = |key: &str| {
                fields.get(key).map(String::as_str).with_context(|| {
                    format!(
                        "data_invalid: {} 缺少 {key}",
                        log::path(&cfg.work_dir, step).unwrap().display()
                    )
                })
            };
            let directory = log::resolve(&cfg.work_dir, field("计算目录")?);
            let stem = field("输出名称")?.to_owned();
            let output = directory.join(format!("{stem}.out"));
            let state = state_code(field("状态")?);
            let job = jobs.get(name).copied();
            let state = if state == "submission_pending" && job.is_some() {
                "submitted"
            } else {
                state
            };
            let id = field("提交标识")?.to_owned();
            let inputs = fields
                .iter()
                .filter(|(k, _)| k.starts_with("科学输入"))
                .map(|(_, v)| optional_identity(&log::resolve(&cfg.work_dir, v)))
                .collect();
            result.push(Task {
                schema_version: 3,
                group: name.strip_prefix("方法 · ").map(str::to_owned),
                key: id.clone(),
                id,
                generation: field("批次")?.into(),
                directory,
                command: shell_words::split(field("运行命令")?)?,
                stem,
                script: optional_identity(&log::resolve(&cfg.work_dir, field("脚本")?)),
                inputs,
                sources: Vec::new(),
                state: state.into(),
                job,
                reservation: fields.get("预算关联").cloned(),
                attached: fields.get("预算已关联").is_some_and(|v| v == "true"),
                exit_status: fields.get("退出码").map(|v| v.parse()).transpose()?,
                duration_secs: field("用时秒")?.parse()?,
                stdout: output.is_file().then(|| optional_identity(&output)),
                stderr: output.is_file().then(|| optional_identity(&output)),
            });
        }
    }
    Ok(result)
}
pub fn group_task(cfg: &Config, group: &str) -> Result<Option<Task>> {
    Ok(tasks(cfg)?
        .into_iter()
        .find(|t| t.group.as_deref() == Some(group)))
}
pub fn active_task(cfg: &Config) -> Result<Option<Task>> {
    Ok(tasks(cfg)?
        .into_iter()
        .filter(|t| t.group.is_none())
        .max_by(|a, b| a.id.cmp(&b.id)))
}
pub fn check_groups(cfg: &Config) -> Result<()> {
    for group in ["iso", "aniso"] {
        if let Some(task) = group_task(cfg, group)? {
            let status = task_status(&task)?;
            ensure!(
                !matches!(status["state"].as_str(), Some("running" | "unknown")),
                "scheduler_unknown: {group} 原生作业仍活动或提交未知"
            );
        }
    }
    Ok(())
}
/// 日志丢失不等于作业退出：任务号表仍须通过调度器确认终态。
pub(crate) fn check_orphan_jobs(cfg: &Config) -> Result<()> {
    use super::{readable_state as log, stages::Step};
    for step in Step::ALL {
        let doc = log::read(&cfg.work_dir, step)?;
        let jobs_path = log::jobs_path(&cfg.work_dir, step)?;
        for (name, job) in log::jobs(&cfg.work_dir, step)? {
            if doc.get(&name, "状态").is_some() {
                continue;
            }
            let accounting = slurm::accounting(job).with_context(|| {
                format!(
                    "scheduler_unknown: {} 中作业 {job} 的阶段记录缺失，无法确认终态，禁止覆盖",
                    jobs_path.display()
                )
            })?;
            ensure!(
                accounting.terminal,
                "scheduler_unknown: {} 中作业 {job} 的阶段记录缺失且作业仍活动，禁止覆盖",
                jobs_path.display()
            );
            log::set_job(&cfg.work_dir, step, &name, None)?;
        }
    }
    Ok(())
}
/// 分配内执行也必须确认其他原生作业已经退出，不能覆盖另一份活动现场。
pub fn check_other_allocations(cfg: &Config) -> Result<()> {
    let own = std::env::var("SLURM_JOB_ID")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    for task in tasks(cfg)?.into_iter().filter(|t| t.group.is_none()) {
        if own.is_some() && task.job == own {
            continue;
        }
        let status = task_status(&task)?;
        ensure!(
            !matches!(status["state"].as_str(), Some("running" | "unknown")),
            "scheduler_unknown: {} 的原生作业仍活动或提交未知，禁止覆盖",
            task.directory.display()
        );
    }
    Ok(())
}
pub fn groups_status(cfg: &Config) -> Result<serde_json::Value> {
    let mut value = serde_json::Map::new();
    for group in ["iso", "aniso"] {
        if let Some(t) = group_task(cfg, group)? {
            value.insert(group.into(), task_status(&t)?);
        }
    }
    Ok(value.into())
}
pub fn reject(cfg: &Config, generation: &str) -> Result<()> {
    for mut t in tasks(cfg)? {
        if t.generation == generation && t.state == "awaiting_validation" {
            t.state = "failed".into();
            persist(cfg, &t)?;
        }
    }
    Ok(())
}
/// 科学程序退出后仍需核对它应交付的文件；显式恢复只补缺少的子任务。
pub(crate) fn require_outputs(
    cfg: &Config,
    dir: &Path,
    stem: &str,
    outputs: &[PathBuf],
) -> Result<()> {
    if outputs.iter().all(|p| p.is_file()) {
        return Ok(());
    }
    if let Some(mut task) = tasks(cfg)?.into_iter().find(|t| {
        t.directory == dir
            && matches_stem(t, stem)
            && matches!(t.state.as_str(), "validated" | "awaiting_validation")
    }) {
        ensure!(
            retry_enabled(),
            "data_invalid: {stem} 缺少科学输出；请使用 --resume 恢复此阶段"
        );
        task.state = "failed".into();
        persist(cfg, &task)?;
    }
    Ok(())
}
pub fn accept(cfg: &Config, generation: &str) -> Result<()> {
    for mut t in tasks(cfg)? {
        if t.generation == generation && t.state == "awaiting_validation" {
            t.state = "validated".into();
            persist(cfg, &t)?;
        }
    }
    Ok(())
}
fn verify(task: &Task) -> Result<()> {
    if task.state == "prepared" {
        ensure!(
            task.script.path.is_file(),
            "data_invalid: 缺少待提交脚本 {}",
            task.script.path.display()
        );
        for input in &task.inputs {
            ensure!(
                input.path.is_file(),
                "data_invalid: 缺少本次科学输入 {}",
                input.path.display()
            );
        }
    }
    Ok(())
}
fn log_paths(task: &Task) -> Result<(PathBuf, PathBuf)> {
    let p = task.directory.join(format!("{}.out", task.stem));
    Ok((p.clone(), p))
}
fn capture_logs(task: &mut Task) -> Result<()> {
    let (out, err) = log_paths(task)?;
    task.stdout = Some(Identity::capture(&out)?);
    task.stderr = Some(Identity::capture(&err)?);
    Ok(())
}
pub fn status(cfg: &Config) -> Result<Option<serde_json::Value>> {
    active_task(cfg)?.as_ref().map(task_status).transpose()
}
pub fn task_status(task: &Task) -> Result<serde_json::Value> {
    let state = if matches!(
        task.state.as_str(),
        "submitted" | "submission_pending" | "unknown" | "running"
    ) {
        if let Some(job) = task.job {
            match slurm::accounting(job) {
                Ok(a) if a.terminal => {
                    if a.state == "COMPLETED" {
                        "awaiting_validation"
                    } else {
                        "failed"
                    }
                }
                Ok(_) => "running",
                Err(_) => "unknown",
            }
        } else {
            "unknown"
        }
    } else {
        &task.state
    };
    Ok(
        serde_json::json!({"state":state,"job":task.job,"script":task.script.path,"directory":task.directory}),
    )
}
/// 只恢复旧版本门误拒绝的成功拟合；完整模型核验仍由正常 Wannier 路径完成。
fn recover_epw6_version_rejection(cfg: &Config) -> Result<()> {
    use super::{readable_state as log, stages::Step};
    let doc = log::read(&cfg.work_dir, Step::Wannier)?;
    if doc.get("问题", "原因") != Some("environment_incompatible: 缺少 EPW 5.9 运行证据") {
        return Ok(());
    }
    for mut task in tasks(cfg)?.into_iter().filter(|t| t.directory == Step::Wannier.directory(cfg)
        && matches_stem(t, "epw_wannier") && t.state == "failed" && t.exit_status == Some(0)) {
        let output = task.directory.join(format!("{}.out", task.stem));
        let text = fs::read_to_string(&output)?;
        if crate::interpolation::native::runtime_version(&text)? != "6.0" { continue; }
        crate::interpolation::native::checked_log(&text, 0)?;
        ensure!(!task.inputs.is_empty() && task.inputs.iter().all(|i| i.path.is_file()), "data_invalid: 误拒绝恢复缺少原生输入");
        let job = task.job.context("scheduler_unknown: 误拒绝恢复缺少作业号")?;
        let observed = slurm::accounting(job)?;
        ensure!(observed.terminal && observed.state == "COMPLETED",
            "scheduler_unknown: 误拒绝恢复需要真实 COMPLETED 作业");
        task.state = "awaiting_validation".into();
        persist(cfg, &task)?;
    }
    Ok(())
}

pub fn reconcile(cfg: &Config, resume: bool) -> Result<()> {
    recover_epw6_version_rejection(cfg)?;
    for mut task in tasks(cfg)?.into_iter().filter(|t| t.group.is_none()) {
        if matches!(
            task.state.as_str(),
            "submitted" | "submission_pending" | "unknown" | "running"
        ) {
            reconcile_task(cfg, resume, &mut task)?;
        }
    }
    Ok(())
}
pub fn reconcile_task(cfg: &Config, _resume: bool, task: &mut Task) -> Result<()> {
    if matches!(task.state.as_str(), "unknown" | "submission_pending") && task.job.is_none() {
        anyhow::bail!(
            "scheduler_unknown: {} 提交回执未知；请核对 Slurm 后恢复",
            task.directory.display()
        );
    }
    if matches!(
        task.state.as_str(),
        "submitted" | "submission_pending" | "unknown" | "running"
    ) {
        let job = task.job.context("scheduler_unknown: 缺少 Slurm 作业号")?;
        let accounting = slurm::accounting(job)?;
        ensure!(
            accounting.terminal,
            "scheduler_unknown: 原生任务 {job} 仍为 {}，禁止重复提交",
            accounting.state
        );
        let text = slurm::exit_code_record(job)?;
        let rows: Vec<_> = text
            .lines()
            .filter_map(|l| l.trim().split_once('|'))
            .filter(|(id, _)| *id == job.to_string())
            .collect();
        ensure!(
            rows.len() == 1,
            "scheduler_unknown: 任务 {job} 退出码缺失或不唯一"
        );
        let (code, signal) = rows[0]
            .1
            .trim_end_matches('|')
            .split_once(':')
            .context("scheduler_unknown: ExitCode 格式无效")?;
        let code: i32 = code.parse()?;
        let signal: i32 = signal.parse()?;
        ensure!(code >= 0 && signal >= 0, "scheduler_unknown: ExitCode 为负");
        task.exit_status = Some(if signal == 0 { code } else { 128 + signal });
        task.duration_secs = accounting.node_seconds as f64 / accounting.nodes.max(1) as f64;
        if let Some(token) = &task.reservation {
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                // 另一方法的结算可能已同时结算本任务；不能重新消费已撤下的 token。
                if l.has_unsettled_token(token) {
                    budget::attach_once(l, token, job)?;
                }
                budget::reconcile(l)
            })?;
            task.attached = true;
        }
        task.state = if accounting.state == "COMPLETED" && task.exit_status == Some(0) {
            "awaiting_validation"
        } else {
            "failed"
        }
        .into();
        let output = task.directory.join(format!("{}.out", task.stem));
        if output.is_file() {
            capture_logs(task)?;
        } else {
            task.state = "failed".into();
        }
        if task.directory.is_dir() {
            store::atomic_bytes(
                &task.directory.join(format!("{}.exitcode", task.stem)),
                task.exit_status.unwrap().to_string().as_bytes(),
            )?;
        }
        persist(cfg, task)?;
    }
    Ok(())
}
fn task_key(generation: &str, work: &Path, _command: &[String], stem: &str) -> Result<String> {
    Ok(format!("{generation}:{}:{stem}", work.display()))
}
// 旧记录只匹配同一输入，不把目录内另一次 EPW 调用误认成当前任务。
fn matches_stem(task: &Task, stem: &str) -> bool {
    task.stem == stem || (super::scientific_logs::legacy_stem(stem) == Some(task.stem.as_str())
        && task.inputs.iter().any(|i| i.path.file_stem().and_then(|s| s.to_str()) == Some(stem)))
}

pub fn continuing(work: &Path, stem: &str) -> Result<bool> {
    SESSION.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else {
            return Ok(false);
        };
        Ok(tasks(&s.cfg)?.iter()
            .filter(|t| t.generation == s.generation && t.directory == work && matches_stem(t, stem))
            .min_by_key(|t| t.stem != stem)
            .is_some_and(|t| t.state != "failed"
                && (!matches!(t.state.as_str(), "validated" | "awaiting_validation")
                    || (t.inputs.iter().all(|i| i.path.is_file())
                        && work.join(format!("{}.out", t.stem)).is_file()))))
    })
}
pub fn continuing_directory(work: &Path) -> Result<bool> {
    SESSION.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else {
            return Ok(false);
        };
        Ok(tasks(&s.cfg)?.iter().any(|t| {
            t.generation == s.generation
                && t.directory == work
                && t.state != "failed"
                && t.inputs.iter().all(|i| i.path.is_file())
        }))
    })
}
pub fn failed(work: &Path) -> Result<bool> {
    SESSION.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else {
            return Ok(false);
        };
        Ok(tasks(&s.cfg)?
            .iter()
            .any(|t| t.generation == s.generation && t.directory == work && t.state == "failed"))
    })
}
pub fn retire(work: &Path) -> Result<()> {
    SESSION.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else { return Ok(()) };
        for t in tasks(&s.cfg)?
            .into_iter()
            .filter(|t| t.directory.starts_with(work))
        {
            let step = task_step(&s.cfg, &t.directory)?;
            let label = task_label(&s.cfg, &t);
            super::readable_state::set_job(&s.cfg.work_dir, step, &label, None)?;
            super::readable_state::update(&s.cfg.work_dir, step, |d| {
                d.remove(&label);
                Ok(())
            })?;
        }
        Ok(())
    })
}
pub fn retire_stage(cfg: &Config, step: super::stages::Step) -> Result<()> {
    for name in super::readable_state::jobs(&cfg.work_dir, step)?
        .keys()
        .filter(|name| name.as_str() != "阶段分配")
    {
        super::readable_state::set_job(&cfg.work_dir, step, name, None)?;
    }
    for t in tasks(cfg)?
        .into_iter()
        .filter(|t| t.directory.starts_with(step.directory(cfg)))
    {
        let label = task_label(cfg, &t);
        super::readable_state::set_job(&cfg.work_dir, step, &label, None)?;
        super::readable_state::update(&cfg.work_dir, step, |d| {
            d.remove(&label);
            Ok(())
        })?;
    }
    Ok(())
}
fn validate_consumed() -> Result<()> {
    let consumed = SESSION.with(|s| {
        s.borrow_mut()
            .as_mut()
            .map(|s| (s.cfg.clone(), std::mem::take(&mut s.consumed)))
    });
    if let Some((cfg, consumed)) = consumed {
        for (directory, stem) in consumed {
            for mut t in tasks(&cfg)?
                .into_iter()
                .filter(|t| t.directory == directory && t.stem == stem)
            {
                if t.exit_status == Some(0) {
                    t.state = "validated".into();
                    persist(&cfg, &t)?;
                }
            }
        }
    }
    Ok(())
}
pub fn execute(command: &[String], work: &Path, stem: &str) -> Result<ExecutionReport> {
    ensure!(!command.is_empty(), "configuration: 空原生命令");
    let (cfg, generation, submit, allocated) = SESSION.with(|s| {
        let s = s.borrow();
        let s = s.as_ref().expect("原生上下文");
        (s.cfg.clone(), s.generation.clone(), s.submit, s.allocated)
    });
    let previous = tasks(&cfg)?
        .into_iter()
        .filter(|t| t.generation == generation && t.directory == work && matches_stem(t, stem))
        .min_by_key(|t| t.stem != stem);
    let effective_stem = previous.as_ref().filter(|t| t.state != "failed")
        .map(|t| t.stem.clone()).unwrap_or_else(|| stem.to_owned());
    let stem = effective_stem.as_str();
    if let Some(t) = &previous {
        ensure!(
            t.state != "failed" || retry_enabled(),
            "execution_failed: {} / {stem} 上次失败；修复后使用 --resume -s",
            work.display()
        );
        if matches!(t.state.as_str(), "validated" | "awaiting_validation")
            && t.exit_status == Some(0)
            && t.directory.join(format!("{stem}.out")).is_file()
        {
            SESSION.with(|s| {
                s.borrow_mut()
                    .as_mut()
                    .unwrap()
                    .consumed
                    .push((work.into(), stem.into()))
            });
            let (stdout_path, stderr_path) = log_paths(t)?;
            return Ok(ExecutionReport {
                exit_status: t.exit_status,
                stdout_path,
                stderr_path,
                duration_secs: t.duration_secs,
            });
        }
        ensure!(
            !matches!(
                t.state.as_str(),
                "submitted" | "submission_pending" | "unknown" | "running"
            ),
            "scheduler_unknown: {} / {stem} 已有活动或未知提交",
            work.display()
        );
    }
    validate_consumed()?;
    let key = task_key(&generation, work, command, stem)?;
    let mut task = match previous.filter(|t| t.state == "prepared") {
        Some(t) => t,
        None => prepare(&cfg, &generation, &key, command, work, stem)?,
    };
    if allocated {
        task.job = Some(
            std::env::var("SLURM_JOB_ID")
                .context("allocation: 分配内子任务缺 Slurm 作业号")?
                .parse()?,
        );
        task.state = "running".into();
        persist(&cfg, &task)?;
        let start = std::time::Instant::now();
        let status = Command::new("/bin/bash")
            .arg(&task.script.path)
            .current_dir(work)
            .status()?;
        task.duration_secs = start.elapsed().as_secs_f64();
        task.exit_status = status.code();
        let capture = capture_logs(&mut task);
        task.state = if status.success() && capture.is_ok() {
            "awaiting_validation"
        } else {
            "failed"
        }
        .into();
        store::atomic_bytes(
            &work.join(format!("{stem}.exitcode")),
            task.exit_status.unwrap_or(128).to_string().as_bytes(),
        )?;
        persist(&cfg, &task)?;
        capture?;
        SESSION.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .consumed
                .push((work.into(), stem.into()))
        });
        let (stdout_path, stderr_path) = log_paths(&task)?;
        return Ok(ExecutionReport {
            exit_status: task.exit_status,
            stdout_path,
            stderr_path,
            duration_secs: task.duration_secs,
        });
    }
    if submit {
        submit_task(&cfg, &mut task)?;
    }
    println!(
        "script: {}\ndirectory: {}\nworkflow_status: {}",
        task.script.path.display(),
        work.display(),
        task.state
    );
    Err(Paused.into())
}

/// 两个独立方法与旧逐任务入口共用同一提交／预算边界。
pub fn submit_task(cfg: &Config, task: &mut Task) -> Result<()> {
    ensure!(
        task.state == "prepared",
        "scheduler_unknown: 任务已提交或提交状态未知"
    );
    verify(task)?;
    task.reservation = Some(budget::with_ledger(
        &cfg.budget.ledger,
        &cfg.budget.campaign_id,
        |l| {
            budget::reconcile(l)?;
            budget::reserve_workflow(l, &format!("native:{}", task.id))
        },
    )?);
    task.state = "submission_pending".into();
    persist(cfg, task)?;
    let out = Command::new("sbatch")
        .arg("--parsable")
        .arg(&task.script.path)
        .current_dir(&task.directory)
        .output();
    let receipt = out
        .context("scheduler_unknown: 无法调用 sbatch")
        .and_then(|o| {
            ensure!(
                o.status.success(),
                "scheduler_unknown: sbatch 失败：{}",
                String::from_utf8_lossy(&o.stderr)
            );
            slurm::parse_job_id(String::from_utf8_lossy(&o.stdout).trim())
        });
    match receipt {
        Ok(job) => {
            task.job = Some(job.job_id);
            task.state = "submitted".into();
            persist(cfg, task)?;
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                budget::attach_once(l, task.reservation.as_ref().unwrap(), job.job_id)
            })?;
            task.attached = true;
            persist(cfg, task)?;
            println!("job_id: {}", job.job_id);
        }
        Err(e) => {
            task.state = "unknown".into();
            persist(cfg, task)?;
            return Err(e);
        }
    }
    Ok(())
}

pub fn finish_group(cfg: &Config, task: &mut Task, complete: bool) -> Result<()> {
    task.state = if complete { "validated" } else { "failed" }.into();
    persist(cfg, task)
}

/// 方法作业自己交回已退出的原生进程；分配仍运行，不能伪造调度器终态。
pub fn allocated_group_exit(cfg: &Config, task: &mut Task, exit: i32) -> Result<()> {
    verify(task)?;
    let job: u64 = std::env::var("SLURM_JOB_ID")
        .context("allocation: 方法收尾需要作业分配")?
        .parse()?;
    ensure!(
        task.group.is_some() && task.job.is_none_or(|j| j == job),
        "input_conflict: 方法收尾作业身份不同"
    );
    task.job = Some(job);
    task.exit_status = Some(exit);
    capture_logs(task)?;
    task.state = if exit == 0 {
        "awaiting_validation"
    } else {
        "failed"
    }
    .into();
    store::atomic_bytes(
        &task.directory.join("epw.exitcode"),
        exit.to_string().as_bytes(),
    )?;
    persist(cfg, task)
}

/// 手动 -r 已处在一个分配中，方法脚本在本分配顺序执行，不额外提交。
pub fn execute_group(cfg: &Config, task: &mut Task) -> Result<()> {
    ensure!(
        task.state == "prepared",
        "scheduler_unknown: 方法尚未处于可执行状态"
    );
    task.job = Some(std::env::var("SLURM_JOB_ID")?.parse()?);
    task.state = "running".into();
    persist(cfg, task)?;
    let start = std::time::Instant::now();
    let status = Command::new("/bin/bash")
        .arg(&task.script.path)
        .current_dir(&task.directory)
        .status()?;
    task.duration_secs = start.elapsed().as_secs_f64();
    allocated_group_exit(cfg, task, status.code().unwrap_or(128))
}

pub fn prepare_group(
    cfg: &Config,
    generation: &str,
    group: &str,
    command: &[String],
    work: &Path,
) -> Result<Task> {
    ensure!(
        matches!(group, "iso" | "aniso"),
        "configuration: 无效方法分组"
    );
    let key = task_key(generation, work, command, "epw")?;
    prepare_mode(cfg, generation, &key, command, work, "epw", Some(group))
}

pub(super) fn double_quote(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        if matches!(c, '\\' | '"' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

fn readable_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/_-.:=,+%@".contains(&c))
    {
        value.to_owned()
    } else {
        double_quote(value)
    }
}

fn prepare(
    cfg: &Config,
    generation: &str,
    key: &str,
    command: &[String],
    work: &Path,
    stem: &str,
) -> Result<Task> {
    prepare_mode(cfg, generation, key, command, work, stem, None)
}
fn prepare_mode(
    cfg: &Config,
    generation: &str,
    _key: &str,
    command: &[String],
    work: &Path,
    stem: &str,
    group: Option<&str>,
) -> Result<Task> {
    // 历史任务键／argv 保持原样，仅新脚本使用运行环境中的启动器。
    let invocation = qf_runner::plain_scientific_argv(command);
    let id = store::token();
    let q = readable_quote;
    // QE/EPW 使用 stdin；Wannier 等种子参数型程序保留原接口。
    let input_arg = invocation
        .windows(2)
        .position(|p| matches!(p[0].as_str(), "-inp" | "-in"));
    let purpose = if stem == "epw" {
        input_arg
            .and_then(|i| Path::new(&invocation[i + 1]).file_stem())
            .and_then(|n| n.to_str())
            .unwrap_or(stem)
    } else {
        stem
    };
    ensure!(
        !work.to_string_lossy().contains(['\n', '\r']),
        "configuration: 工作目录含换行"
    );
    ensure!(
        purpose
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c)),
        "configuration: 脚本功能名称含特殊字符"
    );
    let log = format!("slurm-{purpose}.log");
    let mut text = slurm::script_header(cfg, work, &format!("epw-{purpose}"), &log, &log);
    let mut sources = vec![Identity::capture(&cfg.structure)?];
    sources.extend(
        cfg.pseudos
            .values()
            .map(|p| Identity::capture(p))
            .collect::<Result<Vec<_>>>()?,
    );
    if let Some(p) = &cfg.kpath {
        sources.push(Identity::capture(p)?);
    }
    if let Some(env) = &cfg.programs.environment_script {
        sources.push(Identity::capture(env)?);
        text.push_str(&format!(
            "source {} --force\n",
            double_quote(&env.to_string_lossy())
        ));
    }
    // 固定显式科学程序和自定义启动器；mpirun 已改为作业环境 PATH 查找。
    for arg in &invocation {
        if Path::new(arg).is_absolute()
            && Path::new(arg).is_file()
            && fs::metadata(arg)?.permissions().mode() & 0o111 != 0
        {
            sources.push(Identity::capture(Path::new(arg))?);
        }
    }
    let mut inputs = vec![];
    for pair in command.windows(2) {
        if matches!(pair[0].as_str(), "-inp" | "-in") {
            let path = Path::new(&pair[1]);
            inputs.push(Identity::capture(&if path.is_absolute() {
                path.into()
            } else {
                work.join(path)
            })?);
        }
    }
    if inputs.is_empty() {
        if let Some(seed) = command.last() {
            let win = work.join(format!("{seed}.win"));
            if win.is_file() {
                inputs.push(Identity::capture(&win)?);
            }
        }
    }
    ensure!(!inputs.is_empty(), "configuration: 原生调用缺少固定输入");
    text.push_str(
        &invocation
            .iter()
            .enumerate()
            .filter(|(i, _)| input_arg.is_none_or(|n| *i != n && *i != n + 1))
            .map(|(_, s)| q(s))
            .collect::<Vec<_>>()
            .join(" "),
    );
    if let Some(i) = input_arg {
        text.push_str(&format!(" < {}", q(&invocation[i + 1])));
    }
    text.push_str(&format!(" > {} 2>&1", q(&format!("{stem}.out"))));
    text.push('\n');
    let script = work.join(format!("{purpose}.sh"));
    store::atomic_bytes(&script, text.as_bytes())?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    let task = Task {
        schema_version: 3,
        group: group.map(str::to_owned),
        key: id.clone(),
        id,
        generation: generation.into(),
        directory: work.into(),
        command: invocation,
        stem: stem.into(),
        script: Identity::capture(&script)?,
        inputs,
        sources,
        state: "prepared".into(),
        job: None,
        reservation: None,
        attached: false,
        exit_status: None,
        duration_secs: 0.,
        stdout: None,
        stderr: None,
    };
    persist(cfg, &task)?;
    Ok(task)
}
