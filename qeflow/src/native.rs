//! 供 qeflow 与 EPW 共用的同步原生输入执行边界。
use anyhow::{ensure, Context, Result};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub fn run_input(
    program: &Path,
    args: &[String],
    launcher: &[String],
    input: &Path,
    dir: &Path,
    stem: &str,
    scf: bool,
) -> Result<()> {
    ensure!(
        !stem.is_empty()
            && stem
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
        "非法 QE 日志名称"
    );
    let mut argv = launcher.to_vec();
    argv.push(program.to_string_lossy().into_owned());
    argv.extend_from_slice(args);
    argv.extend(["-inp".into(), input.to_string_lossy().into_owned()]);
    let stdout = fs::File::create(dir.join(format!("{stem}.out")))?;
    let stderr = fs::File::create(dir.join(format!("{stem}.err")))?;
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .context("无法启动 QE 原生程序")?;
    fs::write(
        dir.join(format!("{stem}.exitcode")),
        status
            .code()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "signal".into()),
    )?;
    ensure!(status.success(), "QE {stem} 退出失败：{status}");
    qf_io::qe_completion::checked_log(&dir.join(format!("{stem}.out")), scf)
}
