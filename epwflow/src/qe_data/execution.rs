//! 在已选定执行上下文内同步运行 QE，归档实际日志后才检查产物。
use super::super::foundation::{config::Program, process, stage::Backend};
use anyhow::{ensure, Result};
use std::{fs, path::Path};

pub fn run(
    program: &Program,
    input: &Path,
    dir: &Path,
    launcher: &[String],
    backend: Backend,
    stem: &str,
    scf: bool,
) -> Result<()> {
    ensure!(
        matches!(backend, Backend::Local),
        "allocation: QE 阶段须在 F1 已登记分配内同步执行，不能异步提交后读取产物"
    );
    let mut cmd = launcher.to_vec();
    cmd.push(program.path.to_string_lossy().into_owned());
    cmd.extend(program.args.clone());
    cmd.extend(["-inp".into(), input.to_string_lossy().into_owned()]);
    let report = process::run_report(&cmd, dir)?;
    fs::copy(report.stdout_path, dir.join(format!("{stem}.out")))?;
    fs::copy(report.stderr_path, dir.join(format!("{stem}.err")))?;
    fs::write(dir.join(format!("{stem}.exitcode")),
        report.exit_status.map(|n| n.to_string()).unwrap_or_else(|| "signal".into()))?;
    ensure!(
        report.exit_status == Some(0),
        "execution_failed: {stem} 退出码 {:?}",
        report.exit_status
    );
    qf_io::qe_completion::checked_log(&dir.join(format!("{stem}.out")), scf)
}
