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
#[command(about = "生成、提交或恢复本阶段；默认读取当前 input.toml 并生成脚本")]
#[command(group(ArgGroup::new("operation").args(["check", "prepare_only", "status", "probe", "import", "migrate_state", "collect", "plot", "results_check", "export_comparison", "budget", "execute_attempt", "run_script", "all", "submit_chain"])))]
struct Options {
    #[arg(short = 'c', long, default_value = "input.toml")]
    config: PathBuf,
    /// 编号工作区：选择本程序的独立科学阶段。
    #[arg(long, value_enum)]
    stage: Option<super::stages::Step>,
    /// 一次性校验并导入旧 epwflow/work，保留原来源。
    #[arg(long)]
    import_work: Option<PathBuf>,
    /// 一次性迁移旧编号工作区的隐藏状态；保留历史与预算，不提交计算。
    #[arg(long, conflicts_with_all = ["submit", "run", "status", "check", "import_work", "all", "resume"])]
    migrate_state: bool,
    #[arg(long, hide = true)]
    workspace_attempt: Option<PathBuf>,
    #[arg(long, hide = true)]
    workspace_task: Option<PathBuf>,
    #[arg(short = 'w', long)]
    work: Option<PathBuf>,
    /// 核验上一任务并提交下一次原生调用，作业结束后再次执行 -s 推进。
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
    /// 仅 epw_phonon：准备当前就绪的原生脚本与待办清单，不提交。
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

fn dispatch(phase: Phase, mut o: Options) -> Result<()> {
    if o.config==PathBuf::from("input.toml") && !o.config.exists() && PathBuf::from("inputepw.toml").is_file(){o.config=PathBuf::from("inputepw.toml");}
    if let Some(path)=&o.workspace_attempt { return super::workspace::execute(path); }
    if let Some(path)=&o.workspace_task { return super::workspace::execute_task(path); }
    if let Some(source)=&o.import_work { return super::workspace::import_legacy(&o.config,source); }
    if o.migrate_state { return super::run_state::migrate(&super::config::Config::load(&o.config)?); }
    if let Some(work) = &o.work { super::run_state::ready(work)?; }
    let numbered = if o.config.is_file() && o.work.as_ref().is_none_or(|w|super::layout::enabled(w)) {
        super::config::Config::load(&o.config)?.layout.as_deref()==Some("numbered")
    } else {false};
    if numbered && !o.probe && o.budget.is_none() && o.import.is_none() && !o.collect && !o.plot && !o.results_check && o.export_comparison.is_none() {
        use super::stages::Step;
        ensure!(!o.all || phase == Phase::Phonon, "input_conflict: --all 仅由 epw_phonon 管理");
        ensure!(!o.resume || o.submit || o.run, "required: --resume 需要 -s 或 -r");
        let target=match (phase,o.stage) {
            (Phase::Phonon,None)=>Step::Phonon,
            (Phase::Wannier,None)=>Step::Coarse,
            (Phase::Wannier,Some(s @ (Step::Wannier|Step::Coarse)))=>s,
            (Phase::Superconductivity,None)=>Step::Superconductivity,
            (Phase::Superconductivity,Some(s @ (Step::Fine|Step::Superconductivity)))=>s,
            _=>anyhow::bail!("input_conflict: --stage 不属于当前程序"),
        };
        ensure!(o.mpi.is_none(),"configuration: 编号工作区请修改 input.toml 的资源及启动器/pool");
        let cfg=super::config::Config::load(&o.config)?;
        super::run_state::ready(&cfg.work_dir)?;
        if o.status {if o.json{status::print_json(&super::workspace::report(&cfg)?);return Ok(())}return super::workspace::status(&cfg)}
        if o.check {for s in target.closure(){if s==target || !super::stages::reusable(&cfg,s)?{super::parallelism::preflight(&cfg,s,false)?;}} if o.json{status::print_json(&serde_json::json!({"operation_status":"ok","workflow_status":"configured","workflow_complete":false,"checks":["configuration","input_files","program_paths","mpi_pool_image"]}));}else{println!("operation_status: ok\nworkflow_status: configured");}return Ok(())}
        if o.submit {return super::native_workspace::advance(&o.config,target,true,o.resume)}
        if o.run {return super::workspace::start(&o.config,target,false)}
        super::native_workspace::advance(&o.config,if o.all {Step::Superconductivity}else{target},false,false)?;
        return Ok(())
    }
    ensure!(o.stage.is_none(),"input_conflict: --stage 需要 layout='numbered' 的当前配置");
    if o.submit && !o.probe && o.budget.is_none() && o.work.is_none() {
        super::config::Config::load(&o.config)?;
    }
    ensure!(!o.submit || o.probe || o.budget.is_some(),
        "migration_required: 新原生提交使用 layout='numbered'；旧工作区请设置新的 work_dir 并使用 --import-work 显式导入，禁止继续提交内部执行器包装脚本");
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
