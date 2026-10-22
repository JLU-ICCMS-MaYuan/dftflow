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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub schema_version: u32,
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
    consumed: Vec<PathBuf>,
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
pub fn scoped_allocated<T>(cfg: &Config, generation: &str, resume: bool, run: impl FnOnce() -> Result<T>) -> Result<T> {
    scoped_mode(cfg, generation, false, resume, true, run)
}

fn scoped_mode<T>(cfg: &Config, generation: &str, submit: bool, resume: bool, allocated: bool, run: impl FnOnce() -> Result<T>) -> Result<T> {
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
fn directory(cfg: &Config) -> PathBuf {
    cfg.work_dir.join("epw-state/native-jobs")
}
fn active(cfg: &Config) -> PathBuf {
    cfg.work_dir.join("epw-state/native-active.json")
}
fn persist(cfg: &Config, task: &Task) -> Result<()> {
    store::write(&directory(cfg).join(format!("{}.json", task.key)), task)?;
    store::write(
        &directory(cfg)
            .join("history")
            .join(format!("{}.json", task.id)),
        task,
    )?;
    store::write(&active(cfg), task)
}
pub fn active_task(cfg: &Config) -> Result<Option<Task>> {
    if active(cfg).exists() {
        Ok(Some(store::read(&active(cfg))?))
    } else {
        Ok(None)
    }
}
pub fn reject(cfg: &Config, generation: &str) -> Result<()> {
    if let Some(mut task) = active_task(cfg)? {
        if task.generation == generation && task.state == "awaiting_validation" {
            task.state = "failed".into();
            persist(cfg, &task)?;
        }
    }
    Ok(())
}
pub fn accept(cfg: &Config, generation: &str) -> Result<()> {
    if let Some(mut task) = active_task(cfg)? {
        if task.generation == generation && task.state == "awaiting_validation" {
            task.state = "validated".into();
            persist(cfg, &task)?;
        }
    }
    Ok(())
}
fn verify(task: &Task) -> Result<()> {
    ensure!(
        matches!(task.schema_version, 1 | 2),
        "unsupported_version: 原生任务记录"
    );
    // 新格式的终态只依赖科学证据，固定名称脚本可供下一次计算覆盖。
    if task.schema_version == 1
        || matches!(
            task.state.as_str(),
            "prepared" | "submission_pending" | "submitted" | "unknown"
        )
    {
        task.script
            .verify()
            .context("input_conflict: 脚本已手工修改；独立调试不属于正式交付")?;
    }
    for id in task.inputs.iter().chain(&task.sources) {
        id.verify()?;
    }
    Ok(())
}

/// 状态查询不修改记录；终态只表示可以在下一次公开命令中进行科学校验。
pub fn status(cfg: &Config) -> Result<Option<serde_json::Value>> {
    let Some(task) = active_task(cfg)? else {
        return Ok(None);
    };
    let state = if matches!(
        task.state.as_str(),
        "submitted" | "submission_pending" | "unknown"
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
    Ok(Some(
        serde_json::json!({"state":state,"job":task.job,"script":task.script.path,"directory":task.directory}),
    ))
}

/// 只在登录端采纳自己提交的终态。手工 sbatch 没有登记回执，不能进入此处。
pub fn reconcile(cfg: &Config, resume: bool) -> Result<()> {
    let Some(mut task) = active_task(cfg)? else {
        return Ok(());
    };
    if task.state != "validated" {
        verify(&task)?;
    }
    if matches!(task.state.as_str(), "unknown" | "submission_pending") {
        anyhow::bail!("scheduler_unknown: 提交回执未知，保留预算预留，禁止重提");
    }
    if task.state == "submitted" {
        let job = task.job.context("scheduler_unknown: 缺少作业号")?;
        if !task.attached {
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                budget::attach_once(
                    l,
                    task.reservation
                        .as_deref()
                        .context("data_invalid: 缺预算预留")?,
                    job,
                )
            })?;
            task.attached = true;
            persist(cfg, &task)?;
        }
        let a = slurm::accounting(job)?;
        ensure!(
            a.terminal,
            "scheduler_unknown: 原生任务 {job} 仍为 {}，禁止重复提交",
            a.state
        );
        // allocation 行的 ExitCode 必须明确为 code:signal；不能从 COMPLETED 猜测退出码。
        let out = Command::new("sacct")
            .args([
                "--noheader",
                "--parsable2",
                "-j",
                &job.to_string(),
                "--format=JobIDRaw,ExitCode",
            ])
            .output()?;
        ensure!(
            out.status.success(),
            "scheduler_unknown: 无法查询原生退出码"
        );
        let text = String::from_utf8(out.stdout)?;
        let rows: Vec<_> = text
            .lines()
            .filter_map(|l| l.trim().split_once('|'))
            .filter(|(id, _)| *id == job.to_string())
            .collect();
        ensure!(rows.len() == 1, "scheduler_unknown: 原生退出码缺失或不唯一");
        let code = rows[0]
            .1
            .trim_end_matches('|')
            .split_once(':')
            .context("scheduler_unknown: ExitCode 格式无效")?;
        let exit: i32 = code.0.parse()?;
        let signal: u32 = code.1.parse()?;
        ensure!(exit >= 0, "scheduler_unknown: ExitCode 为负");
        task.exit_status = Some(if signal == 0 {
            exit
        } else {
            128 + signal as i32
        });
        task.duration_secs = a.node_seconds as f64 / a.nodes.max(1) as f64;
        budget::with_ledger(
            &cfg.budget.ledger,
            &cfg.budget.campaign_id,
            budget::reconcile,
        )?;
        task.state = if a.state == "COMPLETED" && task.exit_status == Some(0) {
            "awaiting_validation"
        } else {
            "failed"
        }
        .into();
        let (stdout, stderr) = if task.schema_version == 1 {
            (
                task.directory.join(format!("native-{}-{job}.out", task.id)),
                task.directory.join(format!("native-{}-{job}.err", task.id)),
            )
        } else {
            (
                task.directory.join(format!("{}.out", task.stem)),
                task.directory.join(format!("{}.err", task.stem)),
            )
        };
        task.stdout = Some(Identity::capture(&stdout).context("data_invalid: 缺少原生 stdout")?);
        task.stderr = Some(Identity::capture(&stderr).context("data_invalid: 缺少原生 stderr")?);
        if task.schema_version == 1 {
            for (source, suffix) in [(&stdout, "out"), (&stderr, "err")] {
                fs::copy(
                    source,
                    task.directory.join(format!("{}.{suffix}", task.stem)),
                )?;
            }
        }
        store::atomic_bytes(
            &task.directory.join(format!("{}.exitcode", task.stem)),
            task.exit_status.unwrap().to_string().as_bytes(),
        )?;
        persist(cfg, &task)?;
    }
    if task.state == "running" && task.job.is_none() {
        // 父分配已由入口确认终态；进程可能在写失败回执前被超时／取消终止。
        ensure!(resume, "execution_failed: 声子分配中断；确认终态后显式 --resume -s");
        task.state = "failed".into();
        persist(cfg, &task)?;
    }
    ensure!(
        task.state != "failed" || resume,
        "execution_failed: 上一原生任务失败；修复后显式 --resume -s，保留旧证据"
    );
    Ok(())
}

fn task_key(generation: &str, work: &Path, command: &[String], stem: &str) -> Result<String> {
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        generation, work, command, stem,
    ))?))
}
/// 在重新进入归集函数时禁止覆盖这个原生调用的准备现场。
pub fn continuing(work: &Path, stem: &str) -> Result<bool> {
    SESSION.with(|s| {
        let session = s.borrow();
        let Some(s) = session.as_ref() else {
            return Ok(false);
        };
        if !directory(&s.cfg).exists() {
            return Ok(false);
        }
        for entry in fs::read_dir(directory(&s.cfg))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let task: Task = store::read(&path)?;
                if task.generation == s.generation
                    && task.directory == work
                    && task.stem == stem
                    && task.state != "failed"
                {
                    verify(&task)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    })
}
pub fn failed(work: &Path) -> Result<bool> {
    SESSION.with(|s| {
        let session = s.borrow();
        let Some(s) = session.as_ref() else {
            return Ok(false);
        };
        Ok(active_task(&s.cfg)?.is_some_and(|t| {
            t.generation == s.generation && t.directory == work && t.state == "failed"
        }))
    })
}
pub fn retire(work: &Path) -> Result<()> {
    SESSION.with(|s| {
        let session = s.borrow();
        let Some(s) = session.as_ref() else {
            return Ok(());
        };
        if !directory(&s.cfg).exists() {
            return Ok(());
        }
        for entry in fs::read_dir(directory(&s.cfg))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let t: Task = store::read(&path)?;
                if t.generation == s.generation && t.directory.starts_with(work) {
                    fs::remove_file(path)?;
                }
            }
        }
        Ok(())
    })
}
fn validate_consumed() -> Result<()> {
    SESSION.with(|s| {
        let mut session = s.borrow_mut();
        let Some(s) = session.as_mut() else {
            return Ok(());
        };
        for path in s.consumed.drain(..) {
            let mut task: Task = store::read(&path)?;
            task.state = "validated".into();
            // 不把已经准备的下一任务从 active 指针中覆盖掉。
            store::write(&path, &task)?;
            store::write(
                &directory(&s.cfg)
                    .join("history")
                    .join(format!("{}.json", task.id)),
                &task,
            )?;
            if active_task(&s.cfg)?.is_some_and(|a| a.id == task.id) {
                store::write(&active(&s.cfg), &task)?;
            }
        }
        Ok(())
    })
}

pub fn execute(command: &[String], work: &Path, stem: &str) -> Result<ExecutionReport> {
    ensure!(!command.is_empty(), "configuration: 空原生命令");
    let (cfg, generation, submit, allocated) = SESSION.with(|s| {
        let s = s.borrow();
        let s = s.as_ref().expect("原生上下文必须存在");
        (s.cfg.clone(), s.generation.clone(), s.submit, s.allocated)
    });
    let key = task_key(&generation, work, command, stem)?;
    let file = directory(&cfg).join(format!("{key}.json"));
    let previous: Option<Task> = if file.exists() {
        Some(store::read(&file)?)
    } else {
        None
    };
    if let Some(task) = &previous {
        verify(task)?;
        if matches!(task.state.as_str(), "awaiting_validation" | "validated") {
            let stdout = task
                .stdout
                .as_ref()
                .context("data_invalid: 无原生日志身份")?;
            let stderr = task
                .stderr
                .as_ref()
                .context("data_invalid: 无原生日志身份")?;
            stdout.verify()?;
            stderr.verify()?;
            SESSION.with(|s| s.borrow_mut().as_mut().unwrap().consumed.push(file));
            return Ok(ExecutionReport {
                exit_status: task.exit_status,
                stdout_path: stdout.path.clone(),
                stderr_path: stderr.path.clone(),
                duration_secs: task.duration_secs,
            });
        }
        ensure!(
            !matches!(
                task.state.as_str(),
                "submitted" | "unknown" | "submission_pending"
            ),
            "scheduler_unknown: 原生调用已有未完成提交"
        );
    }
    validate_consumed()?;
    let mut task = match previous.filter(|t| t.state == "prepared") {
        Some(task) => task,
        None => prepare(&cfg, &generation, &key, command, work, stem)?,
    };
    if allocated {
        // 运行相同的可读科学脚本；父尝试负责唯一分配和预算。
        task.state = "running".into();
        persist(&cfg, &task)?;
        let start = std::time::Instant::now();
        let status = Command::new("/bin/bash")
            .arg(&task.script.path)
            .current_dir(work)
            .status()
            .context("execution_failed: 无法执行声子子任务脚本")?;
        task.duration_secs = start.elapsed().as_secs_f64();
        task.exit_status = status.code();
        task.stdout = Some(Identity::capture(&work.join(format!("{stem}.out")))?);
        task.stderr = Some(Identity::capture(&work.join(format!("{stem}.err")))?);
        store::atomic_bytes(&work.join(format!("{stem}.exitcode")), task.exit_status.unwrap_or(128).to_string().as_bytes())?;
        task.state = if status.success() { "awaiting_validation" } else { "failed" }.into();
        persist(&cfg, &task)?;
        SESSION.with(|s| s.borrow_mut().as_mut().unwrap().consumed.push(file));
        return Ok(ExecutionReport {
            exit_status: task.exit_status,
            stdout_path: task.stdout.unwrap().path,
            stderr_path: task.stderr.unwrap().path,
            duration_secs: task.duration_secs,
        });
    }
    if submit {
        task.reservation = Some(budget::with_ledger(
            &cfg.budget.ledger,
            &cfg.budget.campaign_id,
            |l| budget::reserve_workflow(l, &format!("0026:{}", task.id), &cfg.resources),
        )?);
        task.state = "submission_pending".into();
        persist(&cfg, &task)?;
        let out = Command::new("sbatch")
            .arg("--parsable")
            .arg(&task.script.path)
            .current_dir(work)
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
                persist(&cfg, &task)?;
                budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                    budget::attach_once(l, task.reservation.as_ref().unwrap(), job.job_id)
                })?;
                task.attached = true;
                persist(&cfg, &task)?;
                println!("job_id: {}", job.job_id);
            }
            Err(e) => {
                task.state = "unknown".into();
                persist(&cfg, &task)?;
                return Err(e);
            }
        }
    }
    println!(
        "script: {}\ndirectory: {}\nworkflow_status: {}\nnext: 作业结束后再次执行同一公开命令 -s",
        task.script.path.display(),
        work.display(),
        task.state
    );
    Err(Paused.into())
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
    let r = &cfg.resources;
    ensure!(
        !r.partition.contains(['\n', '\r']),
        "configuration: partition 含换行"
    );
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
    let mut text = format!("#!/bin/bash\n#SBATCH --job-name=epw-{purpose}\n#SBATCH --partition={}\n#SBATCH --nodes={}\n#SBATCH --ntasks={}\n#SBATCH --time={}\n#SBATCH --output=slurm-{purpose}.log\n#SBATCH --error=slurm-{purpose}.log\n#SBATCH --open-mode=truncate\nif ! cd -- {}; then exit 1; fi\n", r.partition, r.nodes, r.mpi, r.wall_seconds.div_ceil(60), double_quote(&work.to_string_lossy()));
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
    text.push_str(&format!(
        " > {} 2> {}",
        q(&format!("{stem}.out")),
        q(&format!("{stem}.err"))
    ));
    text.push('\n');
    let script = work.join(format!("{purpose}.sh"));
    store::atomic_bytes(&script, text.as_bytes())?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    let task = Task {
        schema_version: 2,
        id,
        key: key.into(),
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
