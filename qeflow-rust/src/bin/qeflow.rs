//! qeflow 单一静态二进制入口。
//! 子命令映射现有 qe_* CLI；M0 仅打通 config 加载 + QE 命令解析，
//! 其余子命令为占位，由 M3/M4 逐步落地真实编排逻辑。

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::Path;

use qf_state::{refresh_workflow_status, split_resume_step};

#[derive(Parser)]
#[command(name = "qeflow", version, about = "Rust rewrite of qeflow (single static binary)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SCF calculation
    QeScf {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
    },
    /// NSCF calculation
    QeNscf {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Electronic band structure
    QeEband {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Electronic DOS
    QeElEdos {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Phonon workflow (nosplit)
    QePhononNosplit {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// Phonon split by index
    QePhononSplitIndex {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// Phonon split by coords
    QePhononSplitCoords {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// Process phonon (collect + plot)
    QeProcessPhonon {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// Superconducting Tc (scflow)
    Scflow {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// 刷新并打印工作流状态与续算点（M3 状态机演示）
    Status {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short, long, default_value = ".")]
        work_dir: String,
        #[arg(long, default_value = "struct")]
        prefix: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::QeScf { config, run, poscar } => {
            let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let pos_path = poscar.unwrap_or_else(|| "POSCAR".to_string());
            let structure = qf_crystal::parse_poscar(&pos_path)
                .map_err(|e| anyhow::anyhow!("解析 POSCAR 失败: {e}"))?;
            let prefix = cfg
                .qe_phonon
                .scf
                .get("prefix")
                .and_then(|v| v.as_str())
                .unwrap_or("struct")
                .to_string();
            let written = qf_input::pw::write_phonon_scf_input(".", &cfg, &structure, &prefix)?;
            println!("[qe_scf] wrote {written} for prefix '{prefix}'");
            if run {
                let cmd = qf_runner::resolve_qe_command(cfg.qe.executable_path(), "pw.x");
                println!("[qe_scf] would run: {cmd} < {written} > scf.out");
            }
            Ok(())
        }
        Commands::QeNscf { config } => {
            let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let structure = qf_crystal::parse_poscar("POSCAR")
                .map_err(|e| anyhow::anyhow!("解析 POSCAR 失败: {e}"))?;
            let prefix = cfg
                .qe_phonon
                .scf
                .get("prefix")
                .and_then(|v| v.as_str())
                .unwrap_or("struct")
                .to_string();
            let written = qf_input::pw::write_phonon_nscf_input(".", &cfg, &structure, &prefix)?;
            println!("[qe_nscf] wrote {written} for prefix '{prefix}'");
            Ok(())
        }
        Commands::QeEband { config } => demo_config(config, "qe_eband"),
        Commands::QeElEdos { config } => demo_config(config, "qe_eledos"),
        Commands::QePhononNosplit { config, run, submit } => {
            demo_phonon(config, "qe_phonon_nosplit", run, submit)
        }
        Commands::QePhononSplitIndex { config, run, submit } => {
            demo_phonon(config, "qe_phonon_split_index", run, submit)
        }
        Commands::QePhononSplitCoords { config, run, submit } => {
            demo_phonon(config, "qe_phonon_split_coords", run, submit)
        }
        Commands::QeProcessPhonon { config, run, submit } => {
            demo_phonon(config, "qe_process_phonon", run, submit)
        }
        Commands::Scflow { config } => {
            let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let base = config
                .as_deref()
                .and_then(|p| Path::new(p).parent())
                .unwrap_or_else(|| Path::new("."));
            let sc = qf_scflow::config::from_config(base, &cfg)?;
            qf_scflow::run(&sc)?;
            Ok(())
        }
        Commands::Status { config, work_dir, prefix } => {
            let _cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let status = refresh_workflow_status(Path::new(&work_dir), &prefix)?;
            let (step, label) = split_resume_step(&status);
            println!("[status] workflow_status.json 已刷新于 {work_dir}");
            println!(
                "  scf={:?} dyn0={:?} merge={:?} q2r={:?}",
                status.scf, status.ph.dyn0, status.merge, status.q2r
            );
            println!(
                "  matdyn_phband={:?} matdyn_phdos={:?} phonon_plot={:?}",
                status.matdyn_phband, status.matdyn_phdos, status.phonon_plot
            );
            for (q, s) in &status.ph.tasks {
                println!("  split {q}: {s:?}");
            }
            if step == 0 {
                println!("[status] 全部完成，无需续算");
            } else {
                println!("[status] 续算点: {step}/11 {label}");
            }
            Ok(())
        }
    }
}

fn demo_config(config: Option<String>, name: &str) -> Result<()> {
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let cmd = qf_runner::resolve_qe_command(cfg.qe.executable_path(), "pw.x");
    println!("[{name}] resolved command: {cmd} (not yet implemented)");
    Ok(())
}

fn demo_phonon(config: Option<String>, name: &str, run: bool, submit: bool) -> Result<()> {
    let _cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    println!("[{name}] run={run} submit={submit} (not yet implemented, M3/M4)");
    Ok(())
}
