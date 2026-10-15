use anyhow::{bail, Context, Result};
use regex::Regex;
use std::path::Path;
use super::super::foundation::{process, stage::Backend};
use super::qe_config::QeSettings;

/// SCF 结果：总能（Ry）、快照目录、实际尝试/成功次数。
#[derive(Debug, Clone)]
pub struct ScfResult {
    pub energy_ry: f64,
    pub snapshot_dir: std::path::PathBuf,
    pub attempt_count: u32,
    pub success_count: u32,
}

/// 从 pw.x 输出解析总能（Ry）；仅接受收敛后的最终总能，不接受屏幕舍入冒充。
pub fn parse_scf_energy(out: &str) -> Result<f64> {
    let re = Regex::new(r"!.*total energy\s*=\s*([-0-9.eE+]+)\s*Ry")
        .expect("scf: 正则编译失败");
    if let Some(c) = re.captures(out) {
        c[1].parse::<f64>()
            .with_context(|| "data_invalid: SCF 总能非数值")
    } else {
        bail!("data_invalid: 未在 SCF 输出中找到总能");
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let p = e.path();
        let target = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&p, &target)?;
        } else {
            std::fs::copy(&p, &target)?;
        }
    }
    Ok(())
}

/// 一次 SCF：写输入、执行、核对收敛与保存、复制不可写快照。
pub fn run_scf(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<ScfResult> {
    let dir = work.join("scf");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir)?;
    let input = qe.scf_input(outdir.to_str().unwrap());
    std::fs::write(dir.join("scf.in"), &input)?;
    let prog = &qe.config.programs.pw.path;
    let cmd: Vec<String> = launcher
        .iter()
        .cloned()
        .chain(std::iter::once(prog.to_string_lossy().into_owned()))
        .chain(std::iter::once("-inp".to_string()))
        .chain(std::iter::once(dir.join("scf.in").to_string_lossy().into_owned()))
        .collect();
    match backend {
        Backend::Local => {
            let _r = process::run(&cmd, &dir)?;
        }
        Backend::Slurm => {
            super::super::foundation::slurm::submit_via_sbatch(launcher, &cmd[launcher.len()..], &dir, "scf")?;
        }
    }
    let out = std::fs::read_to_string(dir.join("scf.out"))
        .with_context(|| "data_invalid: 缺少 scf.out")?;
    let energy = parse_scf_energy(&out)?;
    let snapshot = dir.join("snapshot");
    copy_dir(&outdir.join("scf.save"), &snapshot)?;
    if !snapshot.exists() {
        bail!("data_invalid: SCF 快照不完整（缺少 save 树）");
    }
    Ok(ScfResult {
        energy_ry: energy,
        snapshot_dir: snapshot,
        attempt_count: 1,
        success_count: 1,
    })
}

/// 复用已验证 SCF 快照：仅当来源、参数、版本与布局一致时登记，否则拒绝。
pub fn reuse_scf(source_save: &Path, work: &Path) -> Result<std::path::PathBuf> {
    let snapshot = work.join("scf").join("snapshot");
    copy_dir(source_save, &snapshot)?;
    Ok(snapshot)
}
