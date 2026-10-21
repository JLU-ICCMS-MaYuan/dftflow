use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::{Path, PathBuf};
use super::{budget, config, environment, snapshot, status, workflow::{self, Boundary}};
use crate::interpolation::cli::InterpolationCommand;
use crate::qe_data::cli::QeDataCommand;
use crate::superconductivity::cli::SuperconductivityCommand;
use crate::delivery::cli::DeliveryCommand;

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
    /// 一次提交执行链；提交成功不代表数值完成。
    Submit {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long, value_enum, default_value = "all")]
        until: Boundary,
    },
    /// 在已登记的 Slurm 分配内依次执行科学阶段。
    Run {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long, value_enum, default_value = "all")]
        until: Boundary,
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
        #[arg(long, value_enum)]
        until: Option<Boundary>,
    },
    #[command(name = "__execute", hide = true)]
    Execute {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        attempt: String,
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
    /// 结果交付阶段边界：collect / results / budget（FR-001）。
    Delivery {
        #[command(subcommand)]
        command: DeliveryCommand,
    },
    /// 收集经过核验的插值叠图数据；不要求已有超导结果。
    Collect {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// 从自包含结果包绘图，不启动 QE、EPW 或 Slurm。
    Plot {
        #[arg(long)]
        results: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', default_value = "png,svg,pdf")]
        formats: Vec<String>,
    },
    /// 只读检查结果包。
    Results {
        #[command(subcommand)]
        command: ResultsCommand,
    },
}

#[derive(Subcommand)]
pub enum ResultsCommand {
    Check {
        #[arg(long)]
        results: PathBuf,
        #[arg(long)]
        json: bool,
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
    dispatch_command(cli.command)
}

pub(crate) fn dispatch_command(command: EpwCommand) -> Result<()> {
    match command {
        EpwCommand::Check { config, json } => check(&config, json),
        EpwCommand::Prepare { config } => prepare(&config),
        EpwCommand::Submit { config, until } => workflow::start(&config, until, true),
        EpwCommand::Run { config, until } => workflow::start(&config, until, false),
        EpwCommand::Status { work, refresh, json } => status_cmd(&work, refresh, json),
        EpwCommand::Resume {
            work,
            submit,
            require_allocation,
            until,
        } => {
            workflow::resume(&work, until, submit, require_allocation)?;
            Ok(())
        }
        EpwCommand::Execute { work, attempt } => workflow::execute(&work, &attempt),
        EpwCommand::Probe {
            config,
            submit,
            require_allocation,
        } => environment::probe(&config, submit, require_allocation),
        EpwCommand::Budget { action } => budget_cmd(action),
        EpwCommand::QeData { command } => crate::qe_data::cli::run(command),
        EpwCommand::Interpolation { command } => crate::interpolation::cli::run(command),
        EpwCommand::Superconductivity { command } => crate::superconductivity::cli::run(command),
        EpwCommand::Delivery { command } => crate::delivery::cli::run(command),
        EpwCommand::Collect { work, output } => {
            let result=crate::delivery::collect_results(&work,output.as_deref())?;
            if result.join("comparison/manifest.json").is_file() {
                if let Some(stats) = crate::delivery::comparison::load(&result.join("comparison"))?.differences {
                    stats.print();
                }
            }
            println!("results: {}",result.display());
            Ok(())
        },
        EpwCommand::Plot { results, output, formats } => {
            let path=crate::delivery::plot_results(&results,output.as_deref(),&formats)?;
            println!("figures: {}",path.display());
            Ok(())
        },
        EpwCommand::Results { command: ResultsCommand::Check { results, json } } => {
            let report=crate::delivery::check_results(&results)?;
            if json {status::print_json(&report);} else {println!("comparison_complete: {}\nworkflow_complete: {}\nfigures_verified: {}",report["comparison_complete"],report["workflow_complete"],report["figures_verified"]);}
            if let Some(stats) = report.get("differences").filter(|v| !v.is_null()) {
                if !json { serde_json::from_value::<crate::interpolation::differences::Differences>(stats.clone())?.print(); }
            }
            Ok(())
        },
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

fn status_cmd(work: &Path, refresh: bool, json: bool) -> Result<()> {
    let v = workflow::status(work, refresh)?;
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
