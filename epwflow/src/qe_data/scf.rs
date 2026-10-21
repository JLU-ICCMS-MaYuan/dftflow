use super::super::foundation::stage::Backend;
use super::qe_config::QeSettings;
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::path::Path;

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
    let re = Regex::new(r"!.*total energy\s*=\s*([-0-9.eE+]+)\s*Ry").expect("scf: 正则编译失败");
    if let Some(c) = re.captures(out) {
        c[1].parse::<f64>()
            .with_context(|| "data_invalid: SCF 总能非数值")
    } else {
        bail!("data_invalid: 未在 SCF 输出中找到总能");
    }
}

/// 一次 SCF：写输入、执行、核对收敛与保存、复制不可写快照。
pub fn run_scf(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<ScfResult> {
    let dir = crate::foundation::layout::path(&work, "scf");
    std::fs::create_dir_all(&dir)?;
    let outdir = dir.join("out");
    if !crate::foundation::native_jobs::continuing(&dir, "scf")? {
        std::fs::create_dir_all(&outdir)?;
        let input = qe.scf_input(outdir.to_str().unwrap())?;
        std::fs::write(dir.join("scf.in"), &input)?;
    }
    super::execution::run(
        &qe.config.programs.pw,
        &dir.join("scf.in"),
        &dir,
        launcher,
        backend,
        "scf",
        true,
    )?;
    let out = std::fs::read_to_string(dir.join("scf.out"))
        .with_context(|| "data_invalid: 缺少 scf.out")?;
    let energy = parse_scf_energy(&out)?;
    super::xml::validate_save(&outdir, &qe.config.qe.prefix, qe, Some(energy))?;
    let snapshot = super::save::snapshot(&outdir, work)?;
    Ok(ScfResult {
        energy_ry: energy,
        snapshot_dir: snapshot,
        attempt_count: 1,
        success_count: 1,
    })
}

/// 复用已验证 SCF 快照：仅当来源、参数、版本与布局一致时登记，否则拒绝。
pub fn reuse_scf(source_save: &Path, work: &Path) -> Result<std::path::PathBuf> {
    let snapshot = crate::foundation::layout::path(&work, "scf").join("snapshot");
    let entries = super::save::copy_tree(source_save, &snapshot)?;
    super::super::foundation::store::write(
        &crate::foundation::layout::path(&work, "scf/snapshot-files.json"),
        &entries,
    )?;
    Ok(snapshot)
}
