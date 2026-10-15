use anyhow::{bail, Result};
use std::path::Path;
use super::super::foundation::stage::Backend;
use super::manifest::{PhResponse, QPoint};
use super::qe_config::QeSettings;
use super::qmap;

/// 构建单 q 的响应产物引用（FR-005/007）：dyn、patterns、dvscf 与元数据统一配对。
fn make_response(outdir: &Path, q: &QPoint) -> PhResponse {
    PhResponse {
        q_id: q.global_index,
        dyn_file: outdir.join(format!("ph.dyn{}", q.global_index)),
        patterns: outdir.join(format!("ph.phsave/patterns.{}", q.global_index)),
        dvscf: Some(outdir.join(format!("ph.dvscf_q.{}", q.global_index))),
        control_metadata: outdir.join("ph.phsave/_ph0/control_ph.xml"),
        format: "qe-ph".to_string(),
        perturbations: 0,
    }
}

/// PH 初始化：写 only_init 输入、执行、解析 dyn0 得到不可约 q 列表（FR-005/006）。
pub fn run_init(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<Vec<QPoint>> {
    let dir = work.join("ph_init");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    let input = qe.phonon_init_input(outdir.to_str().unwrap());
    std::fs::write(dir.join("ph_init.in"), &input)?;
    let prog = &qe.config.programs.ph.path;
    let cmd: Vec<String> = launcher
        .iter()
        .cloned()
        .chain(std::iter::once(prog.to_string_lossy().into_owned()))
        .chain(std::iter::once("-inp".to_string()))
        .chain(std::iter::once(dir.join("ph_init.in").to_string_lossy().into_owned()))
        .collect();
    match backend {
        Backend::Local => {
            let _r = super::super::foundation::process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], &dir, "ph_init")?;
        }
    }
    let dyn0 = outdir.join("ph.dyn0");
    if !dyn0.exists() {
        bail!("data_invalid: PH 初始化未产生 dyn0，禁止后续分 q 提交");
    }
    qmap::parse_dyn0(&dyn0)
}

/// PH 分 q：写该 q 输入、执行、归集 dyn/patterns/dvscf（FR-005/007，index/coords）。
pub fn run_q(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
    q: &QPoint,
    q_index: usize,
) -> Result<PhResponse> {
    let dir = work.join("ph").join(&q.directory);
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    let input = qe.phonon_q_input(outdir.to_str().unwrap(), &qe.config.phonon.mode, Some(q), q_index)?;
    std::fs::write(dir.join("ph.in"), &input)?;
    let prog = &qe.config.programs.ph.path;
    let cmd: Vec<String> = launcher
        .iter()
        .cloned()
        .chain(std::iter::once(prog.to_string_lossy().into_owned()))
        .chain(std::iter::once("-inp".to_string()))
        .chain(std::iter::once(dir.join("ph.in").to_string_lossy().into_owned()))
        .collect();
    match backend {
        Backend::Local => {
            let _r = super::super::foundation::process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], &dir, &format!("ph_{}", q.global_index))?;
        }
    }
    let resp = make_response(&outdir, q);
    if !resp.dyn_file.exists() {
        bail!("data_invalid: q={} 缺少动力学矩阵 dyn{}", q.global_index, q.global_index);
    }
    Ok(resp)
}

/// nosplit：单个 PH 任务计算完整 q 网格，校验 dyn0 与全部对应数据（FR-005）。
pub fn run_nosplit(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
    qpoints: &[QPoint],
) -> Result<Vec<PhResponse>> {
    let dir = work.join("ph_nosplit");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    let input = qe.phonon_nosplit_input(outdir.to_str().unwrap());
    std::fs::write(dir.join("ph.in"), &input)?;
    let prog = &qe.config.programs.ph.path;
    let cmd: Vec<String> = launcher
        .iter()
        .cloned()
        .chain(std::iter::once(prog.to_string_lossy().into_owned()))
        .chain(std::iter::once("-inp".to_string()))
        .chain(std::iter::once(dir.join("ph.in").to_string_lossy().into_owned()))
        .collect();
    match backend {
        Backend::Local => {
            let _r = super::super::foundation::process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], &dir, "ph_nosplit")?;
        }
    }
    let mut responses = Vec::new();
    for q in qpoints {
        let resp = make_response(&outdir, q);
        if !resp.dyn_file.exists() {
            bail!(
                "data_invalid: nosplit 缺 q={} 的动力学矩阵 dyn{}",
                q.global_index,
                q.global_index
            );
        }
        responses.push(resp);
    }
    Ok(responses)
}
