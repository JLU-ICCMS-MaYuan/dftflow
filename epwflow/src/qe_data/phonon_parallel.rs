//! index／coords 的独立分配脚本与依赖；科学执行复用现有 PH 和归集器。
use super::{
    phonon,
    phonon_state::{PhononState, QPointState, TaskStatus},
    qe_config::QeSettings,
};
use crate::foundation::{
    config::{Config, PhMode},
    readable_state as log, slurm,
    stages::{self, Step},
    store,
};
use anyhow::{ensure, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn enabled(cfg: &Config) -> bool {
    cfg.phonon.mode != PhMode::Nosplit && !crate::foundation::phonon_import::bound(cfg)
}

pub fn state_path(cfg: &Config) -> PathBuf {
    Step::Phonon.directory(cfg).join("phonon-qpoints.json")
}

struct Observation {
    state: String,
    terminal: bool,
}

/// 未启用 accounting 时，终态仍由既有 filetxt 回退核验；
/// 排队作业直接读取控制器，不伪造终态或计费时间。
fn observe(job: u64) -> Result<Observation> {
    match slurm::accounting(job) {
        Ok(a) => Ok(Observation {
            state: a.state,
            terminal: a.terminal,
        }),
        Err(error) => {
            let out = Command::new("scontrol")
                .args(["show", "job", "-o", &job.to_string()])
                .output()?;
            if !out.status.success() {
                return Err(error);
            }
            let state = active_controller_state(job, &String::from_utf8(out.stdout)?)
                .with_context(|| format!("scheduler_unknown: accounting failed: {error:#}"))?;
            Ok(Observation {
                state,
                terminal: false,
            })
        }
    }
}

fn active_controller_state(job: u64, text: &str) -> Result<String> {
    let rows = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    ensure!(
        rows.len() == 1,
        "scheduler_unknown: phonon controller job record missing or duplicated"
    );
    let field = |name: &str| -> Result<&str> {
        let fields = rows[0]
            .split_whitespace()
            .filter_map(|s| s.split_once('='))
            .filter(|(key, _)| *key == name)
            .map(|(_, value)| value)
            .collect::<Vec<_>>();
        ensure!(
            fields.len() == 1,
            "scheduler_unknown: phonon controller {name} missing or duplicated"
        );
        Ok(fields[0])
    };
    ensure!(
        field("JobId")?.parse::<u64>()? == job,
        "scheduler_unknown: phonon controller job ID mismatch"
    );
    let state = field("JobState")?;
    ensure!(
        matches!(
            state,
            "PENDING"
                | "RUNNING"
                | "CONFIGURING"
                | "COMPLETING"
                | "SUSPENDED"
                | "REQUEUED"
                | "REQUEUE_FED"
                | "RESIZING"
        ),
        "scheduler_unknown: phonon controller does not prove an active job"
    );
    Ok(state.into())
}

fn script(cfg: &Config, version: &Path, batch: &str, task: &str, path: &Path) -> Result<()> {
    let dir = path
        .parent()
        .context("configuration: phonon script directory missing")?;
    fs::create_dir_all(dir)?;
    let mut body = slurm::script_header(
        cfg,
        dir,
        &format!("epw-ph-{task}"),
        "slurm-phonon.log",
        "slurm-phonon.log",
    );
    if let Some(env) = &cfg.programs.environment_script {
        body.push_str(&crate::foundation::workflow::source_environment(env)?);
    }
    let quote = slurm::shell_quote;
    body.push_str(&format!(
        "exec {} --config {} --phonon-task {} --allocation-id {}\n",
        quote(&std::env::current_exe()?.to_string_lossy()),
        quote(&version.to_string_lossy()),
        quote(task),
        quote(batch)
    ));
    store::atomic_bytes(path, body.as_bytes())
}

pub fn generate(cfg: &Config, version: &Path, batch: &str, state: &PhononState) -> Result<()> {
    ensure!(
        enabled(cfg),
        "configuration: parallel phonon scripts require index or coords mode"
    );
    let root = Step::Phonon.directory(cfg);
    script(cfg, version, batch, "init", &root.join(&state.init.script))?;
    script(
        cfg,
        version,
        batch,
        "collect",
        &root.join(&state.collect.script),
    )?;
    // Γ 的编号恒为 1，先提供可审阅模板；准确列表以 init 的 dyn0 为准。
    if state.qpoints.is_empty() {
        script(cfg, version, batch, "1", &root.join("q/q0001/ph.sh"))?;
    }
    for point in &state.qpoints {
        script(
            cfg,
            version,
            batch,
            &point.global_index.to_string(),
            &root.join(&point.script),
        )?;
    }
    Ok(())
}

pub fn dependency(jobs: &[u64]) -> Result<Option<String>> {
    ensure!(
        jobs.iter().all(|id| *id > 0),
        "data_invalid: invalid phonon dependency ID"
    );
    Ok((!jobs.is_empty()).then(|| {
        format!(
            "afterok:{}",
            jobs.iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(":")
        )
    }))
}

fn task_mut<'a>(
    state: &'a mut PhononState,
    task: &str,
) -> Result<(&'a mut TaskStatus, &'a mut Option<u64>)> {
    match task {
        "init" => Ok((&mut state.init.status, &mut state.init.job_id)),
        "collect" => Ok((&mut state.collect.status, &mut state.collect.job_id)),
        _ => {
            let index: usize = task.parse().context("configuration: invalid phonon task")?;
            let point = state
                .qpoints
                .iter_mut()
                .find(|q| q.global_index == index)
                .context("data_invalid: unknown phonon q point")?;
            Ok((&mut point.status, &mut point.job_id))
        }
    }
}

/// 现有工作区入口也能看到各分配，防止切换模式时遗漏活动 q 作业。
fn record_task(cfg: &Config, task: &str) -> Result<()> {
    let mut state = PhononState::load(&state_path(cfg))?;
    let (status, job) = task_mut(&mut state, task)?;
    let label = format!("Phonon task · {task}");
    log::set_job(&cfg.work_dir, Step::Phonon, &label, *job)?;
    log::update(&cfg.work_dir, Step::Phonon, |doc| {
        doc.set(
            &label,
            "Status",
            if *status == TaskStatus::Submitted && job.is_none() {
                "submission unknown"
            } else {
                status.as_str()
            },
        );
        Ok(())
    })
}

/// 提交前持久化 Submitted/无回执；中断或不明确回执禁止自动重提。
fn submit(cfg: &Config, task: &str, script: &Path, dependencies: &[u64]) -> Result<u64> {
    let path = state_path(cfg);
    PhononState::update(&path, |state| {
        let (status, job) = task_mut(state, task)?;
        ensure!(
            matches!(*status, TaskStatus::Pending | TaskStatus::Failed),
            "scheduler_unknown: phonon task {task} already submitted or completed"
        );
        *status = TaskStatus::Submitted;
        *job = None;
        Ok(())
    })?;
    record_task(cfg, task)?;
    let mut cmd = Command::new("sbatch");
    cmd.args(["--parsable", "--kill-on-invalid-dep=no"]);
    if let Some(dependency) = dependency(dependencies)? {
        cmd.arg(format!("--dependency={dependency}"));
    }
    let out = cmd
        .arg(script)
        .current_dir(script.parent().unwrap())
        .output()
        .context("scheduler_unknown: cannot invoke phonon sbatch; receipt unknown")?;
    ensure!(
        out.status.success(),
        "scheduler_unknown: phonon sbatch failed; receipt unknown: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let job = slurm::parse_job_id(String::from_utf8_lossy(&out.stdout).trim())?;
    ensure!(job.cluster.is_none(), "scheduler_unknown: cross-cluster phonon submissions are unsupported; inspect returned job {}", job.job_id);
    PhononState::update(&path, |state| {
        let (_, stored) = task_mut(state, task)?;
        ensure!(
            stored.is_none_or(|id| id == job.job_id),
            "scheduler_unknown: phonon receipt mismatch"
        );
        *stored = Some(job.job_id);
        Ok(())
    })?;
    record_task(cfg, task)?;
    println!("phonon task: {task}; job_id: {}", job.job_id);
    Ok(job.job_id)
}

fn register_points(cfg: &Config, qe: &QeSettings) -> Result<()> {
    let points = phonon::initialized_points(qe, &cfg.work_dir)?;
    PhononState::update(&state_path(cfg), |state| {
        let actual = points
            .iter()
            .map(|q| QPointState {
                global_index: q.global_index,
                directory: q.directory.clone(),
                crystal: q.crystal,
                local_index: q.local_index,
                status: TaskStatus::Pending,
                job_id: None,
                script: format!("q/{}/ph.sh", q.directory).into(),
            })
            .collect::<Vec<_>>();
        if state.qpoints.is_empty() {
            state.qpoints = actual;
        } else {
            ensure!(
                state.qpoints.len() == actual.len()
                    && state
                        .qpoints
                        .iter()
                        .zip(&actual)
                        .all(|(a, b)| a.global_index == b.global_index
                            && a.local_index == b.local_index
                            && a.directory == b.directory
                            && super::qmap::same_q_modulo_periodic(a.crystal, b.crystal)),
                "input_conflict: initialized phonon q-point list changed"
            );
        }
        Ok(())
    })
}

fn batch(cfg: &Config) -> Result<String> {
    log::read(&cfg.work_dir, Step::Phonon)?
        .get("Parallel phonon", "Batch")
        .map(str::to_owned)
        .context("data_invalid: parallel phonon batch record missing")
}

fn verify_config(cfg: &Config) -> Result<(String, PathBuf)> {
    let doc = log::read(&cfg.work_dir, Step::Phonon)?;
    let version = log::resolve(
        &cfg.work_dir,
        doc.get("Parallel phonon", "Configuration")
            .context("data_invalid: parallel phonon configuration missing")?,
    );
    let old = log::load_config(&version)?;
    ensure!(
        stages::scientific_key(&old, Step::Phonon)? == stages::scientific_key(cfg, Step::Phonon)?,
        "input_conflict: parallel phonon scientific configuration changed; use a new workspace"
    );
    ensure!(cfg.programs.launcher == old.programs.launcher && cfg.programs.ph.args == old.programs.ph.args
        && cfg.programs.environment_script == old.programs.environment_script && cfg.slurm.header == old.slurm.header,
        "input_conflict: parallel phonon batch uses frozen launch settings; restore the original settings or use a new workspace");
    Ok((batch(cfg)?, version))
}

pub(crate) fn check_active(cfg: &Config, own: Option<&str>) -> Result<()> {
    if !enabled(cfg) {
        return Ok(());
    }
    let doc = log::read(&cfg.work_dir, Step::Phonon)?;
    let registered = doc.get("Parallel phonon", "Batch");
    let path = state_path(cfg);
    if registered.is_none() && !path.try_exists()? {
        return Ok(());
    }
    let state = PhononState::load(&path)?;
    if own.is_some() && registered == own {
        return Ok(());
    }
    let tasks = std::iter::once((state.init.status, state.init.job_id))
        .chain(state.qpoints.iter().map(|q| (q.status, q.job_id)))
        .chain(std::iter::once((
            state.collect.status,
            state.collect.job_id,
        )));
    for (status, job) in tasks {
        if matches!(status, TaskStatus::Submitted | TaskStatus::Running) {
            let job =
                job.context("scheduler_unknown: parallel phonon submission receipt missing")?;
            ensure!(observe(job)?.terminal,
                "scheduler_unknown: parallel phonon job {job} is active; workspace modification prohibited");
        }
    }
    Ok(())
}

pub fn status(cfg: &Config) -> Result<()> {
    if !enabled(cfg)
        || (!state_path(cfg).try_exists()?
            && log::read(&cfg.work_dir, Step::Phonon)?
                .get("Parallel phonon", "Batch")
                .is_none())
    {
        return Ok(());
    }
    let state = reconcile(cfg)?;
    println!(
        "Phonon stage: {} mode\n  init: {} (job {:?})\n  q points: {} total",
        state.mode,
        state.init.status.as_str(),
        state.init.job_id,
        state.qpoints.len()
    );
    for q in state.qpoints {
        println!(
            "    {}: {} (job {:?})",
            q.directory,
            q.status.as_str(),
            q.job_id
        );
    }
    println!(
        "  collect: {} (job {:?})",
        state.collect.status.as_str(),
        state.collect.job_id
    );
    Ok(())
}

/// 公开入口；与 nosplit 的整阶段分配完全分离。
pub(crate) fn prepare(cfg: &Config, config: &Path, resume: bool, submit_jobs: bool) -> Result<()> {
    let _operation = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    let root = Step::Phonon.directory(cfg);
    let _submission = log::DirectoryGuard::acquire(&root)?;
    crate::foundation::run_state::ready(&cfg.work_dir)?;
    let path = state_path(cfg);
    let existing = path.try_exists()?;
    let previous = log::read(&cfg.work_dir, Step::Phonon)?;
    ensure!(
        existing || previous.get("Parallel phonon", "Batch").is_none(),
        "data_invalid: phonon-qpoints.json missing; refusing to rebuild state"
    );
    ensure!(
        existing || !resume,
        "data_invalid: --resume requires phonon-qpoints.json"
    );
    let (id, version) = if existing {
        PhononState::load(&path)?;
        verify_config(cfg)?
    } else {
        let id = store::token();
        let version = log::backup_config(config)?;
        (id, version)
    };
    crate::foundation::workspace::check_active(cfg, existing.then_some(id.as_str()))?;
    crate::foundation::parallelism::preflight(cfg, Step::Phonon, false)?;
    if submit_jobs {
        crate::foundation::reuse_files::check_inputs(cfg, Step::Phonon)?;
    }
    if !existing {
        PhononState::new(log::mode(cfg))?.create(&path)?;
        log::update(&cfg.work_dir, Step::Phonon, |doc| {
            doc.set("Parallel phonon", "Batch", &id);
            doc.set(
                "Parallel phonon",
                "Configuration",
                log::relative(&cfg.work_dir, &version),
            );
            doc.set("Stage", "Batch", &id);
            doc.set(
                "Stage",
                "Calculation configuration",
                log::relative(&cfg.work_dir, &version),
            );
            doc.set("Stage", "Status", "prepared");
            Ok(())
        })?;
    }
    if resume {
        recover_state_lock(cfg)?;
    }
    let state = if submit_jobs {
        reconcile(cfg)?
    } else {
        PhononState::load(&path)?
    };
    ensure!(
        state.mode == log::mode(cfg),
        "input_conflict: phonon state mode mismatch"
    );
    // 只为即将提交的任务重写驱动，避免覆盖活动 ph.x 的原生脚本。
    if !submit_jobs {
        ensure!(
            !matches!(
                state.init.status,
                TaskStatus::Submitted | TaskStatus::Running
            ) && state
                .qpoints
                .iter()
                .all(|q| !matches!(q.status, TaskStatus::Submitted | TaskStatus::Running))
                && !matches!(
                    state.collect.status,
                    TaskStatus::Submitted | TaskStatus::Running
                ),
            "scheduler_unknown: parallel phonon scripts are active"
        );
        generate(cfg, &version, &id, &state)?;
        println!(
            "script: {}; not submitted\nq scripts are expanded from the native init dyn0",
            root.join(&state.init.script).display()
        );
        return Ok(());
    }
    ensure!(
        resume
            || (state.init.status != TaskStatus::Failed
                && state.failed_qpoints().next().is_none()
                && state.collect.status != TaskStatus::Failed),
        "execution_failed: parallel phonon task failed; fix the cause and use --resume -s"
    );
    if matches!(state.init.status, TaskStatus::Pending | TaskStatus::Failed) {
        let script_path = root.join(&state.init.script);
        script(cfg, &version, &id, "init", &script_path)?;
        submit(cfg, "init", &script_path, &[])?;
        println!("init submitted; q-point expansion requires its native dyn0");
        return Ok(());
    }
    if state.init.status != TaskStatus::Completed {
        println!("init is active; no duplicate submission");
        return Ok(());
    }
    register_points(cfg, &QeSettings::from_config(cfg)?)?;
    submit_points(cfg, &version, &id, resume)
}

/// scancel 可在短状态事务内终止工作进程；只恢复属于本批已结束作业的锁。
fn recover_state_lock(cfg: &Config) -> Result<()> {
    let path = state_path(cfg);
    let lock = path.with_file_name("phonon-qpoints.json.lock");
    if !lock.try_exists()? {
        return Ok(());
    }
    let owner: serde_json::Value = store::read(&lock.join("owner.json"))?;
    let job = owner["job_id"]
        .as_u64()
        .context("scheduler_unknown: phonon state lock has no confirmed allocation owner")?;
    let state = PhononState::load(&path)?;
    ensure!(
        state.init.job_id == Some(job)
            || state.collect.job_id == Some(job)
            || state.qpoints.iter().any(|q| q.job_id == Some(job)),
        "scheduler_unknown: phonon state lock belongs to an unknown allocation"
    );
    store::recover_finished_lock(&path, job)
}

fn submit_points(cfg: &Config, version: &Path, batch: &str, resume: bool) -> Result<()> {
    let root = Step::Phonon.directory(cfg);
    let state = PhononState::load(&state_path(cfg))?;
    let init = state
        .init
        .job_id
        .context("data_invalid: init job ID missing")?;
    // 已完成的前驱可能已被 Slurm 清除；科学完成回执已满足该依赖。
    let init_dependencies = if state.init.status == TaskStatus::Completed {
        Vec::new()
    } else {
        vec![init]
    };
    for q in &state.qpoints {
        if q.status == TaskStatus::Pending || (resume && q.status == TaskStatus::Failed) {
            let path = root.join(&q.script);
            script(cfg, version, batch, &q.global_index.to_string(), &path)?;
            submit(cfg, &q.global_index.to_string(), &path, &init_dependencies)?;
        }
    }
    let state = PhononState::load(&state_path(cfg))?;
    let mut jobs = state
        .qpoints
        .iter()
        .filter(|q| q.status != TaskStatus::Completed)
        .map(|q| {
            q.job_id
                .context("scheduler_unknown: q-point submission receipt missing")
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        !state.qpoints.is_empty(),
        "data_invalid: no phonon q points"
    );
    if jobs.is_empty() && state.init.status != TaskStatus::Completed {
        jobs.push(init);
    }
    if state.collect.status == TaskStatus::Pending
        || (resume
            && state.collect.status == TaskStatus::Failed
            && state
                .qpoints
                .iter()
                .all(|q| q.status == TaskStatus::Completed))
    {
        let path = root.join(&state.collect.script);
        script(cfg, version, batch, "collect", &path)?;
        PhononState::update(&state_path(cfg), |s| {
            s.collect.dependencies = jobs.clone();
            Ok(())
        })?;
        submit(cfg, "collect", &path, &jobs)?;
    } else if state.collect.status == TaskStatus::Submitted && state.collect.dependencies != jobs {
        let job = state
            .collect
            .job_id
            .context("scheduler_unknown: collect job ID missing")?;
        let dependency = dependency(&jobs)?.unwrap_or_default();
        let out = Command::new("scontrol")
            .args([
                "update",
                &format!("JobId={job}"),
                &format!("Dependency={dependency}"),
            ])
            .output()?;
        ensure!(
            out.status.success(),
            "scheduler_unknown: collect dependency update failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        PhononState::update(&state_path(cfg), |s| {
            s.collect.dependencies = jobs;
            Ok(())
        })?;
    }
    Ok(())
}

/// 每个分配只执行自己的叶子任务；共享锁仅覆盖状态更新。
pub fn allocated(config: &Path, task: &str, id: &str) -> Result<()> {
    let cfg = log::load_config(config)?;
    ensure!(
        enabled(&cfg),
        "configuration: parallel phonon allocation requires split mode"
    );
    let (batch, version) = verify_config(&cfg)?;
    ensure!(
        batch == id && version.canonicalize()? == config.canonicalize()?,
        "input_conflict: stale phonon task script"
    );
    let _config = log::ExecutionConfig::enter(&cfg.work_dir, config);
    let job: u64 = std::env::var("SLURM_JOB_ID")
        .context("allocation: phonon task requires Slurm")?
        .parse()?;
    let observation = slurm::running_allocation(job)?;
    ensure!(
        !observation.terminal && observation.state == "RUNNING",
        "allocation: phonon task allocation is not running"
    );
    crate::foundation::parallelism::preflight(&cfg, Step::Phonon, true)?;
    let path = state_path(&cfg);
    PhononState::update(&path, |state| {
        ensure!(
            state.mode == log::mode(&cfg),
            "input_conflict: phonon state mode mismatch"
        );
        let (status, bound) = task_mut(state, task)?;
        ensure!(
            *status == TaskStatus::Submitted && bound.is_none_or(|bound| bound == job),
            "input_conflict: phonon task already executed or bound to another job"
        );
        *bound = Some(job);
        *status = TaskStatus::Running;
        Ok(())
    })?;
    record_task(&cfg, task)?;
    let generation = if task == "collect" {
        format!("{batch}-ph-{task}")
    } else {
        // 失败 q 的新分配必须从新原生尝试开始；成功 q 不会进入本入口。
        format!("{batch}-ph-{task}-{job}")
    };
    let run = || {
        let qe = QeSettings::from_config(&cfg)?;
        match task {
            "init" => {
                fresh_ph(
                    &cfg,
                    &crate::foundation::layout::path(&cfg.work_dir, "ph_init"),
                    None,
                )?;
                phonon::run_init(
                    &qe,
                    &cfg.work_dir,
                    &cfg.programs.launcher,
                    crate::foundation::stage::Backend::Local,
                )?;
                register_points(&cfg, &qe)?;
            }
            "collect" => {
                register_points(&cfg, &qe)?;
                let state = PhononState::load(&path)?;
                ensure!(
                    state.init.status == TaskStatus::Completed
                        && !state.qpoints.is_empty()
                        && state
                            .qpoints
                            .iter()
                            .all(|q| q.status == TaskStatus::Completed),
                    "execution_failed: collect requires all phonon q points completed"
                );
                super::scheduler::collect_parallel(&cfg, &qe)?;
                crate::foundation::workspace_science::run(&cfg, Step::Phonon)?;
                stages::finish(&cfg, Step::Phonon)?;
                crate::foundation::workspace_science::refresh_post(&cfg, Step::Phonon)?;
            }
            _ => {
                let index: usize = task.parse()?;
                let points = phonon::initialized_points(&qe, &cfg.work_dir)?;
                let point = points
                    .iter()
                    .find(|q| q.global_index == index)
                    .context("data_invalid: q point missing from dyn0")?;
                fresh_ph(
                    &cfg,
                    &crate::foundation::layout::path(&cfg.work_dir, "ph").join(&point.directory),
                    Some(index),
                )?;
                phonon::run_q(
                    &qe,
                    &cfg.work_dir,
                    &cfg.programs.launcher,
                    crate::foundation::stage::Backend::Local,
                    point,
                    index,
                )?;
            }
        }
        Ok(())
    };
    // 所有叶子复用原生任务记录与合并输出；collect 保持后处理局部续算。
    let result = crate::foundation::native_jobs::scoped_allocated(&cfg, &generation, true, run);
    let result = result.and_then(|()| crate::foundation::native_jobs::accept(&cfg, &generation));
    let result = result.and_then(|()| {
        if task == "init" {
            expand_after_init(&cfg, &version, &batch, job)?;
        }
        Ok(())
    });
    // 原生执行器使用相同功能脚本名；离开分配前恢复公开驱动，拒绝旧作业重放。
    let driver = match task {
        "init" => Step::Phonon.directory(&cfg).join("init/ph_init.sh"),
        "collect" => Step::Phonon.directory(&cfg).join("collect/collect.sh"),
        _ => Step::Phonon
            .directory(&cfg)
            .join(format!("q/q{:04}/ph.sh", task.parse::<usize>()?)),
    };
    let restored = script(&cfg, &version, &batch, task, &driver);
    let result = result.and(restored);
    if result.is_err() {
        crate::foundation::native_jobs::reject(&cfg, &generation)?;
    }
    PhononState::update(&path, |state| {
        let (status, bound) = task_mut(state, task)?;
        ensure!(
            *bound == Some(job),
            "input_conflict: phonon job changed while running"
        );
        *status = if result.is_ok() {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        Ok(())
    })?;
    record_task(&cfg, task)?;
    result
}

fn expand_after_init(cfg: &Config, version: &Path, batch: &str, job: u64) -> Result<()> {
    // 首次准确列表只能由 QE 提供；仍在 init 分配退出之前展开 afterok。
    let _submission = log::DirectoryGuard::acquire(&Step::Phonon.directory(cfg))?;
    let state = PhononState::load(&state_path(cfg))?;
    for q in &state.qpoints {
        if q.status == TaskStatus::Submitted {
            let q_job = q
                .job_id
                .context("scheduler_unknown: q-point receipt missing")?;
            let a = observe(q_job)?;
            if a.state == "PENDING" {
                let out = Command::new("scontrol")
                    .args([
                        "update",
                        &format!("JobId={q_job}"),
                        &format!("Dependency=afterok:{job}"),
                    ])
                    .output()?;
                ensure!(
                    out.status.success(),
                    "scheduler_unknown: q-point dependency update failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
    submit_points(cfg, version, batch, false)?;
    Ok(())
}

fn fresh_ph(cfg: &Config, dir: &Path, index: Option<usize>) -> Result<()> {
    let out = dir.join("out");
    if out.try_exists()? {
        fs::remove_dir_all(out)?;
    }
    let prefix = &cfg.qe.prefix;
    let dyn_file = match index {
        None => format!("{prefix}.dyn0"),
        Some(_) if cfg.phonon.mode == PhMode::Coords => format!("{prefix}.dyn.xml"),
        Some(index) => format!("{prefix}.dyn{index}.xml"),
    };
    let dyn_file = dir.join(dyn_file);
    if dyn_file.try_exists()? {
        fs::remove_file(dyn_file)?;
    }
    Ok(())
}

/// sacct 只把调度失败映射为 Failed；COMPLETED 还必须有工作进程的科学核验回执。
pub fn reconcile(cfg: &Config) -> Result<PhononState> {
    let path = state_path(cfg);
    let snapshot = PhononState::load(&path)?;
    let tasks = std::iter::once((
        "init".to_owned(),
        snapshot.init.status,
        snapshot.init.job_id,
    ))
    .chain(
        snapshot
            .qpoints
            .iter()
            .map(|q| (q.global_index.to_string(), q.status, q.job_id)),
    )
    .chain(std::iter::once((
        "collect".to_owned(),
        snapshot.collect.status,
        snapshot.collect.job_id,
    )));
    for (task, status, job) in tasks {
        if !matches!(status, TaskStatus::Submitted | TaskStatus::Running) {
            continue;
        }
        let job = job.context(
            "scheduler_unknown: phonon submission receipt missing; inspect Slurm before retrying",
        )?;
        // 自己仍在运行，不能查询并将当前科学成功降格。
        if std::env::var("SLURM_JOB_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            == Some(job)
        {
            continue;
        }
        let observation = observe(job)?;
        if observation.terminal {
            PhononState::update(&path, |state| {
                let (current, current_job) = task_mut(state, &task)?;
                if *current_job == Some(job) && *current != TaskStatus::Completed {
                    *current = TaskStatus::Failed;
                }
                Ok(())
            })?;
            record_task(cfg, &task)?;
        }
    }
    PhononState::load(&path)
}

#[cfg(test)]
mod phonon_parallel_tests {
    use super::*;

    #[test]
    fn phonon_parallel_pending_controller_requires_unique_matching_identity() {
        assert_eq!(
            active_controller_state(
                42,
                "JobId=42 JobState=PENDING Reason=DependencyNeverSatisfied"
            )
            .unwrap(),
            "PENDING"
        );
        for text in [
            "",
            "JobId=43 JobState=PENDING",
            "JobId=42 JobState=COMPLETED",
            "JobId=42 JobState=UNKNOWN",
            "JobId=42 JobId=42 JobState=PENDING",
            "JobId=42 JobState=PENDING\nJobId=42 JobState=PENDING",
        ] {
            assert!(active_controller_state(42, text).is_err());
        }
    }

    #[test]
    fn phonon_parallel_dependency_is_all_afterok() {
        assert_eq!(dependency(&[]).unwrap(), None);
        assert_eq!(
            dependency(&[12, 23, 34]).unwrap().as_deref(),
            Some("afterok:12:23:34")
        );
        assert!(dependency(&[0]).is_err());
    }

    #[test]
    fn phonon_parallel_scripts_preserve_user_header_and_quote_paths() {
        let root =
            std::env::temp_dir().join(format!("phonon-scripts-{} with space", store::token()));
        fs::create_dir_all(&root).unwrap();
        let mut cfg: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.work_dir = root.clone();
        cfg.layout = Some("numbered".into());
        cfg.slurm.header = Some(
            "#SBATCH --ntasks=12\nexport OMP_NUM_THREADS=1\nexport MKL_NUM_THREADS=1\n".into(),
        );
        cfg.programs.environment_script = None;
        for mode in [PhMode::Index, PhMode::Coords] {
            cfg.phonon.mode = mode;
            log::initialize(&cfg).unwrap();
            let mut state = PhononState::new(log::mode(&cfg)).unwrap();
            state.qpoints.push(QPointState {
                global_index: 1,
                directory: "q0001".into(),
                crystal: [0.; 3],
                local_index: 1,
                status: TaskStatus::Pending,
                job_id: None,
                script: "q/q0001/ph.sh".into(),
            });
            generate(&cfg, &root.join("input file.toml"), "batch", &state).unwrap();
            for (name, task) in [
                ("init/ph_init.sh", "init"),
                ("q/q0001/ph.sh", "1"),
                ("collect/collect.sh", "collect"),
            ] {
                let path = Step::Phonon.directory(&cfg).join(name);
                let text = fs::read_to_string(&path).unwrap();
                assert!(text.contains(cfg.slurm.header.as_ref().unwrap()));
                assert!(text.contains(&format!("--phonon-task '{}'", task)));
                assert!(text.contains("input file.toml'"));
                assert!(!text.contains("set -") && !text.contains("--allocated-stage"));
                assert!(Command::new("bash")
                    .arg("-n")
                    .arg(path)
                    .status()
                    .unwrap()
                    .success());
            }
        }
        cfg.phonon.mode = PhMode::Nosplit;
        assert!(!enabled(&cfg));
        assert!(generate(
            &cfg,
            &root.join("config"),
            "batch",
            &PhononState::new("index").unwrap()
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
