//! 公开执行入口：固定输入、登记分配、依次执行已有科学阶段并验证交付。
use super::{
    budget,
    config::Config,
    slurm,
    snapshot::{self, RunBinding},
    stage::Backend,
    store,
};
use anyhow::{bail, ensure, Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Boundary {
    QeData,
    Interpolation,
    Superconductivity,
    All,
}
impl Boundary {
    pub fn name(self) -> &'static str {
        match self {
            Self::QeData => "qe-data",
            Self::Interpolation => "interpolation",
            Self::Superconductivity => "superconductivity",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    manifest: store::Identity,
    artifacts: Vec<store::Identity>,
}
impl Completion {
    fn verify(&self) -> Result<()> {
        self.manifest.verify()?;
        ensure!(!self.artifacts.is_empty(), "data_invalid: 阶段证据为空");
        for a in &self.artifacts {
            a.verify()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    attempt_id: String,
    until: Boundary,
    state: String,
    reservation: String,
    job_id: Option<u64>,
    cluster: Option<String>,
    executable: store::Identity,
    script: Option<store::Identity>,
    completed: BTreeMap<String, Completion>,
    current_stage: Option<String>,
    error: Option<String>,
    #[serde(default)]
    calculation_dir: Option<PathBuf>,
    #[serde(default)]
    retry_failed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mpi_override: Option<u32>,
}
fn record_path(work: &Path) -> PathBuf {
    work.join(".epwflow/workflow.json")
}
fn load(work: &Path) -> Result<Record> {
    let r: Record = store::read(&record_path(work))?;
    ensure!(
        r.schema_version == 1,
        "unsupported_version: 工作流记录版本不受支持"
    );
    Ok(r)
}
fn save(work: &Path, r: &Record) -> Result<()> {
    store::write(&record_path(work), r)
}
fn fixed(work: &Path) -> Result<(Config, RunBinding)> {
    let b = RunBinding::load(work)?;
    b.verify()?;
    let mut c = Config::load(&b.work_dir.join("input-init.toml"))?;
    if record_path(&b.work_dir).exists() {
        if let Some(mpi) = load(&b.work_dir)?.mpi_override {
            super::parallelism::apply(&mut c, mpi)?;
        }
    }
    Ok((c, b))
}
fn job_id() -> Result<u64> {
    let value =
        std::env::var("SLURM_JOB_ID").context("allocation: run 需要计算资源内的 SLURM_JOB_ID")?;
    Ok(slurm::parse_job_id(&value)?.job_id)
}
fn new_record(
    until: Boundary,
    reservation: String,
    job_id: Option<u64>,
    completed: BTreeMap<String, Completion>,
) -> Result<Record> {
    Ok(Record {
        schema_version: 1,
        attempt_id: store::token(),
        until,
        state: "submission_pending".into(),
        reservation,
        job_id,
        cluster: None,
        executable: store::Identity::capture(&std::env::current_exe()?)?,
        script: None,
        completed,
        current_stage: None,
        error: None,
        calculation_dir: None,
        retry_failed: false,
        mpi_override: None,
    })
}

/// 所有账本/运行联合更新均先取账本锁、再取运行锁。
fn previous(
    work: &Path,
    until: Boundary,
    retry: bool,
) -> Result<(Option<Record>, BTreeMap<String, Completion>)> {
    if !record_path(work).exists() {
        return Ok((None, BTreeMap::new()));
    }
    let r = load(work)?;
    for c in r.completed.values() {
        c.verify()?;
    }
    match r.state.as_str() {
        "submission_pending" | "unknown" => bail!(
            "scheduler_unknown: 尝试 {} 的提交未确认，保留预留，禁止重提",
            r.attempt_id
        ),
        "submitted" | "running" => {
            ensure!(
                until == r.until,
                "input_conflict: 活动作业的 --until 不可更改"
            );
            let a = slurm::accounting(r.job_id.context("scheduler_unknown: 缺作业号")?)?;
            ensure!(
                !a.terminal,
                "execution_failed: 作业已结束，请 status --refresh 后显式 resume"
            );
            return Ok((Some(r), BTreeMap::new()));
        }
        "succeeded" if r.until >= until => return Ok((Some(r), BTreeMap::new())),
        "succeeded" => ensure!(retry, "execution_failed: 扩展执行边界请使用 resume"),
        "failed" => ensure!(
            retry,
            "execution_failed: 之前尝试失败，请用 resume 显式重试"
        ),
        _ => bail!("data_invalid: 工作流状态无效 {}", r.state),
    }
    if let Some(job) = r.job_id {
        ensure!(
            slurm::accounting(job)?.terminal,
            "scheduler_unknown: 之前分配仍活动，禁止重试"
        );
    }
    store::write(
        &work
            .join(".epwflow/history")
            .join(format!("{}.json", r.attempt_id)),
        &r,
    )?;
    Ok((None, r.completed))
}

pub fn start(config_path: &Path, until: Boundary, submit: bool) -> Result<()> {
    // 必须先校验实际配置，不能在加载前无条件返回 not_implemented。
    let cfg = Config::load(config_path)?;
    let b = snapshot::prepare(&cfg, config_path)?;
    start_bound(&b.work_dir, until, submit, false)
}

fn require_phase(work: &Path, until: Boundary, retry: bool) -> Result<()> {
    let (cfg, binding) = fixed(work)?;
    let prerequisite = match until {
        Boundary::QeData => None,
        Boundary::Interpolation => Some("qe-data"),
        Boundary::All => Some("wannier"),
        _ => bail!("input_conflict: 三程序阶段边界无效"),
    };
    if let Some(name) = prerequisite {
        let r = load(work).context("required: 前一个程序尚未交付有效完成记录")?;
        let done = r.completed.get(name).context("required: 前置阶段未完成，请先运行对应程序")?;
        done.verify()?;
        verify_stage(name, &done.manifest.path, &cfg, &binding)?;
    }
    if record_path(work).exists() {
        let r = load(work)?;
        ensure!(retry || r.state != "failed", "execution_failed: 上次失败，请显式 --resume");
        ensure!(r.until <= until, "input_conflict: 禁止退回已经推进的阶段");
    }
    Ok(())
}

pub fn start_phase(config: &Path, until: Boundary, submit: bool) -> Result<()> {
    let cfg = Config::load(config)?;
    let b = snapshot::prepare(&cfg, config)?;
    require_phase(&b.work_dir, until, false)?;
    let extend = record_path(&b.work_dir).exists() && load(&b.work_dir)?.state == "succeeded";
    start_bound(&b.work_dir, until, submit, extend)
}

pub fn resume_phase(work: &Path, until: Boundary, submit: bool, resource: bool) -> Result<()> {
    resume_phase_with_mpi(work, until, submit, resource, None)
}

pub fn resume_phase_with_mpi(work: &Path, until: Boundary, submit: bool, resource: bool, mpi: Option<u32>) -> Result<()> {
    require_phase(work, until, true)?;
    resume_with_mpi(work, Some(until), submit, resource, mpi)
}

pub fn execute_phase(work: &Path, attempt: &str, until: Boundary) -> Result<()> {
    ensure!(load(work)?.until == until, "input_conflict: 工作程序与记录阶段不同");
    execute(work, attempt)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Generated {
    schema_version: u32,
    until: Boundary,
    executable: store::Identity,
    script: store::Identity,
}

// 不改变 shell 选项；环境脚本加载失败时显式传播退出码。
pub(super) fn source_environment(path: &Path) -> Result<String> {
    Ok(format!(
        "source {} --force || exit $?\n",
        slurm::shell_quote(path.to_str().context("环境脚本不是 UTF-8")?)
    ))
}

fn generated_path(work: &Path, until: Boundary) -> PathBuf {
    work.join(".epwflow/generated").join(format!("{}.json", until.name()))
}

/// 生成阶段脚本只固定输入，不查询调度器、不预留预算、不运行科学程序。
pub fn generate_phase(config: &Path, until: Boundary) -> Result<PathBuf> {
    generate_phase_for(config, until, &std::env::current_exe()?)
}

pub(super) fn generate_phase_for(config: &Path, until: Boundary, executable_path: &Path) -> Result<PathBuf> {
    let cfg = Config::load(config)?;
    let b = snapshot::prepare(&cfg, config)?;
    let target = generated_path(&b.work_dir, until);
    let _lock = store::Lock::acquire(&target)?;
    if target.exists() {
        let g: Generated = store::read(&target)?;
        ensure!(g.schema_version == 1 && g.until == until && g.executable.path == executable_path, "input_conflict: 已生成脚本身份不同");
        g.executable.verify()?;
        g.script.verify()?;
        return Ok(g.script.path);
    }
    let executable = store::Identity::capture(executable_path)?;
    let path = target.with_extension("sh");
    ensure!(!path.exists(), "input_conflict: 已有未登记脚本，保留现场");
    let r = &cfg.resources;
    let quote = slurm::shell_quote;
    let mut body = format!("#!/bin/bash\n#SBATCH --partition={}\n#SBATCH --nodes={}\n#SBATCH --ntasks={}\n#SBATCH --time={}:{:02}:{:02}\n#SBATCH --output={}\n#SBATCH --error={}\nexport OMP_NUM_THREADS=1 MKL_NUM_THREADS=1\nulimit -s unlimited\ncd {} || exit $?\n",
        r.partition, r.nodes, r.mpi, r.wall_seconds / 3600, r.wall_seconds / 60 % 60, r.wall_seconds % 60,
        quote(&path.with_extension("%j.out").to_string_lossy()), quote(&path.with_extension("%j.err").to_string_lossy()), quote(&b.work_dir.to_string_lossy()));
    if let Some(env) = &cfg.programs.environment_script {
        body.push_str(&source_environment(env)?);
    }
    body.push_str(&format!("exec {} --run-script -w {}\n", quote(&executable.path.to_string_lossy()), quote(&b.work_dir.to_string_lossy())));
    store::atomic_bytes(&path, body.as_bytes())?;
    store::write(&target, &Generated { schema_version: 1, until, executable, script: store::Identity::capture(&path)? })?;
    Ok(path)
}

pub fn run_generated(work: &Path, until: Boundary) -> Result<()> {
    let (cfg, b) = fixed(work)?;
    let g: Generated = store::read(&generated_path(&b.work_dir, until))?;
    ensure!(g.schema_version == 1 && g.until == until && g.executable.path == std::env::current_exe()?, "input_conflict: 生成脚本身份不同");
    g.executable.verify()?;
    g.script.verify()?;
    require_phase(&b.work_dir, until, false)?;
    let job = job_id()?;
    let a = slurm::running_allocation(job)?;
    ensure!(!a.terminal && a.state == "RUNNING" && a.nodes == cfg.resources.nodes as u64, "allocation: 当前分配状态或节点数不符");
    budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
        budget::reconcile(ledger)?;
        budget::register_generated(ledger, job, &cfg.resources).map(|_| ())
    })?;
    let extend = record_path(&b.work_dir).exists() && load(&b.work_dir)?.state == "succeeded";
    start_bound(&b.work_dir, until, false, extend)
}
pub fn resume(work: &Path, until: Option<Boundary>, submit: bool, resource: bool) -> Result<()> {
    resume_with_mpi(work, until, submit, resource, None)
}

fn resume_with_mpi(work: &Path, until: Option<Boundary>, submit: bool, resource: bool, mpi: Option<u32>) -> Result<()> {
    ensure!(
        submit != resource,
        "required: resume 必须且只能选择 -s 或 -r"
    );
    let (cfg, b) = fixed(work)?;
    let boundary = until.unwrap_or(if record_path(&b.work_dir).exists() {
        load(&b.work_dir)?.until
    } else {
        Boundary::All
    });
    if record_path(&b.work_dir).exists() {
        let prior = load(&b.work_dir)?;
        if let Some(job) = prior.job_id {
            let calculation = prior
                .calculation_dir
                .unwrap_or_else(|| b.work_dir.join("calculations").join(prior.attempt_id));
            for target in [
                b.work_dir.join(".epwflow/execution"),
                calculation.join("superconductivity/execution"),
                calculation.join("interpolation/comparison"),
                calculation.join("results/comparison"),
                calculation.join("results/figures/comparison-figures"),
                b.work_dir.join("results/native-results"),
                b.work_dir.join("results/comparison"),
                b.work_dir.join("results/figures/native-figures"),
                b.work_dir.join("results/figures/comparison-figures"),
                record_path(&b.work_dir),
                cfg.budget.ledger.clone(),
            ] {
                store::recover_finished_lock(&target, job)?;
            }
        }
    }
    start_bound_with_mpi(&b.work_dir, boundary, submit, true, mpi)
}

fn start_bound(work: &Path, until: Boundary, submit: bool, retry: bool) -> Result<()> {
    start_bound_with_mpi(work, until, submit, retry, None)
}

fn start_bound_with_mpi(work: &Path, until: Boundary, submit: bool, retry: bool, mpi: Option<u32>) -> Result<()> {
    let (mut cfg, b) = fixed(work)?;
    if let Some(mpi) = mpi {
        ensure!(retry && until == Boundary::Interpolation,
            "input_conflict: --mpi 仅用于失败 Wannier/粗网格阶段的显式恢复");
        let prior = load(work)?;
        ensure!(prior.state == "failed" && !prior.completed.contains_key("wannier"),
            "input_conflict: 活动或已交付粗网格的运行不能改变并行布局");
        super::parallelism::apply(&mut cfg, mpi)?;
    }
    let direct_job = if submit { None } else { Some(job_id()?) };
    let outcome = budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
        let _lock = store::Lock::acquire(&record_path(work))?;
        let inherited_mpi = if record_path(work).exists() { load(work)?.mpi_override } else { None };
        if mpi.is_some() {
            let prior = load(work)?;
            ensure!(prior.state == "failed" && !prior.completed.contains_key("wannier"),
                "input_conflict: 运行已推进，不能改变并行布局");
        }
        let calculation_dir = if retry && record_path(work).exists() {
            let previous = load(work)?;
            Some(
                previous
                    .calculation_dir
                    .unwrap_or_else(|| work.join("calculations").join(previous.attempt_id)),
            )
        } else {
            None
        };
        let (existing, completed) = previous(work, until, retry)?;
        if let Some(existing) = existing {
            return Ok((existing, false));
        }
        // 收到终态后才释放旧预留；查询失败时保守拒绝新任务。
        budget::reconcile(ledger)?;
        let token = if let Some(job) = direct_job {
            budget::registered_allocation(ledger, job, &cfg.resources)?
        } else {
            budget::reserve_workflow(ledger, &format!("workflow:{}", b.run_id), &cfg.resources)?
        };
        let mut r = new_record(until, token, direct_job, completed)?;
        r.calculation_dir =
            Some(calculation_dir.unwrap_or_else(|| work.join("calculations").join(&r.attempt_id)));
        r.retry_failed = retry;
        r.mpi_override = mpi.or(inherited_mpi);
        save(work, &r)?;
        Ok((r, true))
    })?;
    let (mut record, fresh) = outcome;
    if !fresh {
        println!(
            "workflow_status: {}\nuntil: {}\njob_id: {:?}\nworkflow_complete: {}",
            record.state,
            record.until.name(),
            record.job_id,
            record.state == "succeeded" && record.until == Boundary::All
        );
        return Ok(());
    }
    if !submit {
        return execute(work, &record.attempt_id);
    }

    // 先持久化预留与尝试，再调用一次 sbatch；响应丢失不能退款或重提。
    let submitted = (|| -> Result<slurm::SubmittedJob> {
        let script = work
            .join(".epwflow/slurm")
            .join(format!("{}.sh", record.attempt_id));
        let quote = slurm::shell_quote;
        let mut body = String::from("#!/bin/bash\nexport OMP_NUM_THREADS=1 MKL_NUM_THREADS=1\nulimit -s unlimited\n");
        body.push_str(&format!(
            "cd {} || exit $?\n",
            quote(work.to_str().context("工作目录不是 UTF-8")?)
        ));
        if let Some(env) = &cfg.programs.environment_script {
            body.push_str(&source_environment(env)?);
        }
        body.push_str(&format!(
            "exec {} --execute-attempt {} -w {}\n",
            quote(
                record
                    .executable
                    .path
                    .to_str()
                    .context("程序路径不是 UTF-8")?
            ),
            quote(&record.attempt_id),
            quote(work.to_str().unwrap())
        ));
        store::atomic_bytes(&script, body.as_bytes())?;
        record.script = Some(store::Identity::capture(&script)?);
        save(work, &record)?;
        let r = &cfg.resources;
        let output = Command::new("sbatch")
            .arg("--parsable")
            .args([
                "--partition",
                &r.partition,
                "--nodes",
                &r.nodes.to_string(),
                "--ntasks",
                &r.mpi.to_string(),
                "--time",
                &format!(
                    "{}:{:02}:{:02}",
                    r.wall_seconds / 3600,
                    r.wall_seconds / 60 % 60,
                    r.wall_seconds % 60
                ),
            ])
            .arg(format!("--job-name=epw-{}", record.attempt_id))
            .arg(format!("--comment=epwflow:{}", record.attempt_id))
            .arg(format!(
                "--output={}",
                work.join(".epwflow/slurm/%j.out").display()
            ))
            .arg(format!(
                "--error={}",
                work.join(".epwflow/slurm/%j.err").display()
            ))
            .arg(&script)
            .current_dir(work)
            .output()
            .context("scheduler_unknown: 无法调用 sbatch；先核对提交尝试")?;
        store::atomic_bytes(&script.with_extension("stdout"), &output.stdout)?;
        store::atomic_bytes(&script.with_extension("stderr"), &output.stderr)?;
        ensure!(
            output.status.success(),
            "scheduler_unknown: sbatch 退出 {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        slurm::parse_job_id(&String::from_utf8(output.stdout)?)
    })();
    match submitted {
        Ok(job) => {
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
                let _lock = store::Lock::acquire(&record_path(work))?;
                let mut current = load(work)?;
                ensure!(
                    current.attempt_id == record.attempt_id,
                    "input_conflict: 提交尝试已改变"
                );
                ensure!(
                    current.job_id.is_none_or(|id| id == job.job_id),
                    "scheduler_unknown: 作业回执与计算节点身份不一致"
                );
                budget::attach_once(ledger, &current.reservation, job.job_id)?;
                current.job_id = Some(job.job_id);
                current.cluster = job.cluster.clone();
                if current.state == "submission_pending" {
                    current.state = "submitted".into();
                }
                save(work, &current)
            })?;
            println!(
                "submitted: job_id={} until={}\nworkflow_complete: false",
                job.job_id,
                until.name()
            );
            Ok(())
        }
        Err(error) => {
            let _lock = store::Lock::acquire(&record_path(work))?;
            let mut current = load(work)?;
            if current.state == "submission_pending" {
                current.state = "unknown".into();
            }
            current.error = Some(format!("{error:#}"));
            save(work, &current)?;
            Err(error)
        }
    }
}

fn files(path: &Path, identities: &mut Vec<store::Identity>) -> Result<()> {
    if path.is_dir() {
        let before = identities.len();
        for entry in fs::read_dir(path)? {
            files(&entry?.path(), identities)?;
        }
        ensure!(
            identities.len() > before,
            "data_invalid: 保存树为空 {}",
            path.display()
        );
    } else {
        let id = store::Identity::capture(path)?;
        ensure!(
            id.bytes > 0,
            "data_invalid: 必需产物为空 {}",
            path.display()
        );
        identities.push(id);
    }
    Ok(())
}

fn verify_stage(
    name: &str,
    manifest: &Path,
    cfg: &Config,
    binding: &RunBinding,
) -> Result<Completion> {
    let mut artifacts = Vec::new();
    match name {
        "qe-data" => {
            let m = crate::qe_data::manifest::QeManifest::load(manifest)?;
            m.verify()?;
            ensure!(
                m.validation.all_passed()
                    && m.run_id == binding.run_id
                    && m.fingerprints.structure_sha256 == binding.structure.sha256,
                "data_invalid: QE 清单未通过或来源不匹配"
            );
            for (el, id) in &binding.pseudos {
                ensure!(
                    m.fingerprints.pseudo_sha256.get(el) == Some(&id.sha256),
                    "input_conflict: QE 赝势来源不匹配"
                );
            }
            let scf = m.scf.context("data_invalid: 缺少 SCF 数据")?;
            ensure!(
                scf.total_energy_ry.is_finite() && scf.success_count == 1,
                "data_invalid: SCF 记录无效"
            );
            files(&scf.snapshot_dir, &mut artifacts)?;
            let s = m.states.context("data_invalid: 缺少 NSCF 数据")?;
            let count: usize = cfg.qe.nscf.k_grid.iter().map(|&n| n as usize).product();
            ensure!(
                s.k_grid == cfg.qe.nscf.k_grid
                    && s.kpoints.len() == count
                    && s.nbnd == cfg.qe.nscf.nbnd as usize,
                "data_invalid: 实际 NSCF 网格/带数不完整"
            );
            files(&s.wavefunctions, &mut artifacts)?;
            ensure!(
                !m.qpoints.is_empty() && m.qpoints.len() == m.phonon_files.len(),
                "data_invalid: PH/q 映射不完整"
            );
            for r in m.phonon_files {
                for p in [
                    &r.dyn_file,
                    &r.patterns,
                    &r.control_metadata,
                    r.dvscf.as_ref().context("data_invalid: 缺响应势")?,
                ] {
                    files(p, &mut artifacts)?;
                }
            }
        }
        "wannier" => {
            let stage_work = manifest.parent().and_then(Path::parent).context("清单路径")?;
            let m = crate::interpolation::scheduler::verify_coarse(cfg, stage_work)?;
            ensure!(m.run_id == binding.run_id, "input_conflict: 粗网格运行身份冲突");
            for a in m.coarse.unwrap().artifacts {
                files(&stage_work.join(a.relative_path), &mut artifacts)?;
            }
        }
        "interpolation" => {
            let m = crate::interpolation::manifest::InterpolationManifest::load(manifest)?;
            ensure!(
                m.validation.all_passed()
                    && m.quality.as_ref().is_some_and(|q| q.status == "valid")
                    && !m.spectra.is_empty(),
                "data_invalid: 插值质量/原生数据尚未通过"
            );
            // 清单本身不足以证明可重读，沿用专门适配器的校验。
            let stage_work = manifest.parent().and_then(Path::parent).context("清单路径")?;
            ensure!(crate::interpolation::quality_binding::check(cfg, stage_work)?.passed, "data_invalid: 当前粗网格质量失败");
            crate::superconductivity::source::bind(stage_work, cfg, &m)?;
            let coarse = m.coarse.context("data_invalid: 缺少粗网格数据")?;
            let fine = m.fine.context("data_invalid: 缺少细网格数据")?;
            for a in coarse.artifacts.iter().chain(fine.artifacts.iter()) {
                let p = if a.relative_path.is_absolute() {
                    a.relative_path.clone()
                } else {
                    manifest
                        .parent()
                        .and_then(Path::parent)
                        .context("清单路径")?
                        .join(&a.relative_path)
                };
                ensure!(
                    store::hash_file(&p)? == a.sha256,
                    "input_conflict: 插值产物哈希不匹配"
                );
                files(&p, &mut artifacts)?;
            }
        }
        "superconductivity" => {
            let work = manifest
                .parent()
                .and_then(Path::parent)
                .context("清单路径")?;
            artifacts = crate::superconductivity::verify::complete(work, cfg, &binding.run_id)?;
        }
        "results" => {
            let root = manifest.parent().context("结果根目录")?;
            ensure!(
                crate::delivery::check_results(root)?["workflow_complete"] == true,
                "data_invalid: 交付未完成"
            );
            let (m, _) = crate::delivery::native::load(root)?;
            ensure!(
                m.run_id == binding.run_id,
                "input_conflict: 结果运行身份冲突"
            );
            for file in &m.files {
                artifacts.push(store::Identity::capture(&file.verify(root)?)?);
            }
            files(&root.join("figures"), &mut artifacts)?;
        }
        _ => bail!("data_invalid: 未知阶段 {name}"),
    }
    ensure!(!artifacts.is_empty(), "data_invalid: 缺少阶段证据");
    Ok(Completion {
        manifest: store::Identity::capture(manifest)?,
        artifacts,
    })
}

pub fn execute(work: &Path, attempt: &str) -> Result<()> {
    let (cfg, binding) = fixed(work)?;
    let work = binding.work_dir.as_path();
    let job = job_id()?;
    let _execution = store::Lock::acquire(&work.join(".epwflow/execution"))?;
    let mut record = budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
        let _lock = store::Lock::acquire(&record_path(work))?;
        let mut r = load(work)?;
        ensure!(
            r.attempt_id == attempt
                && matches!(r.state.as_str(), "submission_pending" | "submitted"),
            "input_conflict: 尝试不匹配或已经启动"
        );
        r.executable.verify()?;
        if let Some(s) = &r.script {
            s.verify()?;
        }
        ensure!(
            r.job_id.is_none_or(|id| id == job),
            "allocation: 不是已登记的作业"
        );
        budget::attach_once(ledger, &r.reservation, job)?;
        budget::registered_allocation(ledger, job, &cfg.resources)?;
        let a = slurm::running_allocation(job)?;
        ensure!(
            !a.terminal && a.state == "RUNNING" && a.nodes == cfg.resources.nodes as u64,
            "allocation: 当前分配状态或节点数与固定请求不符"
        );
        r.job_id = Some(job);
        r.state = "running".into();
        save(work, &r)?;
        Ok(r)
    })?;
    let calculation = record
        .calculation_dir
        .clone()
        .unwrap_or_else(|| work.join("calculations").join(attempt));
    ensure!(
        calculation.starts_with(work.join("calculations")),
        "data_invalid: 科学目录逃逸"
    );
    fs::create_dir_all(&calculation)?;
    let result = (|| -> Result<()> {
        for (name, boundary) in [
            ("qe-data", Boundary::QeData),
            ("wannier", Boundary::Interpolation),
            ("interpolation", Boundary::Superconductivity),
            ("superconductivity", Boundary::Superconductivity),
            ("results", Boundary::All),
        ] {
            if boundary > record.until {
                break;
            }
            let manifest = if name == "results" {
                work.join("results/manifest.json")
            } else {
                calculation.join(name).join("manifest.json")
            };
            if let Some(done) = record.completed.get(name) {
                done.verify()?;
                ensure!(
                    manifest == done.manifest.path,
                    "input_conflict: 恢复阶段目录改变"
                );
                verify_stage(name, &manifest, &cfg, &binding)?;
                continue;
            }
            // F4 的合法 partial 由原有 Engine 续算；冲突证据不能绕过校验。
            let partial_solver = record.retry_failed && name == "superconductivity"
                && manifest.exists()
                && !crate::superconductivity::verify::published_complete(
                    &calculation, &cfg, &binding.run_id,
                )?;
            // F5 先收集再出图；未登记的 results 必须由 F5 接续完整交付。
            if record.retry_failed && name != "results" && !partial_solver && manifest.exists() {
                let completion = verify_stage(name, &manifest, &cfg, &binding)?;
                record.completed.insert(name.into(), completion);
                save(work, &record)?;
                let receipt = crate::foundation::layout::path(&work,name).join("manifest.json");
                if receipt != manifest { store::atomic_bytes(&receipt, &fs::read(&manifest)?)?; }
                continue;
            }
            record.current_stage = Some(name.into());
            save(work, &record)?;
            match name {
                "qe-data" => {
                    let qe = crate::qe_data::qe_config::QeSettings::from_config(&cfg)?;
                    crate::qe_data::scheduler::run_qeflow_registered(
                        &binding,
                        &cfg,
                        &qe,
                        Backend::Local,
                        &cfg.programs.launcher,
                        &calculation,
                    )?;
                }
                "wannier" => {
                    crate::interpolation::scheduler::run_coarse_with_retry(
                        &cfg, Backend::Local, &cfg.programs.launcher, &calculation, &calculation, record.retry_failed,
                    )?;
                }
                "interpolation" => {
                    crate::interpolation::scheduler::run_fine_with_retry(
                        &cfg, Backend::Local, &cfg.programs.launcher, &calculation, record.retry_failed,
                    )?;
                }
                "superconductivity" => {
                    crate::superconductivity::scheduler::run_with_retry(
                        &cfg,
                        &calculation,
                        &cfg.programs.launcher,
                        Backend::Local,
                        record.retry_failed,
                    )?;
                }
                _ => {
                    crate::delivery::finalize_to(&calculation, Some(&crate::foundation::layout::path(work,"results")))?;
                }
            }
            let completion = verify_stage(name, &manifest, &cfg, &binding)?;
            record.completed.insert(name.into(), completion);
            save(work, &record)?;
            let receipt = crate::foundation::layout::path(&work,name).join("manifest.json");
            if receipt != manifest {
                store::atomic_bytes(&receipt, &fs::read(&manifest)?)?;
            }
        }
        Ok(())
    })();
    record.state = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    }
    .into();
    record.error = result.as_ref().err().map(|e| format!("{e:#}"));
    save(work, &record)?;
    result?;
    println!(
        "boundary_complete: {}\nworkflow_complete: {}",
        record.until.name(),
        record.until == Boundary::All
    );
    Ok(())
}

pub fn status(work: &Path, refresh: bool) -> Result<serde_json::Value> {
    let (cfg, b) = fixed(work)?;
    let work = b.work_dir.as_path();
    if !record_path(&b.work_dir).exists() {
        return snapshot::status(work);
    }
    if refresh {
        budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
            let _lock = store::Lock::acquire(&record_path(work))?;
            let mut r = load(work)?;
            if let Some(job) = r.job_id {
                let a = slurm::accounting(job)?;
                if a.terminal && matches!(r.state.as_str(), "submitted" | "running") {
                    r.state = "failed".into();
                    r.error = Some(format!(
                        "execution_failed: Slurm {}，缺少经过数值验证的阶段完成记录",
                        a.state
                    ));
                    save(work, &r)?;
                }
            }
            budget::reconcile(ledger)?;
            Ok(())
        })?;
    }
    let r = load(work)?;
    for done in r.completed.values() {
        done.verify()?;
    }
    Ok(
        serde_json::json!({"schema_version":1,"workflow_status":r.state,"workflow_complete":r.state=="succeeded"&&r.until==Boundary::All,
        "boundary_complete":r.state=="succeeded","until":r.until,"job_id":r.job_id,"attempt_id":r.attempt_id,"current_stage":r.current_stage,"error":r.error,"completed_stages":r.completed.keys().collect::<Vec<_>>(),"effective_mpi":cfg.resources.mpi,"mpi_override":r.mpi_override }),
    )
}
