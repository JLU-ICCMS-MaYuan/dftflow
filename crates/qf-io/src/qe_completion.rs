//! 从 qeflow 的 0006 初始化链提取的共用完成检查。
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};

pub fn checked_log(path: &Path, scf: bool) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("Missing Output {}", path.display()))?;
    let lower = text.to_lowercase();
    ensure!(
        text.contains("JOB DONE")
            && !["error in routine", "convergence not achieved", "%%%%%%%%"]
                .iter()
                .any(|s| lower.contains(s)),
        "Calculating Unfinished or Unfinished: {}",
        path.display()
    );
    if scf {
        ensure!(
            lower.contains("convergence has been achieved"),
            "SCF Unrecovered: {}",
            path.display()
        );
    }
    Ok(())
}
