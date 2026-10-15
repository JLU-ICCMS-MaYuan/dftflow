use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use super::{process, slurm, snapshot, state};

/// 执行后端：本地直接运行或经 Slurm 提交。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Local,
    Slurm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramKey {
    Pw,
    Ph,
    Epw,
}

/// 阶段定义：id、程序、依赖与期望产物；F2–F4 逐步填充实际命令与产物。
#[derive(Debug, Clone)]
pub struct StageDef {
    pub id: &'static str,
    pub program: Option<ProgramKey>,
    pub dependencies: &'static [&'static str],
    pub args: &'static [&'static str],
    pub expected_artifacts: &'static [&'static str],
}

pub fn pipeline() -> Vec<StageDef> {
    vec![
        StageDef { id: "scf", program: Some(ProgramKey::Pw), dependencies: &[], args: &[], expected_artifacts: &["scf.out"] },
        StageDef { id: "nscf", program: Some(ProgramKey::Pw), dependencies: &["scf"], args: &[], expected_artifacts: &["nscf.out"] },
        StageDef { id: "phonon", program: Some(ProgramKey::Ph), dependencies: &["nscf"], args: &[], expected_artifacts: &["phonon.out"] },
        StageDef { id: "wannier", program: None, dependencies: &["phonon"], args: &[], expected_artifacts: &["wannier.out"] },
        StageDef { id: "coarse_epw", program: Some(ProgramKey::Epw), dependencies: &["wannier"], args: &[], expected_artifacts: &["coarse.out"] },
        StageDef { id: "fine_epw", program: Some(ProgramKey::Epw), dependencies: &["coarse_epw"], args: &[], expected_artifacts: &["fine.out"] },
        StageDef { id: "interpolate", program: Some(ProgramKey::Epw), dependencies: &["fine_epw"], args: &[], expected_artifacts: &["interpolate.out"] },
        StageDef { id: "superconductivity", program: None, dependencies: &["interpolate"], args: &[], expected_artifacts: &["tc.json"] },
    ]
}

pub fn stage_def(id: &str) -> Option<StageDef> {
    pipeline().into_iter().find(|d| d.id == id)
}

fn program_path(key: ProgramKey, binding: &snapshot::RunBinding) -> std::path::PathBuf {
    match key {
        ProgramKey::Pw => binding.programs["pw"].path.clone(),
        ProgramKey::Ph => binding.programs["ph"].path.clone(),
        ProgramKey::Epw => binding.programs["epw"].path.clone(),
    }
}

/// 执行单个阶段：核对依赖、推进状态机、运行或提交、验证产物；F1 未接入的科学阶段返回 not_implemented。
pub fn execute(
    backend: Backend,
    binding: &snapshot::RunBinding,
    stage_id: &str,
    launcher: &[String],
) -> Result<state::StageAttempt> {
    let def = stage_def(stage_id).context("data_invalid: 未知阶段")?;
    let deps: Vec<String> = def.dependencies.iter().map(|s| s.to_string()).collect();

    let mut states = BTreeMap::new();
    for d in pipeline() {
        if let Ok(a) = state::StageAttempt::load(&binding.work_dir, d.id) {
            states.insert(d.id.to_string(), a.state);
        }
    }
    state::dependencies_satisfied(stage_id, &deps, &states)?;

    let mut attempt = state::StageAttempt::load(&binding.work_dir, stage_id)
        .unwrap_or_else(|_| state::StageAttempt::new(stage_id, deps, &binding.config_sha256));

    attempt.advance(state::StageState::SubmissionPending)?;
    attempt.advance(state::StageState::Submitted)?;
    attempt.advance(state::StageState::Running)?;

    let prog = match def.program {
        Some(k) => program_path(k, binding),
        None => bail!("not_implemented: 阶段 {stage_id} 由 F2–F4 定义，F1 尚未接入"),
    };
    let mut cmd: Vec<String> = vec![prog.to_string_lossy().into_owned()];
    cmd.extend_from_slice(&def.args.iter().map(|s| s.to_string()).collect::<Vec<_>>());

    match backend {
        Backend::Local => {
            let _report = process::run(&cmd, &binding.work_dir)?;
        }
        Backend::Slurm => {
            let job = slurm::submit_via_sbatch(launcher, &cmd, &binding.work_dir, stage_id)?;
            attempt.job_id = Some(job.job_id);
            attempt.cluster = job.cluster;
        }
    }

    attempt.advance(state::StageState::Validating)?;
    let complete = def
        .expected_artifacts
        .iter()
        .all(|a| binding.work_dir.join(a).exists());
    if complete {
        attempt.advance(state::StageState::Succeeded)?;
    } else {
        attempt.error = Some("data_invalid: 阶段产物缺失".to_string());
        attempt.advance(state::StageState::Failed)?;
    }
    attempt.save(&binding.work_dir)?;
    Ok(attempt)
}

/// 工作目录内保存树：<outdir>/<run_id>/<stage_id>；同一绑定内路径不变。
pub fn stage_tree(outdir_base: &std::path::Path, run_id: &str, stage_id: &str) -> std::path::PathBuf {
    outdir_base.join(run_id).join(stage_id)
}
