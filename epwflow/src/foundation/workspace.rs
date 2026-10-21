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
}
fn active(root: &Path) -> PathBuf {
    root.join("epw-state/active.json")
}
fn attempt_path(root: &Path, id: &str) -> PathBuf {
    root.join("epw-state/attempts").join(id).join("attempt.json")
}

fn control(cfg: &Config) -> Result<store::Lock> {
    super::run_state::ready(&cfg.work_dir)?;
    let target = cfg.work_dir.join("epw-state/control");
    if cfg.work_dir.join("epw-state/control.lock").exists() && active(&cfg.work_dir).is_file() {
        let old: Attempt = store::read(&active(&cfg.work_dir))?;
        if let Some(job) = old.job {
            store::recover_finished_lock(&target, job)?;
        }
    }
    store::Lock::acquire(&target)
}

fn check_active(cfg: &Config, own: Option<&str>) -> Result<()> {
    if !active(&cfg.work_dir).exists() {
        return Ok(());
    }
    let old: Attempt = store::read(&active(&cfg.work_dir))?;
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
        store::recover_finished_lock(&cfg.work_dir.join("epw-state/execution"), job)?;
        store::recover_finished_lock(
            &Step::Superconductivity.directory(cfg).join("execution"),
            job,
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
    let task = if active(&cfg.work_dir).is_file() {
        Some(store::read::<Attempt>(&active(&cfg.work_dir))?)
    } else {
        None
    };
    Ok(
        serde_json::json!({"operation_status":"ok","workflow_complete":steps.values().all(|v|v=="reuse"),"steps":steps,"attempt":task}),
    )
}
pub fn status(cfg: &Config) -> Result<()> {
    let value = report(cfg)?;
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
    };
    store::write(&dir.join("attempt.json"), &a)?;
    Ok((dir, a))
}

fn script(cfg: &Config, dir: &Path, a: &Attempt) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let q = slurm::shell_quote;
    let r = &cfg.resources;
    let mut text=format!("#!/bin/bash\n#SBATCH --job-name=epw-{}\n#SBATCH --partition={}\n#SBATCH --nodes={}\n#SBATCH --ntasks={}\n#SBATCH --time={}\n#SBATCH --output={}\n#SBATCH --error={}\ncd -- {} || exit $?\n",
        a.target.name(),r.partition,r.nodes,r.mpi,(r.wall_seconds+59)/60,
        dir.join("slurm-%j.out").display(),dir.join("slurm-%j.err").display(),q(&dir.to_string_lossy()));
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
    let _guard = control(&cfg)?;
    check_active(&cfg, None)?;
    generate_locked(config, target, &cfg)
}

fn generate_locked(config: &Path, target: Step, cfg: &Config) -> Result<PathBuf> {
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

pub fn start(config: &Path, target: Step, submit: bool) -> Result<()> {
    let cfg = Config::load(config)?;
    let _guard = control(&cfg)?;
    check_active(&cfg, None)?;
    let script_path = generate_locked(config, target, &cfg)?;
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
        drop(_guard);
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
    let mut a: Attempt = store::read(path)?;
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
        let _guard = control(&cfg)?;
        check_active(&cfg, Some(&a.id))?;
        if active(&cfg.work_dir).exists() {
            let old: Attempt = store::read(&active(&cfg.work_dir))?;
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
        a.job = Some(job);
        a.state = "running".into();
        persist(&cfg, &a)?;
    }
    let _execution = store::Lock::acquire(&cfg.work_dir.join("epw-state/execution"))?;
    store::atomic_bytes(
        &cfg.work_dir.join("epw-state/current-config.toml"),
        toml::to_string_pretty(&cfg)?.as_bytes(),
    )?;
    let result = (|| -> Result<()> {
        for step in a.target.closure() {
            if stages::reusable(&cfg, step)? {
                println!("reuse: {}", step.name());
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
    result
}

/// 只移动本阶段；成功的相容上游目录不受影响。未知文件保留在历史。
fn archive_step(cfg: &Config, step: Step) -> Result<()> {
    let request = cfg
        .work_dir
        .join("epw-state/requests")
        .join(format!("{}.json", step.name()));
    if request.is_file() && store::read::<String>(&request)? == stages::scientific_key(cfg, step)? {
        let receipt = stages::receipt(&cfg.work_dir, step);
        if step >= Step::Coarse && !receipt.exists() { return Ok(()); }
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
        fs::create_dir_all(&history)?;
        fs::rename(&dir, history.join("calculation"))?;
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
    let _control = control(&cfg)?;
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
    drop(_control);
    let _guard = store::Lock::acquire(&cfg.work_dir.join("epw-state/execution"))?;
    let result = (|| -> Result<()> {
        if let Some(name) = &task.electronic_leaf {
            return super::workspace_science::run_electronic_leaf(&cfg, name);
        }
        // 恢复该次提交的原生输入，避免日志目录里展示另一份请求。
        let frozen = PathBuf::from(task.argv.last().context("data_invalid: 子任务空命令")?);
        fs::copy(frozen, &task.input.path)?;
        let report = super::process::run_report(&task.argv, &task.directory)?;
        fs::copy(
            report.stdout_path,
            task.directory.join(format!("{}.out", task.stem)),
        )?;
        fs::copy(
            report.stderr_path,
            task.directory.join(format!("{}.err", task.stem)),
        )?;
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
