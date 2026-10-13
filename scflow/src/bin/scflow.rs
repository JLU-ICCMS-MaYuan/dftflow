//! scflow 单一静态二进制入口（对齐 Python `scflow` CLI）。
//!
//! 读取 `input.toml`（含 `[scflow]` 段与可选 `[mattersim]`/`[slurm]`）与 QE EPC 输出，
//! 计算 λ/wlog/w2 与 McMillan / Allen-Dynes / Eliashberg Tc，并写出 a2F.dat / lambda.dat /
//! result.json / 对称化 CIF。

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "scflow", version, about = "Rust rewrite of scflow (single static binary)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 计算超导转变温度（scflow -c input.toml）
    Run {
        #[arg(short, long)]
        config: Option<String>,
    },
}

fn main() -> Result<()> {
    // argv[0] 多调用分派：旧命令名 `scflow` 直达 run 子命令。
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(argv0) = args.first() {
        let entry = std::path::Path::new(argv0)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if entry == "scflow" {
            let is_subcommand = args
                .get(1)
                .map(|a| a.to_string_lossy() == "run")
                .unwrap_or(false);
            if !is_subcommand {
                args[0] = std::env::current_exe()
                    .map(|p| p.into_os_string())
                    .unwrap_or_else(|_| argv0.clone());
                args.insert(1, std::ffi::OsString::from("run"));
            }
        }
    }
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Run { config } => {
            let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let base = config
                .as_deref()
                .and_then(|p| Path::new(p).parent())
                .unwrap_or_else(|| Path::new("."));
            let sc = qf_scflow::config::from_config(base, &cfg)?;
            qf_scflow::run(&sc)?;
            Ok(())
        }
    }
}
