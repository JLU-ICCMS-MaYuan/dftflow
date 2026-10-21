//! 三程序的脚本与依赖提交；科学阶段、质量和预算仍由各程序拥有。
use super::{
    config::Config,
    slurm, snapshot, store,
    workflow::{self, Boundary},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Chain {
    schema_version: u32,
    run_id: String,
    script: store::Identity,
    programs: Vec<store::Identity>,
    stages: Vec<store::Identity>,
    metadata: Vec<store::Identity>,
}
impl Chain {
    fn verify(&self, work: &Path) -> Result<()> {
        let b = snapshot::RunBinding::load(work)?;
        b.verify()?;
        ensure!(
            self.schema_version == 1
                && self.run_id == b.run_id
                && self.programs.len() == 3
                && self.stages.len() == 3
                && self.metadata.len() == 3,
            "input_conflict: 总提交脚本身份不符"
        );
        self.script.verify()?;
        for id in self
            .programs
            .iter()
            .chain(&self.stages)
            .chain(&self.metadata)
        {
            id.verify()?;
        }
        ensure!(
            self.programs[0].path == std::env::current_exe()?,
            "input_conflict: 总提交程序身份不符"
        );
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    job_id: u64,
    cluster: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    state: String,
    submitting_phase: Option<usize>,
    jobs: Vec<Job>,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    phase: usize,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn manifest(work: &Path) -> PathBuf {
    work.join("epw-state/generated/chain.json")
}

/// 默认只生成；三个程序必须来自同一构建目录。
pub fn generate(config: &Path) -> Result<PathBuf> {
    let cfg = Config::load(config)?;
    let b = snapshot::prepare(&cfg, config)?;
    let target = manifest(&b.work_dir);
    let _lock = store::Lock::acquire(&target)?;
    if target.exists() {
        let chain: Chain = store::read(&target)?;
        chain.verify(&b.work_dir)?;
        return Ok(chain.script.path);
    }
    let executable = std::env::current_exe()?;
    let parent = executable.parent().context("environment: 程序缺少父目录")?;
    let mut programs = Vec::new();
    for name in ["epw_phonon", "epw_wannier", "epw_superconductivity"] {
        programs.push(store::Identity::capture(&parent.join(name))?);
    }
    let mut stages = Vec::new();
    let mut metadata = Vec::new();
    for (program, until) in
        programs
            .iter()
            .zip([Boundary::QeData, Boundary::Interpolation, Boundary::All])
    {
        let script = workflow::generate_phase_for(config, until, &program.path)?;
        metadata.push(store::Identity::capture(&script.with_extension("json"))?);
        stages.push(store::Identity::capture(&script)?);
    }
    let script = target.with_file_name("submit-all.sh");
    ensure!(
        !script.exists(),
        "input_conflict: 已有未登记总脚本，保留现场"
    );
    let body = format!(
        "#!/bin/bash\nexec {} --submit-chain -w {}\n",
        slurm::shell_quote(&programs[0].path.to_string_lossy()),
        slurm::shell_quote(&b.work_dir.to_string_lossy())
    );
    store::atomic_bytes(&script, body.as_bytes())?;
    let chain = Chain {
        schema_version: 1,
        run_id: b.run_id,
        script: store::Identity::capture(&script)?,
        programs,
        stages,
        metadata,
    };
    chain.verify(&b.work_dir)?;
    store::write(&target, &chain)?;
    Ok(script)
}

/// 在每次 sbatch 前持久化未决状态；中断或未知回执不能触发重提。
pub fn submit(work: &Path) -> Result<()> {
    let b = snapshot::RunBinding::load(work)?;
    let work = b.work_dir.as_path();
    let chain: Chain = store::read(&manifest(work))?;
    chain.verify(work)?;
    let target = work.join("epw-state/generated/chain-submission.json");
    let _lock = store::Lock::acquire(&target)?;
    if target.exists() {
        let receipt: Receipt = store::read(&target)?;
        ensure!(receipt.schema_version == 1 && receipt.state == "submitted"
            && receipt.submitting_phase.is_none() && receipt.jobs.len() == 3,
            "scheduler_unknown: 总提交存在部分或未知回执，核对 chain-submission.json 后人工处理，禁止自动重提");
        println!(
            "already_submitted: {}",
            serde_json::to_string(&receipt.jobs)?
        );
        return Ok(());
    }
    ensure!(
        !work.join("epw-state/workflow.json").exists(),
        "input_conflict: 已有阶段尝试，请使用对应程序恢复，禁止重新提交总链"
    );
    let mut receipt = Receipt {
        schema_version: 1,
        state: "submitting".into(),
        submitting_phase: None,
        jobs: Vec::new(),
        responses: Vec::new(),
    };
    for (index, stage) in chain.stages.iter().enumerate() {
        // 全链已在提交前验证；每次提交再核对对应脚本与程序，节点入口仍会重验。
        b.verify()?;
        stage.verify()?;
        chain.programs[index].verify()?;
        chain.metadata[index].verify()?;
        receipt.submitting_phase = Some(index + 1);
        store::write(&target, &receipt)?;
        let mut cmd = Command::new("sbatch");
        cmd.args(["--parsable", "--kill-on-invalid-dep=yes"]);
        if let Some(job) = receipt.jobs.last() {
            cmd.arg(format!("--dependency=afterok:{}", job.job_id));
            if let Some(cluster) = &job.cluster {
                cmd.arg(format!("--clusters={cluster}"));
            }
        }
        let out = cmd
            .arg(&stage.path)
            .current_dir(work)
            .output()
            .context("scheduler_unknown: 总提交无法调用 sbatch，保留未决回执")?;
        receipt.responses.push(Response {
            phase: index + 1,
            exit_code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
        store::write(&target, &receipt)?;
        ensure!(
            out.status.success(),
            "scheduler_unknown: 总提交 sbatch 失败，保留已提交作业及未决回执: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let job = slurm::parse_job_id(&String::from_utf8(out.stdout)?)?;
        ensure!(
            receipt
                .jobs
                .last()
                .is_none_or(|previous| previous.cluster == job.cluster)
                && receipt
                    .jobs
                    .iter()
                    .all(|previous| previous.job_id != job.job_id),
            "scheduler_unknown: 重复作业号或跨集群回执，禁止继续提交"
        );
        receipt.jobs.push(Job {
            job_id: job.job_id,
            cluster: job.cluster,
        });
        receipt.submitting_phase = None;
        store::write(&target, &receipt)?;
    }
    receipt.state = "submitted".into();
    store::write(&target, &receipt)?;
    println!(
        "submitted_jobs: {}\nworkflow_complete: false",
        serde_json::to_string(&receipt.jobs)?
    );
    Ok(())
}
