//! 在已选定执行上下文内同步运行 QE，归档实际日志后才检查产物。
use super::super::foundation::{config::Program, stage::Backend};
use anyhow::{ensure, Result};
use std::path::Path;

pub fn run(
    program: &Program,
    input: &Path,
    dir: &Path,
    launcher: &[String],
    backend: Backend,
    stem: &str,
    scf: bool,
) -> Result<()> {
    if crate::foundation::native_jobs::enabled() {
        let mut argv = launcher.to_vec();
        argv.push(program.path.to_string_lossy().into_owned());
        argv.extend(program.args.clone());
        argv.extend(["-inp".into(), input.to_string_lossy().into_owned()]);
        let report = crate::foundation::process::run_report_named(&argv, dir, stem)?;
        ensure!(
            report.exit_status == Some(0),
            "execution_failed: QE 原生退出 {:?}",
            report.exit_status
        );
        return qf_io::qe_completion::checked_log(&dir.join(format!("{stem}.out")), scf)
            .map(|_| ());
    }
    crate::foundation::workspace::native_script(program, input, dir, launcher, stem, true, scf)?;
    ensure!(
        matches!(backend, Backend::Local),
        "allocation: QE 阶段须在 F1 已登记分配内同步执行，不能异步提交后读取产物"
    );
    qeflow::native::run_input(
        &program.path,
        &program.args,
        launcher,
        input,
        dir,
        stem,
        scf,
    )
}
