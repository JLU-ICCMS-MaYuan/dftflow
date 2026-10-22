//! 当前配置驱动的编号工作区。提交快照只属于一次尝试，不控制后续请求。
use super::{
    budget,
    config::Config,
    layout, parallelism, slurm,
    stages::{self, Completion, Step},
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attempt {
    schema_version: u32,
    id: String,
    target: Step,
    config: Identity,
    executable: Identity,
    script: Option<Identity>,
    sources: Vec<Identity>,
    state: String,
    job: Option<u64>,
    reservation: Option<String>,
    error: Option<String>,
    #[serde(default)]
    phonon_chain: bool,
    #[serde(default)]
    retry_failed: bool,
}
fn active(root: &Path) -> PathBuf {
    root.join("epw-state/active.json")
}
fn attempt_path(root: &Path, id: &str) -> PathBuf {
    root.join("epw-state/attempts").join(id).join("attempt.json")
}

// 提交端只写独立回执，避免快速启动的 worker 与登录端互相覆盖状态。
fn read_attempt(path: &Path) -> Result<Attempt> {
    let mut a: Attempt = store::read(path)?;
    let receipt = a.config.path.with_file_name("submission.json");
    if a.phonon_chain && a.job.is_none() && receipt.is_file() {
        let value: serde_json::Value = store::read(&receipt)?;
        a.job = value["job"].as_u64();
        if a.state == "submission_pending" {
            a.state = if a.job.is_some() { "submitted" } else { "unknown" }.into();
            a.error = value["error"].as_str().map(str::to_owned);
        }
    }
    Ok(a)
}

pub(super) fn check_active(cfg: &Config, own: Option<&str>) -> Result<()> {
    if !active(&cfg.work_dir).exists() {
        return Ok(());
    }
    let old = read_attempt(&active(&cfg.work_dir))?;
    if own == Some(old.id.as_str()) {
        return Ok(());
    }
    if let Some(job) = old.job {
        ensure!(
            slurm::accounting(job)?.terminal,
            "scheduler_unknown: 工作区已有活动任务 {job}，禁止重复提交或修改原生输入"
        );
        budget::with_ledger(
            &cfg.budget.ledger,
            &cfg.budget.campaign_id,
            budget::reconcile,
        )?;
        for results in [
            layout::path(&cfg.work_dir, "results"),
            Step::Superconductivity.directory(cfg).join("results"),
        ] {
            for relative in [
                "comparison",
                "native-results",
                "figures/comparison-figures",
                "figures/native-figures",
            ] {
                store::recover_finished_lock(&results.join(relative), job)?;
            }
        }
    } else {
        ensure!(
            !matches!(
                old.state.as_str(),
                "submission_pending" | "unknown" | "running"
            ),
            "scheduler_unknown: 尝试 {} 终态未知，禁止重复",
            old.id
        );
    }
    Ok(())
}

pub fn report(cfg: &Config) -> Result<serde_json::Value> {
    super::run_state::ready(&cfg.work_dir)?;
    let mut steps = serde_json::Map::new();
    for step in Step::ALL {
        steps.insert(
            step.name().into(),
            serde_json::json!(if stages::reusable(cfg, step)? {
                "reuse"
            } else {
                "update"
            }),
        );
    }
    let mut task = if active(&cfg.work_dir).is_file() {
        Some(read_attempt(&active(&cfg.work_dir))?)
    } else {
        None
    };
    if let Some(a) = task.as_mut().filter(|a| a.phonon_chain && !matches!(a.state.as_str(), "succeeded" | "failed")) {
        if let Some(job) = a.job {
            a.state = match slurm::accounting(job) {
                Ok(s) if s.terminal => "failed",
                Ok(s) if s.state == "RUNNING" => "running",
                Ok(_) => "submitted",
                Err(_) => "unknown",
            }.into();
        }
    }
    let complete = if task.as_ref().is_some_and(|a| a.phonon_chain) {
        Step::Phonon.closure().iter().all(|s| steps[s.name()] == "reuse")
            && !super::workspace_qe_plot::pending(cfg, Step::Phonon)?
            && task.as_ref().is_some_and(|a| a.state == "succeeded")
    } else { steps.values().all(|v|v=="reuse") };
    Ok(
        serde_json::json!({"operation_status":"ok","workflow_complete":complete,"workflow_status":task.as_ref().map(|a| &a.state),"steps":steps,"attempt":task,"native_task":super::native_jobs::status(cfg)?}),
    )
}
pub fn status(cfg: &Config) -> Result<()> {
    let value = report(cfg)?;
    if value["attempt"]["phonon_chain"] == true {
        println!("workflow_status: {}\nworkflow_complete: {}", value["workflow_status"].as_str().unwrap_or("unknown"), value["workflow_complete"]);
    }
    if !value["native_task"].is_null() { println!("native_task: {}",value["native_task"]); }
    for (name, state) in value["steps"].as_object().unwrap() {
        println!("{name}: {}", state.as_str().unwrap());
    }
    Ok(())
}

fn freeze(cfg: &Config, target: Step) -> Result<(PathBuf, Attempt)> {
    let id = store::token();
    let dir = cfg.work_dir.join("epw-state/attempts").join(&id);
    fs::create_dir_all(dir.join("pp"))?;
    let mut fixed = cfg.clone();
    fs::copy(&cfg.structure, dir.join("POSCAR"))?;
    fixed.structure = dir.join("POSCAR");
    if let Some(p) = &cfg.kpath {
        fs::copy(p, dir.join("KPATH.in"))?;
        fixed.kpath = Some(dir.join("KPATH.in"));
    }
    for (el, p) in &cfg.pseudos {
        let dest = dir.join("pp").join(format!("{el}.UPF"));
        fs::copy(p, &dest)?;
        ensure!(
            store::hash_file(p)? == store::hash_file(&dest)?,
            "input_conflict: 赝势复制期间改变"
        );
        fixed.pseudos.insert(el.clone(), dest);
    }
    let file = dir.join("input.toml");
    store::atomic_bytes(&file, toml::to_string_pretty(&fixed)?.as_bytes())?;
    let mut sources = vec![Identity::capture(&fixed.structure)?];
    sources.extend(
        fixed
            .pseudos
            .values()
            .map(|p| Identity::capture(p))
            .collect::<Result<Vec<_>>>()?,
    );
    if let Some(p) = &fixed.kpath {
        sources.push(Identity::capture(p)?);
    }
    for p in [
        &fixed.programs.pw.path,
        &fixed.programs.ph.path,
        &fixed.programs.epw.path,
    ] {
        sources.push(Identity::capture(p)?);
    }
    for p in [
        &fixed.programs.wannier90,
        &fixed.programs.pw2wannier90,
        &fixed.programs.postw90,
    ]
    .into_iter()
    .flatten()
    {
        sources.push(Identity::capture(&p.path)?);
    }
    if target.closure().contains(&Step::Electronic) {
        for name in ["bands", "dos"] {
            sources.push(Identity::capture(&super::workspace_science::post_program(&fixed, name)?.path)?);
        }
    }
    if target == Step::Phonon {
        for name in ["q2r", "matdyn"] {
            sources.push(Identity::capture(&super::workspace_science::post_program(&fixed, name)?.path)?);
        }
    }
    if let Some(p) = &fixed.programs.environment_script {
        sources.push(Identity::capture(p)?);
    }
    let a = Attempt {
        schema_version: 1,
        id,
        target,
        config: Identity::capture(&file)?,
        executable: Identity::capture(&std::env::current_exe()?)?,
        script: None,
        sources,
        state: "prepared".into(),
        job: None,
        reservation: None,
        error: None,
        phonon_chain: false,
        retry_failed: false,
    };
    store::write(&dir.join("attempt.json"), &a)?;
    Ok((dir, a))
}

fn script(cfg: &Config, dir: &Path, a: &Attempt) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let q = slurm::shell_quote;
    let r = &cfg.resources;
    let mut text=format!("#!/bin/bash\n#SBATCH --job-name=epw-{}\n#SBATCH --partition={}\n#SBATCH --nodes={}\n#SBATCH --ntasks={}\n#SBATCH --time={}\n{}cd -- {} || exit $?\n",
        a.target.name(),r.partition,r.nodes,r.mpi,(r.wall_seconds+59)/60,
        slurm::log_directives(dir, "slurm-%j.out", "slurm-%j.err"),q(&dir.to_string_lossy()));
    if let Some(p) = &cfg.programs.environment_script {
        text.push_str(&super::workflow::source_environment(p)?);
    }
    text.push_str(&format!(
        "exec {} --workspace-attempt {}\n",
        q(&a.executable.path.to_string_lossy()),
        q(&attempt_path(&cfg.work_dir, &a.id).to_string_lossy())
    ));
    let p = dir.join("submit.sh");
    store::atomic_bytes(&p, text.as_bytes())?;
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755))?;
    Ok(p)
}

/// 每个阶段脚本都能独立提交；批量脚本仅负责组织三个公开入口。
pub fn generate(config: &Path, target: Step) -> Result<PathBuf> {
    let cfg = Config::load(config)?;
    super::run_state::ready(&cfg.work_dir)?;
    check_active(&cfg, None)?;
    generate_current(config, target, &cfg)
}

fn generate_current(config: &Path, target: Step, cfg: &Config) -> Result<PathBuf> {
    layout::initialize(cfg)?;
    // 在重建上游之前撤下过期的下游回执，防止相同字节的新结果命中旧交付。
    let stale = Step::ALL.into_iter()
        .map(|s| Ok((s, stages::receipt(&cfg.work_dir, s).exists() && !stages::reusable(cfg, s)?)))
        .collect::<Result<Vec<_>>>()?;
    for (step, outdated) in stale {
        if outdated { archive_step(cfg, step)?; }
    }
    store::atomic_bytes(
        &cfg.work_dir.join("epw-state/current-config.toml"),
        toml::to_string_pretty(&cfg)?.as_bytes(),
    )?;
    for step in target.closure() {
        if step == target || !stages::reusable(&cfg, step)? {
            parallelism::preflight(&cfg, step, false)?;
        }
    }
    // 在生成新输入前保留过期的科学现场，避免用新请求覆盖旧输入。
    for step in target.closure() {
        if stages::reusable(cfg, step)? {
            continue;
        }
        let request = cfg
            .work_dir
            .join("epw-state/requests")
            .join(format!("{}.json", step.name()));
        if stages::receipt(&cfg.work_dir, step).exists()
            || (request.is_file()
                && store::read::<String>(&request)? != stages::scientific_key(cfg, step)?)
        {
            archive_step(cfg, step)?;
        }
    }
    super::workspace_science::generate(&cfg, target)?;
    let mut last = None;
    for step in target.closure() {
        let (_, mut a) = freeze(&cfg, step)?;
        let p = script(&cfg, &step.directory(&cfg), &a)?;
        a.script = Some(Identity::capture(&p)?);
        store::write(&attempt_path(&cfg.work_dir, &a.id), &a)?;
        // 电子两个实际目录分别拥有可独立提交的脚本。
        if step == Step::Electronic {
            for name in ["qe_eband", "qe_eledos"] {
                let (snapshot, mut leaf) = freeze(&cfg, step)?;
                let dir = step.directory(&cfg).join(name);
                let p = script(&cfg, &dir, &leaf)?;
                store::atomic_bytes(
                    &p,
                    fs::read_to_string(&p)?
                        .replace("--workspace-attempt", "--workspace-task")
                        .as_bytes(),
                )?;
                leaf.script = Some(Identity::capture(&p)?);
                let input = dir.join(if name == "qe_eband" {
                    "eband.in"
                } else {
                    "eletdos.in"
                });
                let frozen = snapshot.join(input.file_name().unwrap());
                fs::copy(&input, &frozen)?;
                leaf.sources.push(Identity::capture(&frozen)?);
                let task = NativeTask {
                    attempt: leaf,
                    directory: dir,
                    input: Identity::capture(&input)?,
                    argv: vec![],
                    stem: name.into(),
                    qe: true,
                    scf: false,
                    electronic_leaf: Some(name.into()),
                };
                store::write(&attempt_path(&cfg.work_dir, &task.attempt.id), &task)?;
            }
        }
        last = Some(p);
    }
    let q = slurm::shell_quote;
    let exe = std::env::current_exe()?;
    let bins = exe.parent().context("environment: 缺少程序目录")?;
    let config = super::paths::absolute(&std::env::current_dir()?, config)?;
    // 一个有界分配内按依赖顺序运行；无需在登录节点等待其他作业结束。
    let all = format!(
        "#!/bin/bash\ncd -- {} || exit $?\nexec {} -c {} -s\n",
        q(&cfg.work_dir.to_string_lossy()),
        q(&bins.join("epw_superconductivity").to_string_lossy()),
        q(&config.to_string_lossy())
    );
    store::atomic_bytes(&cfg.work_dir.join("submit_all.sh"), all.as_bytes())?;
    fs::set_permissions(
        cfg.work_dir.join("submit_all.sh"),
        fs::Permissions::from_mode(0o755),
    )?;
    last.context("data_invalid: 阶段集合为空")
}

/// 一份已登记分配内自动完成声子阶段；原生子任务继续使用既有回执。
pub fn start_phonon(config: &Path, resume: bool) -> Result<()> {
    let cfg = Config::load(config)?;
    super::run_state::ready(&cfg.work_dir)?;
    check_active(&cfg, None)?;
    super::native_jobs::reconcile(&cfg, resume)?;
    let mut unfinished_attempt = false;
    if active(&cfg.work_dir).is_file() {
        let old = read_attempt(&active(&cfg.work_dir))?;
        if old.phonon_chain && old.state != "succeeded" {
            unfinished_attempt = true;
            let previous = Config::load(&old.config.path)?;
            ensure!(resume || stages::scientific_key(&previous, Step::Phonon)? != stages::scientific_key(&cfg, Step::Phonon)?,
                "execution_failed: 声子分配未完整成功；修复后显式 --resume -s");
        }
    }
    layout::initialize(&cfg)?;
    let mut pending = false;
    for step in Step::Phonon.closure() {
        if !stages::reusable(&cfg, step)? {
            parallelism::preflight(&cfg, step, false)?;
            pending = true;
        }
    }
    if !pending && !super::workspace_qe_plot::pending(&cfg, Step::Phonon)? && !unfinished_attempt {
        println!("workflow_status: complete\nworkflow_complete: true");
        return Ok(());
    }
    let (_, mut a) = freeze(&cfg, Step::Phonon)?;
    a.phonon_chain = true;
    a.retry_failed = resume;
    let dir = Step::Phonon.directory(&cfg);
    fs::create_dir_all(&dir)?;
    let q = super::native_jobs::double_quote;
    let r = &cfg.resources;
    ensure!(!r.partition.contains(['\n', '\r']) && !dir.to_string_lossy().contains(['\n', '\r']), "configuration: Slurm 参数含换行");
    let mut body = format!("#!/bin/bash\n#SBATCH --job-name=epw-phonon\n#SBATCH --partition={}\n#SBATCH --nodes={}\n#SBATCH --ntasks={}\n#SBATCH --time={}\n#SBATCH --output=slurm-phonon.log\n#SBATCH --error=slurm-phonon.log\n#SBATCH --open-mode=truncate\nif ! cd -- {}; then exit 1; fi\n", r.partition, r.nodes, r.mpi, r.wall_seconds.div_ceil(60), q(&dir.to_string_lossy()));
    if let Some(env) = &cfg.programs.environment_script {
        body.push_str(&format!("source {} --force\n", q(&env.to_string_lossy())));
    }
    body.push_str(&format!("exec {} --workspace-attempt {}\n", q(&a.executable.path.to_string_lossy()), q(&attempt_path(&cfg.work_dir, &a.id).to_string_lossy())));
    let script = dir.join("phonon.sh");
    store::atomic_bytes(&script, body.as_bytes())?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    a.script = Some(Identity::capture(&script)?);
    a.reservation = Some(budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
        budget::reconcile(l)?;
        budget::reserve_workflow(l, &format!("0048:{}", a.id), &cfg.resources)
    })?);
    a.state = "submission_pending".into();
    persist(&cfg, &a)?;
    let result = Command::new("sbatch").arg("--parsable").arg(&script).current_dir(&dir).output()
        .context("scheduler_unknown: 无法调用 sbatch").and_then(|out| {
            ensure!(out.status.success(), "scheduler_unknown: sbatch 失败: {}", String::from_utf8_lossy(&out.stderr));
            slurm::parse_job_id(String::from_utf8_lossy(&out.stdout).trim())
        });
    match result {
        Ok(job) => {
            let current = read_attempt(&attempt_path(&cfg.work_dir, &a.id))?;
            ensure!(current.job.is_none_or(|j| j == job.job_id), "scheduler_unknown: 声子作业身份不符");
            store::write(&attempt_path(&cfg.work_dir, &a.id).with_file_name("submission.json"),
                &serde_json::json!({"job": job.job_id, "error": null}))?;
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                budget::attach_once(l, a.reservation.as_ref().unwrap(), job.job_id)
            })?;
            println!("job_id: {}\nscript: {}\ndirectory: {}\nworkflow_status: {}\nnext: 本声子阶段在分配内自动完成，可用 --status 查看", job.job_id, script.display(), dir.display(), read_attempt(&attempt_path(&cfg.work_dir, &a.id))?.state);
            Ok(())
        }
        Err(e) => {
            store::write(&attempt_path(&cfg.work_dir, &a.id).with_file_name("submission.json"),
                &serde_json::json!({"job": null, "error": format!("{e:#}")}))?;
            Err(e)
        }
    }
}

pub fn start(config: &Path, target: Step, submit: bool) -> Result<()> {
    let cfg = Config::load(config)?;
    super::run_state::ready(&cfg.work_dir)?;
    check_active(&cfg, None)?;
    let script_path = generate_current(config, target, &cfg)?;
    status(&cfg)?;
    let pending = target
        .closure()
        .into_iter()
        .map(|s| stages::reusable(&cfg, s).map(|ok| !ok))
        .collect::<Result<Vec<_>>>()?;
    if pending.iter().all(|b| !*b) && !super::workspace_science::post_pending(&cfg, target)? {
        println!("workflow_status: complete（全部复用）");
        return Ok(());
    }
    for step in target.closure() {
        if step == target || !stages::reusable(&cfg, step)? {
            parallelism::preflight(&cfg, step, false)?;
        }
    }
    let text = fs::read_to_string(&script_path)?;
    // 从生成脚本对应的尝试查找身份，绝不从旧 active 继承配置。
    let mut candidates = fs::read_dir(cfg.work_dir.join("epw-state/attempts"))?
        .map(|e| e.map(|x| x.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    candidates.sort();
    let mut a = candidates
        .iter()
        .rev()
        .filter_map(|p| store::read::<Attempt>(&p.join("attempt.json")).ok())
        .find(|a| {
            a.target == target
                && a.script.as_ref().is_some_and(|id| {
                    id.path == script_path && id.sha256 == store::hash_bytes(text.as_bytes())
                })
        })
        .context("data_invalid: 生成脚本没有对应尝试")?;
    a.script.as_ref().unwrap().verify()?;
    if !submit {
        return execute(&attempt_path(&cfg.work_dir, &a.id));
    }
    let token = budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
        budget::reserve_workflow(l, &format!("0018:{}", a.id), &cfg.resources)
    })?;
    a.reservation = Some(token);
    a.state = "submission_pending".into();
    persist(&cfg, &a)?;
    let out = Command::new("sbatch")
        .arg("--parsable")
        .arg(&script_path)
        .current_dir(target.directory(&cfg))
        .output();
    let result = out
        .context("scheduler_unknown: 无法调用 sbatch")
        .and_then(|out| {
            ensure!(
                out.status.success(),
                "scheduler_unknown: sbatch 失败: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            slurm::parse_job_id(String::from_utf8_lossy(&out.stdout).trim())
        });
    match result {
        Ok(job) => {
            a.job = Some(job.job_id);
            a.state = "submitted".into();
            persist(&cfg, &a)?;
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                budget::attach_once(l, a.reservation.as_ref().unwrap(), job.job_id)
            })?;
            println!("job_id: {}\nworkflow_status: submitted", job.job_id);
            Ok(())
        }
        Err(e) => {
            a.state = "unknown".into();
            a.error = Some(e.to_string());
            persist(&cfg, &a)?;
            Err(e)
        }
    }
}
fn persist(cfg: &Config, a: &Attempt) -> Result<()> {
    store::write(&attempt_path(&cfg.work_dir, &a.id), a)?;
    store::write(&active(&cfg.work_dir), a)
}

fn verify_script(id: &Identity) -> Result<()> {
    id.verify().context("input_conflict: 提交脚本已改变；不要重提或修改旧尝试，请在 run/ 重新执行 epw_wannier --stage wannier --resume -s（其他阶段使用对应公开命令），生成当前配置的新尝试")
}

pub fn execute(path: &Path) -> Result<()> {
    let mut a = read_attempt(path)?;
    ensure!(a.schema_version == 1, "unsupported_version: 提交快照版本");
    a.config.verify()?;
    a.executable.verify()?;
    for id in &a.sources {
        id.verify()?;
    }
    if let Some(id) = &a.script {
        verify_script(id)?;
    }
    let cfg = Config::load(&a.config.path)?;
    let job = std::env::var("SLURM_JOB_ID")
        .context("allocation: 需要 Slurm 分配")?
        .parse::<u64>()?;
    for step in a.target.closure() {
        if step == a.target || !stages::reusable(&cfg, step)? {
            parallelism::preflight(&cfg, step, true)?;
        }
    }
    let allocation = slurm::running_allocation(job)?;
    ensure!(
        allocation.nodes == u64::from(cfg.resources.nodes),
        "allocation: 节点数量与配置不符"
    );
    {
        super::run_state::ready(&cfg.work_dir)?;
        check_active(&cfg, Some(&a.id))?;
        if active(&cfg.work_dir).exists() {
            let old = read_attempt(&active(&cfg.work_dir))?;
            if old.id == a.id {
                ensure!(
                    old.state != "running",
                    "scheduler_unknown: 同一尝试已在运行"
                );
                a = old;
            }
        }
        ensure!(
            a.job.is_none_or(|j| j == job),
            "input_conflict: 旧尝试 {} 已绑定作业 {}，当前作业 {job} 不能重用此脚本；请在 run/ 重新执行 epw_wannier --stage wannier -s（其他阶段使用对应公开命令）生成新尝试，不要 sbatch 已执行过的旧 submit.sh",
            a.id,
            a.job.unwrap_or(job)
        );
        if a.reservation.is_none() {
            a.reservation = Some(budget::with_ledger(
                &cfg.budget.ledger,
                &cfg.budget.campaign_id,
                |l| budget::register_generated(l, job, &cfg.resources),
            )?);
        }
        if a.phonon_chain {
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |l| {
                budget::attach_once(l, a.reservation.as_ref().unwrap(), job)
            })?;
        }
        a.job = Some(job);
        a.state = "running".into();
        persist(&cfg, &a)?;
    }
    store::atomic_bytes(
        &cfg.work_dir.join("epw-state/current-config.toml"),
        toml::to_string_pretty(&cfg)?.as_bytes(),
    )?;
    let result = (|| -> Result<()> {
        if a.phonon_chain {
            return super::native_workspace::allocated(&a.config.path, a.retry_failed, &a.id);
        }
        for step in a.target.closure() {
            if stages::reusable(&cfg, step)? {
                println!("reuse: {}", step.name());
                super::workspace_qe_plot::refresh(&cfg, step)?;
                continue;
            }
            parallelism::preflight(&cfg, step, true)?;
            let upstream = stages::upstream(&cfg, step)?;
            let key = stages::scientific_key(&cfg, step)?;
            let directory = step.directory(&cfg);
            let scripts = if directory.exists() {
                crate::qe_data::save::files(&directory)?
                    .into_iter()
                    .filter(|f| {
                        f.relative_path
                            .file_name()
                            .is_some_and(|n| n == "submit.sh")
                    })
                    .map(|f| {
                        Ok((
                            f.relative_path.clone(),
                            fs::read(directory.join(f.relative_path))?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![]
            };
            archive_step(&cfg, step)?;
            store::write(
                &cfg.work_dir
                    .join("epw-state/requests")
                    .join(format!("{}.json", step.name())),
                &key,
            )?;
            println!("calculate: {}", step.name());
            let scientific_result = super::workspace_science::run(&cfg, step);
            for (rel, bytes) in scripts {
                let p = directory.join(rel);
                store::atomic_bytes(&p, &bytes)?;
                fs::set_permissions(&p, fs::Permissions::from_mode(0o755))?;
            }
            scientific_result?;
            let outputs = super::workspace_science::evidence(&cfg, step)?;
            let c = Completion {
                schema_version: 1,
                step,
                scientific_key: key,
                upstream,
                outputs,
                provenance: None,
            };
            c.verify()?;
            store::write(&stages::receipt(&cfg.work_dir, step), &c)?;
        }
        super::workspace_science::refresh_post(&cfg, a.target)?;
        Ok(())
    })();
    a.state = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    }
    .into();
    a.error = result.as_ref().err().map(|e| format!("{e:#}"));
    persist(&cfg, &a)?;
    if a.phonon_chain {
        println!("workflow_status: {}\nworkflow_complete: {}", a.state, result.is_ok());
    }
    result
}

/// 只移动本阶段；成功的相容上游目录不受影响。未知文件保留在历史。
pub(super) fn archive_step(cfg: &Config, step: Step) -> Result<()> {
    let request = cfg
        .work_dir
        .join("epw-state/requests")
        .join(format!("{}.json", step.name()));
    if request.is_file() && store::read::<String>(&request)? == stages::scientific_key(cfg, step)? {
        let receipt = stages::receipt(&cfg.work_dir, step);
        if step >= Step::Coarse && !receipt.exists() { return Ok(()); }
        if step == Step::Phonon && !receipt.exists()
            && super::workspace_science::partial_phonon(cfg)?.is_some() {
            return Ok(());
        }
        if step == Step::Electronic && super::workspace_electronic::compatible_partial(cfg)? {
            let compatible = if receipt.exists() {
                let c: Completion = store::read(&receipt)?;
                c.available()? && c.scientific_key == stages::scientific_key(cfg, step)? && c.upstream == stages::upstream(cfg, step)?
            } else { true };
            if compatible {
                if receipt.exists() {
                    let history = cfg.work_dir.join("epw-state/history").join(format!("electronic-completion-{}", store::token()));
                    fs::create_dir_all(&history)?;
                    fs::rename(receipt, history.join("completion.json"))?;
                }
                return Ok(());
            }
        }
    }
    let history =
        cfg.work_dir
            .join("epw-state/history")
            .join(format!("{}-{}", step.name(), store::token()));
    let dir = step.directory(cfg);
    if dir.exists() {
        let chain_script = if step == Step::Phonon && dir.join("phonon.sh").is_file() {
            Some(fs::read(dir.join("phonon.sh"))?)
        } else { None };
        fs::create_dir_all(&history)?;
        fs::rename(&dir, history.join("calculation"))?;
        preserve_active_logs(&history.join("calculation"), &dir)?;
        if let Some(body) = chain_script {
            store::atomic_bytes(&dir.join("phonon.sh"), &body)?;
            fs::set_permissions(dir.join("phonon.sh"), fs::Permissions::from_mode(0o755))?;
        }
    }
    let receipt = stages::receipt(&cfg.work_dir, step);
    if receipt.exists() {
        fs::create_dir_all(&history)?;
        fs::rename(receipt, history.join("completion.json"))?;
    }
    super::workspace_science::invalidate(cfg, step, &history)?;
    if step == Step::Electronic { super::workspace_electronic::archive(cfg, &history)?; }
    Ok(())
}

pub(crate) fn slurm_log(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else { return false; };
    let Some((job, suffix)) = name.strip_prefix("slurm-").and_then(|s| s.split_once('.')) else { return false; };
    !job.is_empty() && job.bytes().all(|c| c.is_ascii_digit())
        && matches!(suffix, "out" | "err")
}

/// Slurm 持有原 inode；归档后用硬链接保留当前目录的持续写入入口。
pub(crate) fn preserve_active_logs(source: &Path, dir: &Path) -> Result<()> {
    if let Ok(job) = std::env::var("SLURM_JOB_ID") {
        if job.parse::<u64>().is_ok() {
            for suffix in ["out", "err"] {
                let name = format!("slurm-{job}.{suffix}");
                let path = source.join(&name);
                if path.is_file() && !fs::symlink_metadata(&path)?.file_type().is_symlink() {
                    fs::create_dir_all(dir)?;
                    let target = dir.join(name);
                    if let Err(error) = fs::hard_link(&path, &target) {
                        if error.kind() != std::io::ErrorKind::AlreadyExists { return Err(error.into()); }
                        #[cfg(unix)] {
                            use std::os::unix::fs::MetadataExt;
                            let a = fs::metadata(&path)?;
                            let b = fs::symlink_metadata(&target)?;
                            ensure!(b.is_file() && a.dev() == b.dev() && a.ino() == b.ino(),
                                "input_conflict: 当前 Slurm 日志入口不是原 inode");
                        }
                        #[cfg(not(unix))]
                        anyhow::bail!("input_conflict: 当前平台无法核验已存在日志的 inode");
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn import_legacy(config: &Path, source: &Path) -> Result<()> {
    super::workspace_import::import(config, source)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeTask {
    attempt: Attempt,
    directory: PathBuf,
    input: Identity,
    argv: Vec<String>,
    stem: String,
    qe: bool,
    scf: bool,
    #[serde(default)]
    electronic_leaf: Option<String>,
}

/// 原生子任务有自己的输入及提交脚本；输入另存尝试快照，排队期间不重读根配置。
pub fn native_script(
    program: &super::config::Program,
    input: &Path,
    dir: &Path,
    launcher: &[String],
    stem: &str,
    qe: bool,
    scf: bool,
) -> Result<()> {
    if super::native_jobs::enabled() { return Ok(()); }
    let Some(root) = dir.ancestors().find(|p| layout::enabled(p)) else {
        return Ok(());
    };
    let cfg = Config::load(&root.join("epw-state/current-config.toml"))?;
    let step = Step::ALL
        .into_iter()
        .find(|s| dir.starts_with(s.directory(&cfg)))
        .unwrap_or(Step::Coarse);
    // 阶段根的 submit.sh 由阶段编排器管理；只有真正的子计算目录在这里写任务脚本。
    if dir == step.directory(&cfg) || step == Step::Electronic {
        return Ok(());
    }
    let (snapshot, mut a) = freeze(&cfg, step)?;
    let frozen = snapshot.join(
        input
            .file_name()
            .context("data_invalid: 原生输入无文件名")?,
    );
    fs::copy(input, &frozen)?;
    let mut argv = launcher.to_vec();
    argv.push(program.path.to_string_lossy().into_owned());
    argv.extend(program.args.clone());
    argv.extend(["-inp".into(), frozen.to_string_lossy().into_owned()]);
    let p = script(&cfg, dir, &a)?;
    let text = fs::read_to_string(&p)?.replace("--workspace-attempt", "--workspace-task");
    store::atomic_bytes(&p, text.as_bytes())?;
    a.script = Some(Identity::capture(&p)?);
    a.sources.push(Identity::capture(&frozen)?);
    let task = NativeTask {
        attempt: a.clone(),
        directory: dir.into(),
        input: Identity::capture(input)?,
        argv,
        stem: stem.into(),
        qe,
        scf,
        electronic_leaf: None,
    };
    store::write(&attempt_path(root, &a.id), &task)?;
    Ok(())
}

pub fn execute_task(path: &Path) -> Result<()> {
    let mut task: NativeTask = store::read(path)?;
    let a = &mut task.attempt;
    a.config.verify()?;
    a.executable.verify()?;
    for id in &a.sources {
        id.verify()?;
    }
    verify_script(a.script.as_ref().context("data_invalid: 子任务无脚本身份")?)?;
    let cfg = Config::load(&a.config.path)?;
    parallelism::preflight(&cfg, a.target, true)?;
    let job = std::env::var("SLURM_JOB_ID")
        .context("allocation: 需要 Slurm 分配")?
        .parse::<u64>()?;
    ensure!(
        slurm::running_allocation(job)?.nodes == u64::from(cfg.resources.nodes),
        "allocation: 节点数不一致"
    );
    super::run_state::ready(&cfg.work_dir)?;
    check_active(&cfg, Some(&a.id))?;
    if active(&cfg.work_dir).exists() {
        let old: Attempt = store::read(&active(&cfg.work_dir))?;
        ensure!(
            old.id != a.id || old.job.is_none(),
            "scheduler_unknown: 子任务已经执行，禁止重复"
        );
    }
    a.job = Some(job);
    a.reservation = Some(budget::with_ledger(
        &cfg.budget.ledger,
        &cfg.budget.campaign_id,
        |l| budget::register_generated(l, job, &cfg.resources),
    )?);
    a.state = "running".into();
    store::write(&active(&cfg.work_dir), a)?;
    let result = (|| -> Result<()> {
        if let Some(name) = &task.electronic_leaf {
            return super::workspace_science::run_electronic_leaf(&cfg, name);
        }
        // 恢复该次提交的原生输入，避免日志目录里展示另一份请求。
        let frozen = PathBuf::from(task.argv.last().context("data_invalid: 子任务空命令")?);
        fs::copy(frozen, &task.input.path)?;
        let report = super::process::run_report(&task.argv, &task.directory)?;
        report.copy_logs(&task.directory, &task.stem)?;
        fs::write(
            task.directory.join(format!("{}.exitcode", task.stem)),
            report
                .exit_status
                .map(|x| x.to_string())
                .unwrap_or_else(|| "signal".into()),
        )?;
        ensure!(
            report.exit_status == Some(0),
            "execution_failed: 子任务退出失败"
        );
        if task.qe {
            qf_io::qe_completion::checked_log(
                &task.directory.join(format!("{}.out", task.stem)),
                task.scf,
            )?;
        } else {
            crate::interpolation::native::checked_log(
                &fs::read_to_string(task.directory.join(format!("{}.out", task.stem)))?,
                0,
            )?;
        }
        Ok(())
    })();
    a.state = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    }
    .into();
    a.error = result.as_ref().err().map(|e| format!("{e:#}"));
    store::write(&active(&cfg.work_dir), a)?;
    store::write(path, &task)?;
    result
}
