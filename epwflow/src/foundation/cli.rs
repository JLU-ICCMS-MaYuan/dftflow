use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::{Path, PathBuf};
use super::{budget, config, environment, snapshot, status};
use crate::interpolation::cli::InterpolationCommand;
use crate::qe_data::cli::QeDataCommand;
use crate::superconductivity::cli::SuperconductivityCommand;

#[derive(Parser)]
#[command(name = "epwflow", about = "受控的 QE/EPW 超导计算工作流")]
pub struct Cli {
    #[command(subcommand)]
    command: EpwCommand,
}

#[derive(Subcommand)]
pub enum EpwCommand {
    /// 运行前检查配置与文件；不启动科学程序或提交作业。
    Check {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 固定输入并准备独立运行。
    Prepare {
        #[arg(short = 'c', long)]
        config: PathBuf,
    },
    /// 提交全流程；科学阶段未接入前返回明确不可用。
    Submit {
        #[arg(short = 'c', long)]
        config: PathBuf,
    },
    /// 直接运行；科学阶段未接入前返回明确不可用。
    Run {
        #[arg(short = 'c', long)]
        config: PathBuf,
    },
    /// 查询运行状态；只读可以成功返回失败/未知，但要显式显示。
    Status {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        json: bool,
    },
    /// 使用固定快照核对并续算；-s 与 -r 互斥且必选其一。
    Resume {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(short = 's', long)]
        submit: bool,
        #[arg(short = 'r', long)]
        require_allocation: bool,
    },
    /// 环境探测；-s 与 -r 互斥且必选其一。
    Probe {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(short = 's', long)]
        submit: bool,
        #[arg(short = 'r', long)]
        require_allocation: bool,
    },
    /// 共享预算账本：status/reserve/attach/reconcile。
    Budget {
        #[command(subcommand)]
        action: Option<BudgetAction>,
    },
    /// QE 数据准备阶段边界：check / import（FR-001）。
    QeData {
        #[command(subcommand)]
        command: QeDataCommand,
    },
    /// Wannier 化与 EPW 插值阶段边界：check / import（FR-001）。
    Interpolation {
        #[command(subcommand)]
        command: InterpolationCommand,
    },
    /// 超导求解阶段边界：check / import（FR-001）。
    Superconductivity {
        #[command(subcommand)]
        command: SuperconductivityCommand,
    },
}

#[derive(Subcommand)]
pub enum BudgetAction {
    Status {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Reserve {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        label: String,
        #[arg(long)]
        nodes: u32,
        #[arg(long)]
        mpi: u32,
        #[arg(long = "wall-seconds")]
        wall_seconds: u64,
        #[arg(long = "storage-bytes")]
        storage_bytes: u64,
    },
    Attach {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        token: String,
        #[arg(long)]
        job: Option<u64>,
    },
    Reconcile {
        #[arg(long)]
        ledger: PathBuf,
    },
}

pub fn run() {
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli) {
        status::fail(e);
    }
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        EpwCommand::Check { config, json } => check(&config, json),
        EpwCommand::Prepare { config } => prepare(&config),
        EpwCommand::Submit { config } => submit(&config),
        EpwCommand::Run { config } => run_cmd(&config),
        EpwCommand::Status { work, refresh: _, json } => status_cmd(&work, json),
        EpwCommand::Resume {
            work,
            submit,
            require_allocation,
        } => {
            snapshot::resume(&work, submit, require_allocation)?;
            Ok(())
        }
        EpwCommand::Probe {
            config,
            submit,
            require_allocation,
        } => environment::probe(&config, submit, require_allocation),
        EpwCommand::Budget { action } => budget_cmd(action),
        EpwCommand::QeData { command } => crate::qe_data::cli::run(command),
        EpwCommand::Interpolation { command } => crate::interpolation::cli::run(command),
        EpwCommand::Superconductivity { command } => crate::superconductivity::cli::run(command),
    }
}

fn check(config_path: &Path, json: bool) -> Result<()> {
    let cfg = config::Config::load(config_path)?;
    // 普通 check 不暗中探测或提交；运行版本未验证明确标为 unverified。
    if json {
        let v = status::envelope(
            "ok",
            "configured",
            json!({"environment": {"runtime_version": "unverified"}}),
        );
        status::print_json(&v);
    } else {
        println!("configuration: 通过（schema_version={}）", cfg.schema_version);
        println!("environment.runtime_version: unverified");
    }
    Ok(())
}

fn prepare(config_path: &Path) -> Result<()> {
    let cfg = config::Config::load(config_path)?;
    let binding = snapshot::prepare(&cfg, config_path)?;
    println!(
        "prepared: run_id={} work={} outdir_base={}",
        binding.run_id,
        binding.work_dir.display(),
        binding.outdir_base.display()
    );
    Ok(())
}

fn submit(config_path: &Path) -> Result<()> {
    let _ = config_path;
    // F1：科学阶段未接入前，生产 submit 必须返回明确不可用错误，不启动任何程序。
    anyhow::bail!("not_implemented: 科学阶段尚未接入，submit 不可用");
}

fn run_cmd(config_path: &Path) -> Result<()> {
    let _ = config_path;
    anyhow::bail!("not_implemented: 科学阶段尚未接入，run 不可用");
}

fn status_cmd(work: &Path, json: bool) -> Result<()> {
    let v = snapshot::status(work)?;
    if json {
        status::print_json(&v);
    } else {
        println!("workflow_status: {}", v["workflow_status"]);
        println!("workflow_complete: {}", v["workflow_complete"]);
    }
    Ok(())
}

fn budget_cmd(action: Option<BudgetAction>) -> Result<()> {
    match action {
        None => {
            println!("usage: epwflow budget <status|reserve|attach|reconcile> --help");
            Ok(())
        }
        Some(BudgetAction::Status { ledger, json }) => {
            let v = budget::status_json(&ledger, "epic-0007")?;
            if json {
                status::print_json(&v);
            } else {
                println!("campaign_id: {}", v["campaign_id"]);
                println!("spent_node_seconds: {}", v["spent_node_seconds"]);
                println!("reserved_node_seconds: {}", v["reserved_node_seconds"]);
                println!("lanes: {}", v["lanes"]);
            }
            Ok(())
        }
        Some(BudgetAction::Reserve {
            ledger,
            label,
            nodes,
            mpi,
            wall_seconds,
            storage_bytes,
        }) => {
            let token = budget::with_ledger(&ledger, "epic-0007", |ledger| {
                budget::reserve(ledger, &label, nodes, mpi, wall_seconds, storage_bytes)
            })?;
            println!("reserved: token={}", token);
            Ok(())
        }
        Some(BudgetAction::Attach {
            ledger,
            token,
            job,
        }) => {
            budget::with_ledger(&ledger, "epic-0007", |ledger| {
                budget::attach(ledger, &token, job)
            })?;
            println!("attached: token={}", token);
            Ok(())
        }
        Some(BudgetAction::Reconcile { ledger }) => {
            let settled =
                budget::with_ledger(&ledger, "epic-0007", |ledger| budget::reconcile(ledger))?;
            println!("reconciled: settled_node_seconds={}", settled);
            Ok(())
        }
    }
}
