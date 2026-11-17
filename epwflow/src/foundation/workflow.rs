//! 公开执行入口：固定输入、登记分配、依次执行已有科学阶段并验证交付。
use super::{
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
        ensure!(!self.artifacts.is_empty(), "data_invalid: The evidence at the stage is empty");
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
    work.join("epw-state/workflow.json")
}
fn load(work: &Path) -> Result<Record> {
    let r: Record = store::read(&record_path(work))?;
    ensure!(
        r.schema_version == 1,
        "unsupported_version: Workstream version not supported"
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
        std::env::var("SLURM_JOB_ID").context("allocation: run Within resources to be calculated SLURM_JOB_ID")?;
    Ok(slurm::parse_job_id(&value)?.job_id)
}
fn new_record(
    until: Boundary,
    job_id: Option<u64>,
    completed: BTreeMap<String, Completion>,
) -> Result<Record> {
    Ok(Record {
        schema_version: 1,
        attempt_id: store::token(),
        until,
        state: "submission_pending".into(),
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

/// 调用方持有运行锁，按持久作业记录阻止重复提交。
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
            "scheduler_unknown: Attempt {} has no confirmed submission receipt; resubmission prohibited",
            r.attempt_id
        ),
        "submitted" | "running" => {
            ensure!(
                until == r.until,
                "input_conflict: Active --until Unchangeable"
            );
            let a = slurm::accounting(r.job_id.context("scheduler_unknown: Synchronising folder")?)?;
            ensure!(
                !a.terminal,
                "execution_failed: The operation is over, please status --refresh Post Rear resume"
            );
            return Ok((Some(r), BTreeMap::new()));
        }
        "succeeded" if r.until >= until => return Ok((Some(r), BTreeMap::new())),
        "succeeded" => ensure!(retry, "execution_failed: Expand Implementation Borders resume"),
        "failed" => ensure!(
            retry,
            "execution_failed: Previous attempt failed. Please use resume Highlight Retry"
        ),
        _ => bail!("data_invalid: Workflow status invalid {}", r.state),
    }
    if let Some(job) = r.job_id {
        ensure!(
            slurm::accounting(job)?.terminal,
            "scheduler_unknown: Previously assigned to activity, no retrying"
        );
    }
    store::write(
        &work
            .join("epw-state/history")
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
        _ => bail!("input_conflict: Invalidity of the three-procedure boundary"),
    };
    if let Some(name) = prerequisite {
        let r = load(work).context("required: The previous procedure had not been delivered for effective completion of the record")?;
        let done = r.completed.get(name).context("required: The pre-phase is not completed, please run the corresponding application first")?;
        done.verify()?;
        verify_stage(name, &done.manifest.path, &cfg, &binding)?;
    }
    if record_path(work).exists() {
        let r = load(work)?;
        ensure!(retry || r.state != "failed", "execution_failed: Last failure, show off --resume");
        ensure!(r.until <= until, "input_conflict: Prohibition of return to advanced stages");
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
    ensure!(load(work)?.until == until, "input_conflict: Work processes differ from the recording stage");
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
        slurm::shell_quote(path.to_str().context("Environmental scripts are not UTF-8")?)
    ))
}

fn generated_path(work: &Path, until: Boundary) -> PathBuf {
    work.join("epw-state/generated").join(format!("{}.json", until.name()))
}

/// 生成阶段脚本只固定输入，不查询调度器、不运行科学程序。
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
        ensure!(g.schema_version == 1 && g.until == until && g.executable.path == executable_path, "input_conflict: Created a different script identity");
        g.executable.verify()?;
        g.script.verify()?;
        return Ok(g.script.path);
    }
    let executable = store::Identity::capture(executable_path)?;
    let path = target.with_extension("sh");
    ensure!(!path.exists(), "input_conflict: Unregistered script, on-site");
    let quote = slurm::shell_quote;
    let mut body = slurm::script_header(&cfg, path.parent().unwrap(), &format!("epw-{}", until.name()), &format!("{}.%j.out", until.name()), &format!("{}.%j.err", until.name()));
    body.push_str(&format!("export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1\nulimit -s unlimited\ncd {} || exit $?\n", quote(&b.work_dir.to_string_lossy())));
    if let Some(env) = &cfg.programs.environment_script {
        body.push_str(&source_environment(env)?);
    }
    body.push_str(&format!("exec {} --run-script -w {}\n", quote(&executable.path.to_string_lossy()), quote(&b.work_dir.to_string_lossy())));
    store::atomic_bytes(&path, body.as_bytes())?;
    store::write(&target, &Generated { schema_version: 1, until, executable, script: store::Identity::capture(&path)? })?;
    Ok(path)
}

pub fn run_generated(work: &Path, until: Boundary) -> Result<()> {
    let (_cfg, b) = fixed(work)?;
    let g: Generated = store::read(&generated_path(&b.work_dir, until))?;
    ensure!(g.schema_version == 1 && g.until == until && g.executable.path == std::env::current_exe()?, "input_conflict: Generate Script Identity Different");
    g.executable.verify()?;
    g.script.verify()?;
    require_phase(&b.work_dir, until, false)?;
    let job = job_id()?;
    let a = slurm::running_allocation(job)?;
    ensure!(!a.terminal && a.state == "RUNNING", "allocation: Current distribution is not running");
    let extend = record_path(&b.work_dir).exists() && load(&b.work_dir)?.state == "succeeded";
    start_bound(&b.work_dir, until, false, extend)
}
pub fn resume(work: &Path, until: Option<Boundary>, submit: bool, resource: bool) -> Result<()> {
    resume_with_mpi(work, until, submit, resource, None)
}

fn resume_with_mpi(work: &Path, until: Option<Boundary>, submit: bool, resource: bool, mpi: Option<u32>) -> Result<()> {
    ensure!(
        submit != resource,
        "required: resume There's got to be and only a choice -s or -r"
    );
    let (_cfg, b) = fixed(work)?;
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
                calculation.join("interpolation/comparison"),
                calculation.join("results/comparison"),
                calculation.join("results/figures/comparison-figures"),
                b.work_dir.join("results/native-results"),
                b.work_dir.join("results/comparison"),
                b.work_dir.join("results/figures/native-figures"),
                b.work_dir.join("results/figures/comparison-figures"),
                record_path(&b.work_dir),
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
    let (mut cfg, _binding) = fixed(work)?;
    if let Some(mpi) = mpi {
        ensure!(retry && until == Boundary::Interpolation,
            "input_conflict: --mpi For failure only Wannier/Visible resumption of the coarse grid phase");
        let prior = load(work)?;
        ensure!(prior.state == "failed" && !prior.completed.contains_key("wannier"),
            "input_conflict: The running of active or delivered coarse grid cannot change the parallel layout");
        super::parallelism::apply(&mut cfg, mpi)?;
    }
    let direct_job = if submit { None } else { Some(job_id()?) };
    let outcome = (|| -> Result<_> {
        let _lock = store::Lock::acquire(&record_path(work))?;
        let inherited_mpi = if record_path(work).exists() { load(work)?.mpi_override } else { None };
        if mpi.is_some() {
            let prior = load(work)?;
            ensure!(prior.state == "failed" && !prior.completed.contains_key("wannier"),
                "input_conflict: Running advanced, cannot change the parallel layout");
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
        let mut r = new_record(until, direct_job, completed)?;
        r.calculation_dir =
            Some(calculation_dir.unwrap_or_else(|| work.join("calculations").join(&r.attempt_id)));
        r.retry_failed = retry;
        r.mpi_override = mpi.or(inherited_mpi);
        save(work, &r)?;
        Ok((r, true))
    })()?;
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

    // 先持久化尝试，再调用一次 sbatch；响应丢失不能重提。
    let submitted = (|| -> Result<slurm::SubmittedJob> {
        let script = work
            .join("epw-state/slurm")
            .join(format!("{}.sh", record.attempt_id));
        let quote = slurm::shell_quote;
        let mut body = slurm::script_header(&cfg, script.parent().unwrap(), &format!("epw-{}", record.attempt_id), "%j.out", "%j.err");
        body.push_str("export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1\nulimit -s unlimited\n");
        body.push_str(&format!(
            "cd {} || exit $?\n",
            quote(work.to_str().context("_Other Organiser UTF-8")?)
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
                    .context("Program path is not UTF-8")?
            ),
            quote(&record.attempt_id),
            quote(work.to_str().unwrap())
        ));
        store::atomic_bytes(&script, body.as_bytes())?;
        record.script = Some(store::Identity::capture(&script)?);
        save(work, &record)?;
        let output = Command::new("sbatch")
            .arg("--parsable")
            .arg(format!("--comment=epwflow:{}", record.attempt_id))
            .arg(&script)
            .current_dir(work)
            .output()
            .context("scheduler_unknown: Unable to call sbatch ;Check submission attempts first")?;
        store::atomic_bytes(&script.with_extension("stdout"), &output.stdout)?;
        store::atomic_bytes(&script.with_extension("stderr"), &output.stderr)?;
        ensure!(
            output.status.success(),
            "scheduler_unknown: sbatch Exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        slurm::parse_job_id(&String::from_utf8(output.stdout)?)
    })();
    match submitted {
        Ok(job) => {
            (|| -> Result<()> {
                let _lock = store::Lock::acquire(&record_path(work))?;
                let mut current = load(work)?;
                ensure!(
                    current.attempt_id == record.attempt_id,
                    "input_conflict: Submission attempt changed"
                );
                ensure!(
                    current.job_id.is_none_or(|id| id == job.job_id),
                    "scheduler_unknown: This is not consistent with the count node identity"
                );
                current.job_id = Some(job.job_id);
                current.cluster = job.cluster.clone();
                if current.state == "submission_pending" {
                    current.state = "submitted".into();
                }
                save(work, &current)
            })()?;
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
    if qf_io::qe_payload::unchecked(path) { return Ok(()); }
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            files(&entry?.path(), identities)?;
        }
    } else {
        let id = store::Identity::capture(path)?;
        ensure!(
            id.bytes > 0,
            "data_invalid: The necessary product is empty {}",
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
                "data_invalid: QE manifest not passed or source not matched"
            );
            for (el, id) in &binding.pseudos {
                ensure!(
                    m.fingerprints.pseudo_sha256.get(el) == Some(&id.sha256),
                    "input_conflict: QE The source doesn't match"
                );
            }
            let scf = m.scf.context("data_invalid: Missing SCF Data")?;
            ensure!(
                scf.total_energy_ry.is_finite() && scf.success_count == 1,
                "data_invalid: SCF Record invalid"
            );
            files(&scf.snapshot_dir, &mut artifacts)?;
            let s = m.states.context("data_invalid: Missing NSCF Data")?;
            let count: usize = cfg.qe.nscf.k_grid.iter().map(|&n| n as usize).product();
            ensure!(
                s.k_grid == cfg.qe.nscf.k_grid
                    && s.kpoints.len() == count
                    && s.nbnd == cfg.qe.nscf.nbnd as usize,
                "data_invalid: Actual NSCF Grid/band incomplete"
            );
            files(&s.wavefunctions, &mut artifacts)?;
            ensure!(
                !m.qpoints.is_empty() && m.qpoints.len() == m.phonon_files.len(),
                "data_invalid: PH/q Map incomplete"
            );
            for r in m.phonon_files {
                for p in [
                    &r.dyn_file,
                    &r.patterns,
                    &r.control_metadata,
                    r.dvscf.as_ref().context("data_invalid: Lack of response")?,
                ] {
                    files(p, &mut artifacts)?;
                }
            }
        }
        "wannier" => {
            let stage_work = manifest.parent().and_then(Path::parent).context("Manifest path")?;
            let m = crate::interpolation::scheduler::verify_coarse(cfg, stage_work)?;
            ensure!(m.run_id == binding.run_id, "input_conflict: coarse Grid Runs Identity Conflict");
            for a in m.coarse.unwrap().artifacts {
                files(&stage_work.join(a.relative_path), &mut artifacts)?;
            }
        }
        "interpolation" => {
            let m = crate::interpolation::manifest::InterpolationManifest::load(manifest)?;
            ensure!(
                m.validation.all_passed()
                    && !m.spectra.is_empty(),
                "data_invalid: Plugin native data not yet passed"
            );
            // 清单本身不足以证明可重读，沿用专门适配器的校验。
            let stage_work = manifest.parent().and_then(Path::parent).context("Manifest path")?;
            crate::superconductivity::source::bind(stage_work, cfg, &m)?;
            let coarse = m.coarse.context("data_invalid: Missing coarse grid data")?;
            let fine = m.fine.context("data_invalid: Missing fine grid data")?;
            for a in coarse.artifacts.iter().chain(fine.artifacts.iter()) {
                let p = if a.relative_path.is_absolute() {
                    a.relative_path.clone()
                } else {
                    manifest
                        .parent()
                        .and_then(Path::parent)
                        .context("Manifest path")?
                        .join(&a.relative_path)
                };
                if qf_io::qe_payload::unchecked(&p) { continue; }
                ensure!(
                    store::hash_file(&p)? == a.sha256,
                    "input_conflict: The interpolation product Hash doesn't match"
                );
                files(&p, &mut artifacts)?;
            }
        }
        "superconductivity" => {
            let work = manifest
                .parent()
                .and_then(Path::parent)
                .context("Manifest path")?;
            artifacts = crate::superconductivity::verify::complete(work, cfg, &binding.run_id)?;
        }
        "results" => {
            let root = manifest.parent().context("Results root")?;
            ensure!(
                crate::delivery::check_results(root)?["workflow_complete"] == true,
                "data_invalid: Delivery not completed"
            );
            let (m, _) = crate::delivery::native::load(root)?;
            ensure!(
                m.run_id == binding.run_id,
                "input_conflict: Resulting in identity conflict"
            );
            for file in &m.files {
                artifacts.push(store::Identity::capture(&file.verify(root)?)?);
            }
            files(&root.join("figures"), &mut artifacts)?;
        }
        _ => bail!("data_invalid: Unknown stage {name}"),
    }
    ensure!(!artifacts.is_empty(), "data_invalid: Lack of stage evidence");
    Ok(Completion {
        manifest: store::Identity::capture(manifest)?,
        artifacts,
    })
}

pub fn execute(work: &Path, attempt: &str) -> Result<()> {
    let (cfg, binding) = fixed(work)?;
    let work = binding.work_dir.as_path();
    let job = job_id()?;
    let mut record = (|| -> Result<_> {
        let _lock = store::Lock::acquire(&record_path(work))?;
        let mut r = load(work)?;
        ensure!(
            r.attempt_id == attempt
                && matches!(r.state.as_str(), "submission_pending" | "submitted"),
            "input_conflict: Try not to match or have started"
        );
        r.executable.verify()?;
        if let Some(s) = &r.script {
            s.verify()?;
        }
        ensure!(
            r.job_id.is_none_or(|id| id == job),
            "allocation: Not a registered job"
        );
        let a = slurm::running_allocation(job)?;
        ensure!(
            !a.terminal && a.state == "RUNNING",
            "allocation: Current distribution is not running"
        );
        r.job_id = Some(job);
        r.state = "running".into();
        save(work, &r)?;
        Ok(r)
    })()?;
    let calculation = record
        .calculation_dir
        .clone()
        .unwrap_or_else(|| work.join("calculations").join(attempt));
    ensure!(
        calculation.starts_with(work.join("calculations")),
        "data_invalid: Scientific catalogue escapes"
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
                    "input_conflict: Recover phase directory changes"
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
                    crate::superconductivity::scheduler::run_configured(
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
        let _lock = store::Lock::acquire(&record_path(work))?;
        let mut r = load(work)?;
        if let Some(job) = r.job_id {
            let a = slurm::accounting(job)?;
            if a.terminal && matches!(r.state.as_str(), "submitted" | "running") {
                r.state = "failed".into();
                r.error = Some(format!(
                    "execution_failed: Slurm {},Lack of numerically validated stage completion records",
                    a.state
                ));
                save(work, &r)?;
            }
        }
    }
    let r = load(work)?;
    for done in r.completed.values() {
        done.verify()?;
    }
    Ok(
        serde_json::json!({"schema_version":1,"workflow_status":r.state,"workflow_complete":r.state=="succeeded"&&r.until==Boundary::All,
        "boundary_complete":r.state=="succeeded","until":r.until,"job_id":r.job_id,"attempt_id":r.attempt_id,"current_stage":r.current_stage,"error":r.error,"completed_stages":r.completed.keys().collect::<Vec<_>>(),"effective_mpi":super::parallelism::mpi_count(&cfg)?,"mpi_override":r.mpi_override }),
    )
}
