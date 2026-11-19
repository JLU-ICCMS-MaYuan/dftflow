use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::{Path, PathBuf};
use super::{config, environment, snapshot, status, workflow::{self, Boundary}};
use crate::interpolation::cli::InterpolationCommand;
use crate::qe_data::cli::QeDataCommand;
use crate::superconductivity::cli::SuperconductivityCommand;
use crate::delivery::cli::DeliveryCommand;

#[derive(Parser)]
#[command(name = "epwflow", about = "Controlled QE/EPW superconductivity workflow")]
pub struct Cli {
    #[command(subcommand)]
    command: EpwCommand,
}

#[derive(Subcommand)]
pub enum EpwCommand {
    /// 运行前检查配置与文件；不启动科学程序或提交作业。
    #[command(about = "Check configuration and files without running scientific programs or submitting jobs", long_about = "Check configuration and files without running scientific programs or submitting jobs")]
    Check {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 固定输入并准备独立运行。
    #[command(about = "Freeze input and prepare an independent run", long_about = "Freeze input and prepare an independent run")]
    Prepare {
        #[arg(short = 'c', long)]
        config: PathBuf,
    },
    /// 一次提交执行链；提交成功不代表数值完成。
    #[command(about = "Submit the execution chain; submission success does not establish numerical completion", long_about = "Submit the execution chain; submission success does not establish numerical completion")]
    Submit {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long, value_enum, default_value = "all")]
        until: Boundary,
    },
    /// 在现有 Slurm 分配内依次执行科学阶段。
    #[command(about = "Run scientific stages sequentially within an existing Slurm allocation", long_about = "Run scientific stages sequentially within an existing Slurm allocation")]
    Run {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long, value_enum, default_value = "all")]
        until: Boundary,
    },
    /// 查询运行状态；只读可以成功返回失败/未知，但要显式显示。
    #[command(about = "Query run status read-only, explicitly reporting failed or unknown states", long_about = "Query run status read-only, explicitly reporting failed or unknown states")]
    Status {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        json: bool,
    },
    /// 使用固定快照核对并续算；-s 与 -r 互斥且必选其一。
    #[command(about = "Validate and resume with the frozen snapshot; choose exactly one of -s or -r", long_about = "Validate and resume with the frozen snapshot; choose exactly one of -s or -r")]
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
    #[command(about = "Probe the environment; choose exactly one of -s or -r", long_about = "Probe the environment; choose exactly one of -s or -r")]
    Probe {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(short = 's', long)]
        submit: bool,
        #[arg(short = 'r', long)]
        require_allocation: bool,
    },
    /// QE 数据准备阶段边界：check / import（FR-001）。
    #[command(about = "QE data preparation: check / import", long_about = "QE data preparation: check / import")]
    QeData {
        #[command(subcommand)]
        command: QeDataCommand,
    },
    /// Wannier 化与 EPW 插值阶段边界：check / import（FR-001）。
    #[command(about = "Wannier fitting and EPW interpolation: check / import", long_about = "Wannier fitting and EPW interpolation: check / import")]
    Interpolation {
        #[command(subcommand)]
        command: InterpolationCommand,
    },
    /// 超导求解阶段边界：check / import（FR-001）。
    #[command(about = "Superconductivity solving: check / import", long_about = "Superconductivity solving: check / import")]
    Superconductivity {
        #[command(subcommand)]
        command: SuperconductivityCommand,
    },
    /// 结果交付阶段边界：collect / results（FR-001）。
    #[command(about = "Result delivery: collect / results", long_about = "Result delivery: collect / results")]
    Delivery {
        #[command(subcommand)]
        command: DeliveryCommand,
    },
    /// 收集经过核验的插值叠图数据；不要求已有超导结果。
    #[command(about = "Collect validated interpolation comparison data; superconductivity results are not required", long_about = "Collect validated interpolation comparison data; superconductivity results are not required")]
    Collect {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// 从自包含结果包绘图，不启动 QE、EPW 或 Slurm。
    #[command(about = "Plot from a self-contained result package without starting QE, EPW or Slurm", long_about = "Plot from a self-contained result package without starting QE, EPW or Slurm")]
    Plot {
        #[arg(long)]
        results: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', default_value = "png,svg,pdf")]
        formats: Vec<String>,
    },
    /// 只读检查结果包。
    #[command(about = "Validate the result package read-only", long_about = "Validate the result package read-only")]
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
        println!("configuration: passed (schema_version={})", cfg.schema_version);
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
