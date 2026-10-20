//! 三个独立阶段入口，共用已有配置、账本、状态和科学执行器。
use super::{
    cli::{self, BudgetAction, EpwCommand, ResultsCommand},
    status,
    workflow::{self, Boundary},
};
use anyhow::{ensure, Context, Result};
use clap::{ArgGroup, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Phonon,
    Wannier,
    Superconductivity,
}
impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Phonon => "epw_phonon",
            Self::Wannier => "epw_wannier",
            Self::Superconductivity => "epw_superconductivity",
        }
    }
    pub fn boundary(self) -> Boundary {
        match self {
            Self::Phonon => Boundary::QeData,
            Self::Wannier => Boundary::Interpolation,
            Self::Superconductivity => Boundary::All,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum BudgetOperation {
    Status,
    Reserve,
    Attach,
    Reconcile,
}

#[derive(Parser)]
#[command(about = "生成、提交或恢复本阶段；默认读取 inputepw.toml 并生成脚本")]
#[command(group(ArgGroup::new("operation").args(["check", "prepare_only", "status", "probe", "import", "collect", "plot", "results_check", "export_comparison", "budget", "execute_attempt", "run_script", "all", "submit_chain"])))]
struct Options {
    #[arg(short = 'c', long, default_value = "inputepw.toml")]
    config: PathBuf,
    #[arg(short = 'w', long)]
    work: Option<PathBuf>,
    /// 提交本阶段的有界 Slurm 作业。
    #[arg(short = 's', long, conflicts_with = "run")]
    submit: bool,
    /// 在已经登记的 Slurm 分配内运行本阶段。
    #[arg(short = 'r', long)]
    run: bool,
    /// 从固定输入和已验证的完成记录恢复，须搭配 -s 或 -r。
    #[arg(long)]
    resume: bool,
    /// 检查配置；结合 -w 检查本阶段数据。
    #[arg(long)]
    check: bool,
    #[arg(long)]
    prepare_only: bool,
    #[arg(long)]
    status: bool,
    #[arg(long)]
    refresh: bool,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    probe: bool,
    #[arg(long, value_name = "MANIFEST")]
    import: Option<PathBuf>,
    #[arg(long)]
    collect: bool,
    #[arg(long)]
    plot: bool,
    #[arg(long)]
    results_check: bool,
    #[arg(long)]
    results: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, value_delimiter = ',', default_value = "png,svg,pdf")]
    formats: Vec<String>,
    #[arg(long, value_name = "SOURCES")]
    export_comparison: Option<PathBuf>,
    #[arg(long, value_enum)]
    budget: Option<BudgetOperation>,
    #[arg(long)]
    ledger: Option<PathBuf>,
    #[arg(long)]
    label: Option<String>,
    #[arg(long)]
    nodes: Option<u32>,
    /// 预算预留或失败 Wannier 阶段恢复时的 MPI 数；恢复同步设置单 image pool 数。
    #[arg(long)]
    mpi: Option<u32>,
    #[arg(long)]
    wall_seconds: Option<u64>,
    #[arg(long)]
    storage_bytes: Option<u64>,
    #[arg(long)]
    token: Option<String>,
    #[arg(long)]
    job: Option<u64>,
    #[arg(long, hide = true)]
    execute_attempt: Option<String>,
    #[arg(long, hide = true)]
    run_script: bool,
    /// 仅 epw_phonon：生成三个阶段脚本和总提交脚本，不提交。
    #[arg(long, conflicts_with_all = ["submit", "run", "resume"])]
    all: bool,
    #[arg(long, hide = true, conflicts_with_all = ["submit", "run", "resume"])]
    submit_chain: bool,
}

pub fn run(phase: Phase) {
    use clap::CommandFactory;
    let matches = Options::command().name(phase.name()).get_matches();
    let options =
        <Options as clap::FromArgMatches>::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    if let Err(e) = dispatch(phase, options) {
        status::fail(e);
    }
}

fn dispatch(phase: Phase, o: Options) -> Result<()> {
    let work = || o.work.clone().context("required: 此操作需要 -w/--work");
    let results = || o.results.clone().context("required: 此操作需要 --results");
    if o.all || o.submit_chain {
        ensure!(phase == Phase::Phonon, "input_conflict: 总脚本仅由 epw_phonon 管理");
        if o.submit_chain { return super::chain::submit(&work()?); }
        let path = super::chain::generate(&o.config)?;
        println!("script: {}\nworkflow_complete: false", path.display());
        return Ok(());
    }
    let maintenance = o.check
        || o.prepare_only
        || o.status
        || o.import.is_some()
        || o.collect
        || o.plot
        || o.results_check
        || o.export_comparison.is_some()
        || o.budget.is_some();
    ensure!(
        !maintenance || (!o.submit && !o.run && !o.resume),
        "input_conflict: 只读、导入或准备操作不能同时执行/提交/恢复"
    );
    ensure!(!o.refresh || o.status, "required: --refresh 需要 --status");
    ensure!(o.mpi.is_none() || o.budget.is_some() || (o.resume && phase == Phase::Wannier),
        "input_conflict: --mpi 仅用于预算操作或 epw_wannier --resume");
    ensure!(
        !o.resume || o.submit || o.run,
        "required: --resume 需要 -s 或 -r"
    );
    if let Some(attempt) = o.execute_attempt {
        return workflow::execute_phase(&work()?, &attempt, phase.boundary());
    }
    if o.run_script {
        return workflow::run_generated(&work()?, phase.boundary());
    }
    let command = if o.check {
        if let Some(work) = o.work {
            match phase {
                Phase::Phonon => EpwCommand::QeData {
                    command: crate::qe_data::cli::QeDataCommand::Check { work, json: o.json },
                },
                Phase::Wannier => EpwCommand::Interpolation {
                    command: crate::interpolation::cli::InterpolationCommand::Check {
                        work,
                        json: o.json,
                    },
                },
                Phase::Superconductivity => EpwCommand::Superconductivity {
                    command: crate::superconductivity::cli::SuperconductivityCommand::Check {
                        work,
                        json: o.json,
                    },
                },
            }
        } else {
            EpwCommand::Check {
                config: o.config,
                json: o.json,
            }
        }
    } else if o.prepare_only {
        EpwCommand::Prepare { config: o.config }
    } else if o.status {
        EpwCommand::Status {
            work: work()?,
            refresh: o.refresh,
            json: o.json,
        }
    } else if o.probe {
        EpwCommand::Probe {
            config: o.config,
            submit: o.submit,
            require_allocation: o.run,
        }
    } else if let Some(source) = o.import {
        match phase {
            Phase::Phonon => EpwCommand::QeData {
                command: crate::qe_data::cli::QeDataCommand::Import {
                    config: o.config,
                    source,
                },
            },
            Phase::Wannier => EpwCommand::Interpolation {
                command: crate::interpolation::cli::InterpolationCommand::Import {
                    config: o.config,
                    source,
                },
            },
            Phase::Superconductivity => EpwCommand::Superconductivity {
                command: crate::superconductivity::cli::SuperconductivityCommand::Import { source },
            },
        }
    } else if let Some(source) = o.export_comparison {
        ensure!(
            phase == Phase::Wannier,
            "input_conflict: 叠图来源导出属于 epw_wannier"
        );
        EpwCommand::Interpolation {
            command: crate::interpolation::cli::InterpolationCommand::ExportComparison {
                source,
                output: o.output.context("required: 需要 --output")?,
            },
        }
    } else if o.collect || o.plot || o.results_check {
        ensure!(
            phase != Phase::Phonon,
            "input_conflict: 第一阶段不提供 EPW 绘图操作"
        );
        if o.collect && o.json {
            return crate::delivery::cli::run(crate::delivery::cli::DeliveryCommand::Collect {
                work: work()?,
                json: true,
            });
        }
        if o.collect {
            EpwCommand::Collect {
                work: work()?,
                output: o.output,
            }
        } else if o.plot {
            EpwCommand::Plot {
                results: results()?,
                output: o.output,
                formats: o.formats,
            }
        } else {
            EpwCommand::Results {
                command: ResultsCommand::Check {
                    results: results()?,
                    json: o.json,
                },
            }
        }
    } else if let Some(operation) = o.budget {
        let ledger = o.ledger.context("required: 账本操作需要 --ledger")?;
        let action = match operation {
            BudgetOperation::Status => BudgetAction::Status {
                ledger,
                json: o.json,
            },
            BudgetOperation::Reconcile => BudgetAction::Reconcile { ledger },
            BudgetOperation::Attach => BudgetAction::Attach {
                ledger,
                token: o.token.context("required: 需要 --token")?,
                job: o.job,
            },
            BudgetOperation::Reserve => BudgetAction::Reserve {
                ledger,
                label: o.label.context("required: 需要 --label")?,
                nodes: o.nodes.context("required: 需要 --nodes")?,
                mpi: o.mpi.context("required: 需要 --mpi")?,
                wall_seconds: o.wall_seconds.context("required: 需要 --wall-seconds")?,
                storage_bytes: o.storage_bytes.context("required: 需要 --storage-bytes")?,
            },
        };
        EpwCommand::Budget {
            action: Some(action),
        }
    } else {
        if o.resume {
            return workflow::resume_phase_with_mpi(&work()?, phase.boundary(), o.submit, o.run, o.mpi);
        }
        if o.submit || o.run {
            return workflow::start_phase(&o.config, phase.boundary(), o.submit);
        }
        let path = workflow::generate_phase(&o.config, phase.boundary())?;
        println!("script: {}\nworkflow_complete: false", path.display());
        return Ok(());
    };
    cli::dispatch_command(command)
}
