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
    crate::foundation::workspace::native_script(program,input,dir,launcher,stem,true,scf)?;
    ensure!(
        matches!(backend, Backend::Local),
        "allocation: QE 阶段须在 F1 已登记分配内同步执行，不能异步提交后读取产物"
    );
    qeflow::native::run_input(&program.path,&program.args,launcher,input,dir,stem,scf)
}
