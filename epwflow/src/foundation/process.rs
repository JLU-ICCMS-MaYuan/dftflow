use anyhow::{ensure, Context, Result};
use std::{fs, path::Path, process::Command, time::Instant};

/// 外部程序执行报告；路径及启动参数不经隐式 shell 求值，仅按参数列表执行。
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub exit_status: Option<i32>,
    pub stdout_path: std::path::PathBuf,
    pub stderr_path: std::path::PathBuf,
    pub duration_secs: f64,
}

/// 在明确工作目录执行命令，stdout/stderr 落盘到日志；失败按退出码或启动错误判定。
pub fn run(command: &[String], work: &Path) -> Result<ExecutionReport> {
    let report = run_report(command, work)?;
    ensure!(
        report.exit_status == Some(0),
        "execution_failed: 命令 {} 退出码 {:?}",
        command[0],
        report.exit_status
    );
    Ok(report)
}

/// 保留失败进程的退出状态和日志，供科学尝试持久化；不把非零退出当成功。
pub fn run_report(command: &[String], work: &Path) -> Result<ExecutionReport> {
    ensure!(!command.is_empty(), "execution_failed: 空命令");
    let dir = work.join(".epwflow").join("logs");
    fs::create_dir_all(&dir)?;
    let stdout_path = dir.join(format!("out-{}.log", store_token()));
    let stderr_path = dir.join(format!("err-{}.log", store_token()));
    let start = Instant::now();
    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(work)
        .stdout(fs::File::create(&stdout_path)?)
        .stderr(fs::File::create(&stderr_path)?)
        .status()
        .with_context(|| format!("execution_failed: 无法启动 {}", command[0]))?;
    let duration = start.elapsed().as_secs_f64();
    let exit_status = status.code();
    Ok(ExecutionReport {
        exit_status,
        stdout_path,
        stderr_path,
        duration_secs: duration,
    })
}

fn store_token() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}
