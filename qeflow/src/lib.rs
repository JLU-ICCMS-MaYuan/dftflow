//! qeflow 单一静态二进制入口。
//! 子命令及 argv[0] 兼容入口映射原 qe_* CLI。

#[path = "phonon_chain.rs"]
mod phonon_chain;
#[path = "phonon_init.rs"]
mod phonon_init;
pub mod native;

use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::path::Path;

use qf_input::{electronic as electronic_input, matdyn as matdyn_input, ph as ph_input, q2r as q2r_input};
use qf_state::{refresh_workflow_status, split_resume_step, TaskState};

#[derive(Parser)]
#[command(name = "qeflow", version, about = "Rust rewrite of qeflow (single static binary)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(hide = true)]
    QePhInitRun {
        #[arg(short, long)]
        config: String,
        #[arg(short, long)]
        input: Option<String>,
    },
    /// SCF calculation
    QeScf {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'r', long, conflicts_with = "submit")]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        poscar: Option<String>,
    },
    /// NSCF calculation
    QeNscf {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'r', long, conflicts_with = "submit")]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        poscar: Option<String>,
    },
    /// Electronic band structure
    QeEband {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        poscar: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
    },
    /// Electronic DOS
    QeElEdos {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        poscar: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
    },
    /// Phonon workflow (nosplit)
    QePhononNosplit {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
    },
    /// Phonon split by index
    QePhononSplitIndex {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
    },
    /// Phonon split by coords
    QePhononSplitCoords {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
    },
    /// Merge split phonon results
    #[command(alias = "qe-merge")]
    QePhononMerge {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        input: Option<String>,
    },
    /// Process phonon (q2r + matdyn + collect + plot)
    QeProcessPhonon {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long = "input", visible_short_alias = 'p', visible_alias = "poscar")]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long)]
        submit: bool,
        /// 仅归档已有 q2r/matdyn 结果（phband/phdos）
        #[arg(long)]
        collect: bool,
        /// 仅读取归档结果并绘图
        #[arg(long)]
        plot: bool,
    },
    /// Superconducting Tc (scflow)
    Scflow {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
    },
    /// 刷新并打印工作流状态与续算点（M3 状态机演示）
    Status {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short, long, default_value = ".")]
        work_dir: String,
        #[arg(long, default_value = "struct")]
        prefix: String,
    },
    /// Quantum ESPRESSO ph.x 单步任务（qe_ph_nosplit）
    QePhNosplit {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long)]
        input: Option<String>,
        /// 使用 QE only_init 正常初始化并生成 dyn0（需要已有 SCF 数据）
        #[arg(long)]
        dyn0: bool,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long, conflicts_with = "run")]
        submit: bool,
    },
    /// Quantum ESPRESSO q2r.x 单步任务
    QeQ2r {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long)]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long, conflicts_with = "run")]
        submit: bool,
    },
    /// matdyn.x 声子能带单步任务
    QeMatdynPhband {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long)]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long, conflicts_with = "run")]
        submit: bool,
    },
    /// matdyn.x 声子 DOS 单步任务
    QeMatdynPhdos {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long)]
        input: Option<String>,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long, conflicts_with = "run")]
        submit: bool,
    },
    /// 提交 split 声子 q 点任务
    QeSplitSubmit {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(short = 'i', long)]
        input: Option<String>,
        /// split q 点物化模式
        #[arg(long)]
        mode: String,
        #[arg(long)]
        chain: bool,
    },
    #[command(hide = true)]
    QePhononChainStage {
        #[arg(short, long)] config: String,
        #[arg(short = 'i', long)] input: Option<String>,
        #[arg(long)] mode: String,
        #[arg(long)] stage: String,
        #[arg(long)] q_dir: Option<String>,
    },
    /// QE 输出状态检查器
    QeCheck {
        #[arg(short, long, default_value = "input.toml")]
        config: Option<String>,
        #[arg(long)]
        scf: bool,
        #[arg(long)]
        relax: bool,
        #[arg(long)]
        ph: bool,
        #[arg(long)]
        q2r: bool,
        #[arg(long)]
        matdyn_band: bool,
        #[arg(long)]
        matdyn_dos: bool,
        #[arg(long)]
        eph: bool,
    },
    /// 处理 QE projwfc 能带投影（qe_process_eband）
    QeProcessEband {
        /// projwfc 投影文件（如 *.projwfc_up）
        #[arg(short = 'i', long)]
        input: String,
        /// bands.x 输出文件
        #[arg(short = 'b', long, default_value = "elebanddata.dat")]
        bandfile: String,
        /// 包含晶胞信息的 eband.in
        #[arg(long, default_value = "eband.in")]
        cell: String,
        /// 输出文件名，默认 <prefix>_eleband_proj.dat
        #[arg(short = 'o', long)]
        output: Option<String>,
        /// 输出文件前缀（覆盖自动推断）
        #[arg(long)]
        prefix: Option<String>,
    },
    /// 处理 QE DOS/PDOS 数据输出表格（qe_process_eledos）
    QeProcessElEdos {
        /// TDOS 文件路径或包含 DOS 文件的目录
        #[arg(short = 'i', long, default_value = ".")]
        input: String,
        /// projwfc 文件路径（用于解析壳层编号）
        #[arg(long)]
        proj: Option<String>,
    },

}

/// 旧命令名 → 子命令（argv[0] 多调用分派，配合 install-compat.sh 的硬链接）。
const LEGACY_ENTRY_NAMES: &[(&str, &str)] = &[
    ("qe_scf", "qe-scf"),
    ("qe_nscf", "qe-nscf"),
    ("qe_eband", "qe-eband"),
    ("qe_eledos", "qe-el-edos"),
    ("qe_process_eband", "qe-process-eband"),
    ("qe_process_eledos", "qe-process-el-edos"),
    ("qe_ph_nosplit", "qe-ph-nosplit"),
    ("qe_merge", "qe-merge"),
    ("qe_q2r", "qe-q2r"),
    ("qe_matdyn_phband", "qe-matdyn-phband"),
    ("qe_matdyn_phdos", "qe-matdyn-phdos"),
    ("qe_process_phonon", "qe-process-phonon"),
    ("qe_split_submit", "qe-split-submit"),
    ("qe_check", "qe-check"),
    ("qe_phonon_nosplit", "qe-phonon-nosplit"),
    ("qe_phonon_split_index", "qe-phonon-split-index"),
    ("qe_phonon_split_coords", "qe-phonon-split-coords"),
];

pub fn run_cli() -> Result<()> {
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    // argv[0] 旧名分派：把旧命令名改写为对应子命令。
    if let Some(argv0) = args.first() {
        let entry = std::path::Path::new(argv0)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some((_, sub)) = LEGACY_ENTRY_NAMES.iter().find(|(name, _)| *name == entry) {
            args[0] = std::env::current_exe()
                .map(|p| p.into_os_string())
                .unwrap_or_else(|_| argv0.clone());
            args.insert(1, std::ffi::OsString::from(*sub));
        }
    }
    // 兼容 Python argparse 的单横线长选项（qe_check 的 -scf/-relax/-ph/-q2r/...）。
    let raw_args: Vec<std::ffi::OsString> = args
        .into_iter()
        .map(|a| {
            let s = a.to_string_lossy().into_owned();
            match s.as_str() {
                "-scf" | "-relax" | "-ph" | "-q2r" | "-matdyn-band" | "-matdyn-dos" | "-eph" => {
                    std::ffi::OsString::from(format!("-{s}"))
                }
                _ => a,
            }
        })
        .collect();
    let cli = Cli::parse_from(raw_args);
    match cli.command {
        Commands::QePhInitRun { config, input } => phonon_init::execute(config, input),
        Commands::QeScf { config, run, submit, poscar } => {
            run_qe_pw("scf", config, poscar, run, submit)
        }
        Commands::QeNscf { config, run, submit, poscar } => {
            run_qe_pw("nscf", config, poscar, run, submit)
        }
        Commands::QeEband { config, poscar, run, submit } => {
            run_qe_eband(config, poscar, run, submit)
        }
        Commands::QeElEdos { config, poscar, run, submit } => {
            run_qe_eledos(config, poscar, run, submit)
        }
        Commands::QePhononNosplit { config, input, run, submit } => {
            run_qe_phonon_nosplit(config, input, run, submit)
        }
        Commands::QePhononSplitIndex { config, input, run, submit } => {
            phonon_chain::run(config, input, run, submit, "index")
        }
        Commands::QePhononSplitCoords { config, input, run, submit } => {
            phonon_chain::run(config, input, run, submit, "coords")
        }
        Commands::QePhononMerge { config, input } => run_qe_phonon_merge(config, input),
        Commands::QeProcessPhonon {
            config,
            input,
            run,
            submit,
            collect,
            plot,
        } => run_qe_process_phonon(config, input, run, submit, collect, plot),
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
        Commands::QePhNosplit { config, input, dyn0, run, submit } => {
            run_ph_nosplit(config, input, dyn0, run, submit)
        }
        Commands::QeQ2r { config, input, run, submit } => run_q2r_step(config, input, run, submit),
        Commands::QeMatdynPhband { config, input, run, submit } => {
            run_matdyn_phband(config, input, run, submit)
        }
        Commands::QeMatdynPhdos { config, input, run, submit } => {
            run_matdyn_phdos(config, input, run, submit)
        }
        Commands::QePhononChainStage {config,input,mode,stage,q_dir} => phonon_chain::stage(config,input,mode,stage,q_dir),
        Commands::QeSplitSubmit { config, input, mode, chain } => {
            if chain { return phonon_chain::stage(config.unwrap_or_else(|| "input.toml".into()),input,mode,"fanout".into(),None); }
            let code = run_split_submit(config, input, &mode)?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Commands::QeCheck { config, scf, relax, ph, q2r, matdyn_band, matdyn_dos, eph } => {
            run_qe_check(config, scf, relax, ph, q2r, matdyn_band, matdyn_dos, eph)
        }
        Commands::QeProcessEband { input, bandfile, cell, output, prefix } => {
            let written = qf_io::qe_proj::run_process_eband(
                &input,
                &bandfile,
                &cell,
                output.as_deref(),
                prefix.as_deref(),
            )?;
            println!("已生成投影能带数据: {}", written[0]);
            println!("已生成元素汇总投影能带数据: {}", written[1]);
            println!("已生成能带数据: {}", written[2]);
            Ok(())
        }
        Commands::QeProcessElEdos { input, proj } => {
            let written = qf_io::qe_proj::run_process_eledos(&input, proj.as_deref())?;
            for w in &written {
                println!("已生成: {w}");
            }
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

/// 取出结构元素与计数（用于 DOS 元素分波表头）。
fn structure_elements(structure: &qf_crystal::Structure) -> (Vec<String>, Vec<usize>) {
    let elements = structure.elements.clone();
    let counts = structure.counts.clone();
    (elements, counts)
}

fn print_resume_point(ctx: &PhononCtx) -> Result<()> {
    let status = refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    let (step, label) = split_resume_step(&status);
    if step == 0 {
        println!("[status] 全部完成，无需续算");
    } else {
        println!("[status] 续算点: {step}/11 {label}");
    }
    Ok(())
}

/// `qe_phonon_merge` 真实逻辑：回收各 split 目录的 dyn / phsave / q 点目录到 workspace。
fn run_qe_phonon_merge(config: Option<String>, input: Option<String>) -> Result<()> {
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let work_dir = cfg
        .qe_phonon
        .work_dir
        .clone()
        .unwrap_or_else(|| ".".to_string());
    let prefix = cfg.qe_phonon.prefix.clone().or_else(|| cfg.qe_phonon.scf.get("prefix").and_then(|v|v.as_str()).map(str::to_owned)).unwrap_or_else(|| "struct".into());
    let _ = input;
    let map = qf_phonon::index_map::load_q_index_map(&work_dir);
    if map.qpoints.is_empty() {
        anyhow::bail!("缺少 q_index_map.json 或其中不包含 qpoints，请先运行 qe_phonon_split_*");
    }
    let outdir = cfg.qe_phonon.scf.get("outdir").and_then(|v|v.as_str()).unwrap_or("./tmp");
    let result = if map.qpoints.iter().all(|e|e.q_cart.is_some()) { qf_phonon::merge::merge_split_results_with_outdir(&work_dir,&prefix,&map,outdir)? } else { qf_phonon::merge_split_results(&work_dir,&prefix,&map)? };
    if result.success {
        println!("[qe_phonon_merge] 成功回收 {} 个 q 点 dyn 文件", result.dyn_summaries.len());
        for (idx, vals) in &result.dyn_summaries {
            let formatted = vals
                .iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!("  {prefix}.dyn{idx}: {formatted}");
        }
    } else {
        for e in &result.errors {
            println!("[qe_phonon_merge][error] {e}");
        }
        anyhow::bail!("merge 失败，请先完成各 split ph.x 计算");
    }
    Ok(())
}

/// 写出带 step 标签的运行脚本；按 `-r`（本地运行）或 `--submit`（Slurm 提交）决定执行方式，
/// 二者皆无则仅 dry-run。`work_dir` 与 `prefix` 由各命令显式传入（eband/eledos 与 phonon 不同）。
fn run_qe_steps(
    cfg: &qf_config::Config,
    work_dir: &str,
    prefix: &str,
    run: bool,
    submit: bool,
    steps: Vec<(String, String)>,
    script_name: &str,
    name: &str,
) -> Result<()> {
    let absolute = std::fs::canonicalize(work_dir)?;
    let slurm_header = cfg.slurm.header.clone().unwrap_or_default();
    let ctx = qf_runner::ScriptContext {
        workspace: &absolute,
        prefix,
        slurm_header: &slurm_header,
    };
    let script = if script_name == "run_qe.sh" {
        let mut commands = vec!["set -e".to_string(), "echo 'Job started at' `date`".to_string()];
        commands.extend(steps.iter().map(|(_, cmd)| cmd.clone()));
        commands.push("echo 'Job finished at' `date`".to_string());
        qf_runner::write_job_script(&absolute, script_name, &slurm_header, &commands)?
    } else {
        qf_runner::write_labeled_script(&ctx, script_name, &steps, 1, steps.len(), None, Some(script_name))?
    };
    println!("[{name}] wrote {script:?} (work_dir={work_dir})");
    if run {
        qf_runner::run_shell(&format!("bash {script_name}"), Path::new(&work_dir))?;
    } else if submit {
        if script_name == "run_qe.sh" {
            qf_runner::run_shell("sbatch run_qe.sh", &absolute)?;
            return Ok(());
        }
        if let Some(job) =
            qf_runner::submit_via_sbatch(Path::new(&work_dir), script_name)?
        {
            println!("[{name}] submitted Slurm job {job}");
        }
    } else {
        println!("[{name}] (dry-run) 未指定 -r/--submit，仅写出输入与脚本");
    }
    Ok(())
}

/// 取某外部程序的 launcher（优先 `merged[section].executable_path`，否则回退 `qe.executable_path`）。
fn exec_for(merged: &qf_config::SectionMap, section: &str, qe_exec: &str) -> String {
    merged
        .get(section)
        .and_then(|v| v.get("executable_path"))
        .and_then(|v| v.as_str())
        .unwrap_or(qe_exec)
        .to_string()
}

/// 拼外部程序命令：`exec [args] < in > out`；`args` 为空时仅 `exec < in > out`（对齐 Python 的 strip 语义）。
fn with_args(exec_cmd: &str, args: &str, redirect: &str) -> String {
    if args.is_empty() {
        format!("{exec_cmd} {redirect}")
    } else {
        format!("{exec_cmd} {args} {redirect}")
    }
}

/// `qe_phonon_nosplit` 真实逻辑（对齐 `phonon/qe_phonon_nosplit.py`）：准备
/// scf/ph/q2r/matdyn 全部输入与脚本，按状态推进剩余步骤；-r 本地顺序运行并出图，
/// -s 生成聚合 `submit_phonon_workflow.sh` 提交。
pub fn run_qe_phonon_nosplit(config: Option<String>, input: Option<String>, run: bool, submit: bool) -> Result<()> {
    run_nosplit(config, input, run, submit, None)
}

/// 已登记分配内的共享执行入口；SCF 校验后、PH 启动前交付快照检查点。
pub fn run_nosplit_with_scf_checkpoint(
    config: String, input: String, checkpoint: &mut dyn FnMut(&Path) -> Result<()>,
) -> Result<()> {
    run_nosplit(Some(config), Some(input), true, false, Some(checkpoint))
}

fn run_nosplit(config: Option<String>, input: Option<String>, run: bool, submit: bool, mut checkpoint: Option<&mut dyn FnMut(&Path) -> Result<()>>) -> Result<()> {
    let source_config = std::fs::canonicalize(config.as_deref().unwrap_or("input.toml"))?;
    let input = if source_config.file_name().and_then(|p| p.to_str()) == Some("input-init.toml") {
        Some(phonon_chain::snapshot_structure(source_config.parent().unwrap().to_str().unwrap())?)
    } else { input };
    let ctx = create_phonon_context(config.as_deref(), input.as_deref(), Some("qe_phonon_nosplit"), true)?;
    let (frozen_config, frozen_structure) = phonon_chain::snapshot_inputs(&ctx, source_config.to_str().unwrap(), input.as_deref())?;
    prepare_root_scf(&ctx)?;
    prepare_ph_step(&ctx)?;
    q2r_input::write_q2r_input(&ctx.workspace, &ctx.cfg, &ctx.prefix)?;
    let q2r_exec = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "q2r.x");
    let matdyn_exec = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "matdyn.x");
    matdyn_input::write_matdyn_phband_input(
        &ctx.workspace,
        &ctx.cfg,
        &ctx.prefix,
        ctx.structure.lattice,
    )?;
    matdyn_input::write_matdyn_phdos_input(&ctx.workspace, &ctx.cfg, &ctx.prefix)?;
    let slurm_header = ctx.cfg.slurm.header.clone().unwrap_or_default();
    write_root_script(&ctx.workspace, "run_q2r.sh", &slurm_header, &[format!("{q2r_exec} < q2r.in > q2r.out")])?;
    write_root_script(
        &ctx.workspace,
        "run_matdyn_phband.sh",
        &slurm_header,
        &[format!("{matdyn_exec} < matdyn_phband.in > matdyn_phband.out")],
    )?;
    write_root_script(
        &ctx.workspace,
        "run_matdyn_phdos.sh",
        &slurm_header,
        &[format!("{matdyn_exec} < matdyn_phdos.in > matdyn_phdos.out")],
    )?;

    let status = refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    let mut needed: Vec<(String, String)> = Vec::new();
    if status.scf != TaskState::Finished {
        needed.push(("运行根目录 SCF".to_string(), "bash run_scf.sh".to_string()));
    }
    if status.ph.state != TaskState::Finished {
        needed.push(("运行根目录 PH".to_string(), "bash run_ph.sh".to_string()));
    }
    let process_needed = status.q2r != TaskState::Finished
        || status.matdyn_phband != TaskState::Finished
        || status.matdyn_phdos != TaskState::Finished;
    if status.q2r != TaskState::Finished {
        needed.push(("运行 q2r".to_string(), "bash run_q2r.sh".to_string()));
    }
    if status.matdyn_phband != TaskState::Finished {
        needed.push(("运行 matdyn 声子谱".to_string(), "bash run_matdyn_phband.sh".to_string()));
    }
    if status.matdyn_phdos != TaskState::Finished {
        needed.push(("运行 matdyn 声子 DOS".to_string(), "bash run_matdyn_phdos.sh".to_string()));
    }

    if run {
        for (_, command) in &needed {
            if command == "bash run_ph.sh" {
                if let Some(checkpoint) = checkpoint.as_mut() { checkpoint(Path::new(&ctx.workspace))?; }
            }
            qf_runner::run_shell(command, Path::new(&ctx.workspace))?;
            let (log, scf) = match command.as_str() {
                "bash run_scf.sh" => ("scf.out", true),
                "bash run_ph.sh" => ("ph.out", false),
                "bash run_q2r.sh" => ("q2r.out", false),
                "bash run_matdyn_phband.sh" => ("matdyn_phband.out", false),
                _ => ("matdyn_phdos.out", false),
            };
            phonon_init::checked_log(&Path::new(&ctx.workspace).join(log), scf)?;
        }
        // 后处理：归档 + 表格 + 绘图（对齐 run_process_phonon_steps(include_plot=True)）
        let (elements, counts) = structure_elements(&ctx.structure);
        qf_phonon::collect_phband_outputs(&ctx.workspace, &ctx.prefix)?;
        qf_phonon::collect_phdos_outputs(&ctx.workspace, &ctx.prefix)?;
        let (band_rows, label_rows) =
            qf_phonon::build_phonon_band_tables(&ctx.workspace, &ctx.prefix, &elements, &counts)?;
        let (atoms_rows, elements_rows, _) =
            qf_phonon::build_phonon_dos_tables(&ctx.workspace, &ctx.prefix, &elements, &counts)?;
        qf_phonon::write_csv(&format!("{}/phband/phonon_band.csv", ctx.workspace), &band_rows)?;
        qf_phonon::write_csv(&format!("{}/phband/phonon_band_labels.csv", ctx.workspace), &label_rows)?;
        qf_phonon::write_csv(&format!("{}/phdos/phonon_dos_atoms.csv", ctx.workspace), &atoms_rows)?;
        qf_phonon::write_csv(&format!("{}/phdos/phonon_dos_elements.csv", ctx.workspace), &elements_rows)?;
        qf_phonon::plot::plot_phonon_band(&ctx.workspace, &ctx.prefix)?;
        qf_phonon::plot::plot_phonon_dos(&ctx.workspace, &ctx.prefix, &elements, &counts, qf_phonon::plot::dos_plot_sigma(&ctx.cfg)?)?;
        qf_phonon::plot::plot_phonon_combined(&ctx.workspace, &ctx.prefix, &elements, &counts, qf_phonon::plot::dos_plot_sigma(&ctx.cfg)?)?;
        export_epw_if_requested(&ctx.cfg, Path::new(&ctx.workspace), &ctx.prefix)?;
        println!("声子流程已完成：{}", ctx.workspace);
    } else if submit {
        if needed.is_empty() {
            println!("声子流程已完成：{}", ctx.workspace);
        } else {
            let mut steps = needed;
            if process_needed {
                steps.push((
                    "collect phonon".to_string(),
                    format!("{} qe-process-phonon -c {} -i {} --collect", phonon_chain::quote(std::env::current_exe()?.to_str().unwrap()), phonon_chain::quote(&frozen_config), phonon_chain::quote(&frozen_structure)),
                ));
                steps.push((
                    "plot".to_string(),
                    format!("{} qe-process-phonon -c {} -i {} --plot", phonon_chain::quote(std::env::current_exe()?.to_str().unwrap()), phonon_chain::quote(&frozen_config), phonon_chain::quote(&frozen_structure)),
                ));
            }
            let script_ctx = qf_runner::ScriptContext {
                workspace: Path::new(&ctx.workspace),
                prefix: &ctx.prefix,
                slurm_header: &slurm_header,
            };
            qf_runner::write_labeled_script(
                &script_ctx,
                "submit_phonon_workflow.sh",
                &steps,
                1,
                steps.len(),
                None,
                Some("声子总工作流"),
            )?;
            if let Some(job) = qf_runner::submit_via_sbatch(Path::new(&ctx.workspace), "submit_phonon_workflow.sh")? {
                println!("已提交声子流程作业 {job}");
            }
        }
    } else {
        println!("已准备声子流程输入与脚本：{}", ctx.workspace);
    }
    refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    Ok(())
}

fn export_epw_if_requested(cfg: &qf_config::Config, work: &Path, prefix: &str) -> Result<()> {
    if cfg.qe_phonon.retain_epw_data {
        let outdir = cfg.qe_phonon.scf.get("outdir").and_then(|v| v.as_str()).unwrap_or("./tmp");
        qf_phonon::epw::export_nosplit(work, &work.join(outdir), prefix)?;
    }
    Ok(())
}

/// `qe_process_phonon` 真实逻辑：写出 `q2r.in` / `matdyn_phband.in` / `matdyn_phdos.in` 并运行；
/// `--collect` 仅归档已有结果到 `phband/` `phdos/`；`--plot` 仅读取归档结果绘图。
fn run_qe_process_phonon(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
    collect: bool,
    plot: bool,
) -> Result<()> {
    let mut cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let work_dir = cfg
        .qe_phonon
        .work_dir
        .clone()
        .unwrap_or_else(|| ".".to_string());
    let config_path = std::fs::canonicalize(config.as_deref().unwrap_or("input.toml"))?;
    let work_dir = config_path.parent().unwrap().join(&work_dir).to_string_lossy().into_owned();
    cfg.qe_phonon.work_dir = Some(work_dir.clone());
    let prefix = cfg.qe_phonon.prefix.clone().or_else(|| cfg.qe_phonon.scf.get("prefix").and_then(|v|v.as_str()).map(str::to_string)).unwrap_or_else(|| "struct".into());
    let pos_path = input.unwrap_or_else(|| format!("{work_dir}/POSCAR"));
    let structure = qf_crystal::parse_poscar(&pos_path)
        .map_err(|e| anyhow::anyhow!("解析 POSCAR 失败: {e}"))?;
    let (elements, counts) = structure_elements(&structure);

    if plot {
        // 重绘与 CSV 使用相同的已校验路径/频率，不复用可能过期的旧表格。
        let (band_rows, label_rows) = qf_phonon::build_phonon_band_tables(&work_dir, &prefix, &elements, &counts)?;
        let (atoms_rows, elements_rows, _) = qf_phonon::build_phonon_dos_tables(&work_dir, &prefix, &elements, &counts)?;
        std::fs::create_dir_all(format!("{work_dir}/phband"))?;
        std::fs::create_dir_all(format!("{work_dir}/phdos"))?;
        qf_phonon::write_csv(&format!("{work_dir}/phband/phonon_band.csv"), &band_rows)?;
        qf_phonon::write_csv(&format!("{work_dir}/phband/phonon_band_labels.csv"), &label_rows)?;
        qf_phonon::write_csv(&format!("{work_dir}/phdos/phonon_dos_atoms.csv"), &atoms_rows)?;
        qf_phonon::write_csv(&format!("{work_dir}/phdos/phonon_dos_elements.csv"), &elements_rows)?;
        let band_png = qf_phonon::plot::plot_phonon_band(&work_dir, &prefix)?;
        let dos_png = qf_phonon::plot::plot_phonon_dos(&work_dir, &prefix, &elements, &counts, qf_phonon::plot::dos_plot_sigma(&cfg)?)?;
        let combined = qf_phonon::plot::plot_phonon_combined(&work_dir, &prefix, &elements, &counts, qf_phonon::plot::dos_plot_sigma(&cfg)?)?;
        println!("[qe_process_phonon] 绘图完成: {band_png}, {dos_png}, {combined}");
        return Ok(());
    }

    if collect {
        qf_phonon::collect_phband_outputs(&work_dir, &prefix)?;
        qf_phonon::collect_phdos_outputs(&work_dir, &prefix)?;
        // 表格
        let (band_rows, label_rows) =
            qf_phonon::build_phonon_band_tables(&work_dir, &prefix, &elements, &counts)?;
        let (atoms_rows, elements_rows, _) =
            qf_phonon::build_phonon_dos_tables(&work_dir, &prefix, &elements, &counts)?;
        qf_phonon::write_csv(&format!("{work_dir}/phband/phonon_band.csv"), &band_rows)?;
        qf_phonon::write_csv(&format!("{work_dir}/phband/phonon_band_labels.csv"), &label_rows)?;
        qf_phonon::write_csv(&format!("{work_dir}/phdos/phonon_dos_atoms.csv"), &atoms_rows)?;
        qf_phonon::write_csv(&format!("{work_dir}/phdos/phonon_dos_elements.csv"), &elements_rows)?;
        export_epw_if_requested(&cfg, Path::new(&work_dir), &prefix)?;
        println!("[qe_process_phonon] 归档并生成表格完成");
        return Ok(());
    }

    let q2r_path = q2r_input::write_q2r_input(&work_dir, &cfg, &prefix)?;
    let phband_path = matdyn_input::write_matdyn_phband_input(&work_dir, &cfg, &prefix, structure.lattice)?;
    let phdos_path = matdyn_input::write_matdyn_phdos_input(&work_dir, &cfg, &prefix)?;
    println!("[qe_process_phonon] wrote {q2r_path}, {phband_path}, {phdos_path}");
    let collect_cmd=format!("{} qe-process-phonon -c {} -i {}", phonon_chain::quote(std::env::current_exe()?.to_str().unwrap()), phonon_chain::quote(config_path.to_str().unwrap()), phonon_chain::quote(std::fs::canonicalize(&pos_path)?.to_str().unwrap()));
    let exec = cfg.qe.executable_path();
    let steps = vec![
        (
            "运行 q2r".to_string(),
            format!("{} < q2r.in > q2r.out", qf_runner::resolve_qe_command(exec, "q2r.x")),
        ),
        (
            "运行 matdyn 声子谱".to_string(),
            format!(
                "{} < matdyn_phband.in > matdyn_phband.out",
                qf_runner::resolve_qe_command(exec, "matdyn.x")
            ),
        ),
        (
            "运行 matdyn 声子 DOS".to_string(),
            format!(
                "{} < matdyn_phdos.in > matdyn_phdos.out",
                qf_runner::resolve_qe_command(exec, "matdyn.x")
            ),
        ),
        (
            "归档并生成表格".to_string(),
            format!("{collect_cmd} --collect"),
        ),
        (
            "绘图".to_string(),
            format!("{collect_cmd} --plot"),
        ),
    ];
    run_qe_steps(
        &cfg,
        &work_dir,
        &prefix,
        run,
        submit,
        steps,
        "run_q2r_matdyn.sh",
        "qe_process_phonon",
    )
}

/// 在准备阶段把 SCF 的 `.save` 数据目录拷入目标工作流目录（对齐 `_copy_scf_save_files`）。
/// `copy_tree=true` 复制整个 `.save`；源缺失时仅告警并继续（与 Python 行为一致）。
fn copy_scf_save(work_dir: &str, prefix: &str, outdir: &str, copy_tree: bool) -> Result<()> {
    let source = Path::new("qe_scf").join(outdir).join(format!("{prefix}.save"));
    let target = Path::new(work_dir).join(outdir).join(format!("{prefix}.save"));
    if !source.exists() {
        println!("[warn] 找不到 SCF 数据目录 {}", source.display());
        return Ok(());
    }
    if source == target { return Ok(()); }
    std::fs::create_dir_all(&target)?;
    if copy_tree {
        copy_save_tree(&source, &target)?;
    } else {
        for name in ["data-file-schema.xml", "charge-density.dat", "paw.txt"] {
            if source.join(name).is_file() {
                std::fs::copy(source.join(name), target.join(name))?;
            }
        }
    }
    Ok(())
}

/// `qe_eband` 真实逻辑：写 eband.in（pw.x bands）+ elebanddata.in + elebandprojdata.in + labelinfo，
/// 生成 `run_qe.sh`（pw.x → bands.x → projwfc.x）。KPATH.in 需用户预先提供（vaspkit 自动生成留待后续）。
fn run_qe_eband(
    config: Option<String>,
    poscar: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let pos_path = poscar.unwrap_or_else(|| "POSCAR".to_string());
    let structure = qf_crystal::parse_poscar(&pos_path)
        .map_err(|e| anyhow::anyhow!("解析 POSCAR 失败: {e}"))?;
    let prefix = structure.formula.clone();
    let work_dir = "qe_eband";
    std::fs::create_dir_all(work_dir)?;
    std::fs::copy(&pos_path, Path::new(work_dir).join("POSCAR"))?;
    let section = electronic_input::eband_merged(&cfg);
    let outdir = section.get("CONTROL").and_then(|s| s.get("outdir")).and_then(|v| v.as_str()).unwrap_or("./tmp").trim_matches('\'').trim_matches('"');
    copy_scf_save(work_dir, &prefix, outdir, false)?;
    let kpath = section.get("k_points").and_then(|s| s.get("kpath_file")).and_then(|v| v.as_str()).unwrap_or("KPATH.in");
    if !Path::new(work_dir).join(kpath).exists() {
        matdyn_input::run_vaspkit_for_kpath(work_dir)?;
    }
    std::fs::create_dir_all(Path::new(work_dir).join(outdir))?;
    let written = electronic_input::write_eband_inputs(work_dir, &cfg, &structure, &prefix)?;
    for w in &written {
        println!("[qe_eband] wrote {w}");
    }
    let merged = electronic_input::eband_merged(&cfg);
    let qe_exec = cfg.qe.executable_path();
    let pw = qf_runner::resolve_qe_command(qe_exec, "pw.x");
    let bands_exec = qf_runner::resolve_qe_command(&exec_for(&merged, "bands", qe_exec), "bands.x");
    let proj_exec = qf_runner::resolve_qe_command(&exec_for(&merged, "projwfc", qe_exec), "projwfc.x");
    let bands_args = merged
        .get("bands")
        .and_then(|v| v.get("args"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let proj_args = merged
        .get("projwfc")
        .and_then(|v| v.get("args"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let bands_cmd = with_args(&bands_exec, &bands_args, "< elebanddata.in > elebanddata.out");
    let proj_cmd = with_args(&proj_exec, &proj_args, "< elebandprojdata.in > elebandprojdata.out");
    let steps = vec![
        ("运行 pw.x 能带".to_string(), format!("{pw} < eband.in > eband.out")),
        ("运行 bands.x".to_string(), bands_cmd),
        ("运行 projwfc.x".to_string(), proj_cmd),
    ];
    run_qe_steps(
        &cfg,
        work_dir,
        &prefix,
        run,
        submit,
        steps,
        "run_qe.sh",
        "qe_eband",
    )
}

/// `qe_el_edos` 真实逻辑：写 eletdos.in（&dos）+ elepdos.in（&projwfc），先拷 SCF `.save`，
/// 生成 `run_qe.sh`（dos.x → projwfc.x）。
fn run_qe_eledos(
    config: Option<String>,
    poscar: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let pos_path = poscar.unwrap_or_else(|| "POSCAR".to_string());
    let structure = qf_crystal::parse_poscar(&pos_path)
        .map_err(|e| anyhow::anyhow!("解析 POSCAR 失败: {e}"))?;
    let prefix = structure.formula.clone();
    let work_dir = "qe_eledos";
    std::fs::create_dir_all(work_dir)?;
    std::fs::copy(&pos_path, Path::new(work_dir).join("POSCAR"))?;
    let merged = electronic_input::eledos_merged(&cfg);
    let outdir = merged
        .get("CONTROL")
        .and_then(|v| v.get("outdir"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
        .unwrap_or_else(|| "./tmp".to_string());
    copy_scf_save(work_dir, &prefix, &outdir, true)?;
    let written = electronic_input::write_eledos_inputs(work_dir, &cfg, &prefix)?;
    for w in &written {
        println!("[qe_el_edos] wrote {w}");
    }
    let qe_exec = cfg.qe.executable_path();
    let dos_exec = qf_runner::resolve_qe_command(&exec_for(&merged, "dos", qe_exec), "dos.x");
    let proj_exec = qf_runner::resolve_qe_command(&exec_for(&merged, "projwfc", qe_exec), "projwfc.x");
    let dos_args = merged
        .get("dos")
        .and_then(|v| v.get("args"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let proj_args = merged
        .get("projwfc")
        .and_then(|v| v.get("args"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let dos_cmd = with_args(&dos_exec, &dos_args, "< eletdos.in > eletdos.out");
    let proj_cmd = with_args(&proj_exec, &proj_args, "< elepdos.in > elepdos.out");
    let steps = vec![
        ("运行 dos.x".to_string(), dos_cmd),
        ("运行 projwfc.x".to_string(), proj_cmd),
    ];
    run_qe_steps(
        &cfg,
        work_dir,
        &prefix,
        run,
        submit,
        steps,
        "run_qe.sh",
        "qe_el_edos",
    )
}

// ---------------------------------------------------------------------------
// 声子工作流公共上下文（对齐 `qeflow/phonon/context.py` + `workspace.py` + `prefix.py`）
// ---------------------------------------------------------------------------

struct PhononCtx {
    cfg: qf_config::Config,
    workspace: String,
    prefix: String,
    structure: qf_crystal::Structure,
    structure_source: String,
}

const WORKFLOW_DEFAULT_WORKDIRS: &[(&str, &str)] = &[
    ("qe_phonon_nosplit", "qe_phonon_nosplit"),
    ("qe_phonon_split_coords", "qe_phonon_split_coords"),
    ("qe_phonon_split_index", "qe_phonon_split_index"),
];

const WORKSPACE_MARKERS: &[&str] = &[
    "q_index_map.json",
    "workflow_status.json",
    "run_scf.sh",
    "run_ph.sh",
    "run_q2r.sh",
    "run_matdyn_phband.sh",
    "run_matdyn_phdos.sh",
];

fn ensure_structure_source(struct_file: Option<&str>) -> Result<String> {
    if let Some(f) = struct_file {
        if !Path::new(f).exists() {
            anyhow::bail!("找不到结构文件: {f}");
        }
        return Ok(f.to_string());
    }
    for candidate in ["POSCAR", "CONTCAR"] {
        if Path::new(candidate).exists() {
            return Ok(candidate.to_string());
        }
    }
    anyhow::bail!("找不到结构文件，请使用 -i 指定，或在当前目录准备 POSCAR/CONTCAR。")
}

/// 复刻 `resolve_workspace` + `create_context` + `resolve_prefix`。
fn create_phonon_context(
    config_file: Option<&str>,
    struct_file: Option<&str>,
    workflow_name: Option<&str>,
    prepare_inputs: bool,
) -> Result<PhononCtx> {
    let cfg = qf_config::load(config_file, config_file)?;
    let default_work_dir = workflow_name
        .and_then(|w| {
            WORKFLOW_DEFAULT_WORKDIRS
                .iter()
                .find(|(name, _)| *name == w)
                .map(|(_, dir)| dir.to_string())
        })
        .unwrap_or_else(|| "qe_phonon".to_string());
    let work_dir_name = cfg
        .qe_phonon
        .work_dir
        .clone()
        .unwrap_or(default_work_dir);

    let current_dir = std::env::current_dir()?;
    let current_name = current_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let in_place = !Path::new(&work_dir_name).is_absolute() && (current_name == work_dir_name
        || WORKSPACE_MARKERS
            .iter()
            .any(|m| Path::new(m).exists()));
    let workspace = if in_place {
        current_dir.to_string_lossy().into_owned()
    } else {
        let ws = format!("{}/work_dir_marker_placeholder", "");
        let _ = ws;
        format!(
            "{}",
            current_dir.join(&work_dir_name).to_string_lossy()
        )
    };
    if prepare_inputs && !in_place {
        std::fs::create_dir_all(&workspace)?;
    }

    let mut original_source = format!("{workspace}/POSCAR");
    if prepare_inputs {
        let structure_target = format!("{workspace}/POSCAR");
        let structure_source = if struct_file.is_none() && Path::new(&structure_target).exists() {
            structure_target.clone()
        } else {
            ensure_structure_source(struct_file)?
        };
        original_source = structure_source.clone();
        if std::fs::canonicalize(&structure_source).ok()
            != std::fs::canonicalize(&structure_target).ok()
        {
            std::fs::copy(&structure_source, &structure_target).with_context(|| {
                format!("复制结构文件 {structure_source} -> {structure_target}")
            })?;
        }
    }

    let poscar_path = format!("{workspace}/POSCAR");
    let structure = qf_crystal::parse_poscar(&poscar_path)
        .map_err(|e| anyhow::anyhow!("解析 POSCAR 失败: {e}"))?;
    let prefix = cfg
        .qe_phonon
        .prefix
        .clone()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| structure.formula.clone());
    Ok(PhononCtx {
        cfg,
        workspace,
        prefix,
        structure,
        structure_source: original_source,
    })
}

/// 复刻 `input_deck/scripts.write_root_run_script`（announce=False）。
fn write_root_script(
    workspace: &str,
    script_name: &str,
    slurm_header: &str,
    commands: &[String],
) -> Result<String> {
    let mut lines: Vec<String> = vec!["#!/bin/bash".to_string()];
    let header = slurm_header.trim();
    if !header.is_empty() {
        lines.extend(header.lines().map(|l| l.to_string()));
    }
    lines.extend(commands.iter().cloned());
    let path = format!("{workspace}/{script_name}");
    std::fs::write(&path, format!("{}\n", lines.join("\n")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(path)
}

/// 复刻 `root_steps.prepare_root_scf_step`：写 scf.in + run_scf.sh。
fn prepare_root_scf(ctx: &PhononCtx) -> Result<String> {
    qf_input::pw::write_phonon_scf_input(&ctx.workspace, &ctx.cfg, &ctx.structure, &ctx.prefix)?;
    let pw = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "pw.x");
    let slurm_header = ctx.cfg.slurm.header.clone().unwrap_or_default();
    write_root_script(
        &ctx.workspace,
        "run_scf.sh",
        &slurm_header,
        &[format!("{pw} < scf.in > scf.out")],
    )
}

/// 写出完整 PH 的 ph.in 和 run_ph.sh；仅初始化由 phonon_init 处理。
fn prepare_ph_step(ctx: &PhononCtx) -> Result<String> {
    ph_input::write_ph_input(&ctx.workspace, &ctx.cfg, &ctx.prefix, "ph.in")?;
    let ph = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "ph.x");
    let slurm_header = ctx.cfg.slurm.header.clone().unwrap_or_default();
    write_root_script(
        &ctx.workspace,
        "run_ph.sh",
        &slurm_header,
        &[format!("{ph} < ph.in > ph.out")],
    )
}

/// `qe_ph_nosplit`：单步 ph.x（或 --dyn0 只生成 dyn0）。
fn run_ph_nosplit(
    config: Option<String>,
    input: Option<String>,
    dyn0_only: bool,
    run: bool,
    submit: bool,
) -> Result<()> {
    if dyn0_only {
        return phonon_init::entry(config, input, run, submit);
    }
    let ctx = create_phonon_context(config.as_deref(), input.as_deref(), None, true)?;
    prepare_root_scf(&ctx)?;
    let script = prepare_ph_step(&ctx)?;
    println!("[qe_ph_nosplit] wrote {script}");
    qf_runner::submit_or_run(run, submit, Path::new(&ctx.workspace), "run_ph.sh")?;
    refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    Ok(())
}

/// `qe_q2r`：单步 q2r.x。
fn run_q2r_step(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    let ctx = create_phonon_context(config.as_deref(), input.as_deref(), None, true)?;
    let q2r_path = q2r_input::write_q2r_input(&ctx.workspace, &ctx.cfg, &ctx.prefix)?;
    let q2r_exec = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "q2r.x");
    let slurm_header = ctx.cfg.slurm.header.clone().unwrap_or_default();
    let script = write_root_script(
        &ctx.workspace,
        "run_q2r.sh",
        &slurm_header,
        &[format!("{q2r_exec} < q2r.in > q2r.out")],
    )?;
    println!("[qe_q2r] wrote {q2r_path}, {script}");
    qf_runner::submit_or_run(run, submit, Path::new(&ctx.workspace), "run_q2r.sh")?;
    refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    Ok(())
}

/// `qe_matdyn_phband`：matdyn.x 声子能带单步 + 归档。
fn run_matdyn_phband(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    let ctx = create_phonon_context(config.as_deref(), input.as_deref(), None, true)?;
    let input_path =
        matdyn_input::write_matdyn_phband_input(&ctx.workspace, &ctx.cfg, &ctx.prefix, ctx.structure.lattice)?;
    let matdyn_exec = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "matdyn.x");
    let slurm_header = ctx.cfg.slurm.header.clone().unwrap_or_default();
    write_root_script(
        &ctx.workspace,
        "run_matdyn_phband.sh",
        &slurm_header,
        &[format!("{matdyn_exec} < matdyn_phband.in > matdyn_phband.out")],
    )?;
    println!("[qe_matdyn_phband] wrote {input_path}, run_matdyn_phband.sh");
    if run {
        qf_runner::run_shell("bash run_matdyn_phband.sh", Path::new(&ctx.workspace))?;
        println!("[qe_matdyn_phband] 归档 phband");
        qf_phonon::collect_phband_outputs(&ctx.workspace, &ctx.prefix)?;
    } else if submit {
        let mut commands = vec!["bash run_matdyn_phband.sh".to_string()];
        commands.extend(qf_phonon::archive::build_collect_phband_commands(&ctx.prefix));
        let script = write_root_script(&ctx.workspace, "submit_matdyn_phband.sh", &slurm_header, &commands)?;
        if let Some(job) = qf_runner::submit_via_sbatch(Path::new(&ctx.workspace), &script_name_of(&script))? {
            println!("[qe_matdyn_phband] submitted Slurm job {job}");
        }
    } else {
        println!("[qe_matdyn_phband] (dry-run) 未指定 -r/--submit");
    }
    refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    Ok(())
}

/// `qe_matdyn_phdos`：matdyn.x 声子 DOS 单步 + 归档。
fn run_matdyn_phdos(
    config: Option<String>,
    input: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    let ctx = create_phonon_context(config.as_deref(), input.as_deref(), None, true)?;
    let input_path = matdyn_input::write_matdyn_phdos_input(&ctx.workspace, &ctx.cfg, &ctx.prefix)?;
    let matdyn_exec = qf_runner::resolve_qe_command(ctx.cfg.qe.executable_path(), "matdyn.x");
    let slurm_header = ctx.cfg.slurm.header.clone().unwrap_or_default();
    write_root_script(
        &ctx.workspace,
        "run_matdyn_phdos.sh",
        &slurm_header,
        &[format!("{matdyn_exec} < matdyn_phdos.in > matdyn_phdos.out")],
    )?;
    println!("[qe_matdyn_phdos] wrote {input_path}, run_matdyn_phdos.sh");
    if run {
        qf_runner::run_shell("bash run_matdyn_phdos.sh", Path::new(&ctx.workspace))?;
        println!("[qe_matdyn_phdos] 归档 phdos");
        qf_phonon::collect_phdos_outputs(&ctx.workspace, &ctx.prefix)?;
    } else if submit {
        let mut commands = vec!["bash run_matdyn_phdos.sh".to_string()];
        commands.extend(qf_phonon::archive::build_collect_phdos_commands(&ctx.prefix));
        let script = write_root_script(&ctx.workspace, "submit_matdyn_phdos.sh", &slurm_header, &commands)?;
        if let Some(job) = qf_runner::submit_via_sbatch(Path::new(&ctx.workspace), &script_name_of(&script))? {
            println!("[qe_matdyn_phdos] submitted Slurm job {job}");
        }
    } else {
        println!("[qe_matdyn_phdos] (dry-run) 未指定 -r/--submit");
    }
    refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    Ok(())
}

fn script_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// `qe_split_submit`：物化 split 输入并逐 q 点提交（跳过 finished）。返回退出码。
fn run_split_submit(
    config: Option<String>,
    input: Option<String>,
    mode: &str,
) -> Result<i32> {
    if mode != "index" && mode != "coords" {
        anyhow::bail!("不支持的 split 模式: {mode}");
    }
    let ctx = create_phonon_context(
        config.as_deref(),
        input.as_deref(),
        Some(&format!("qe_phonon_split_{mode}")),
        true,
    )?;
    let status = refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    if status.ph.dyn0 != TaskState::Finished {
        println!(
            "[qe_split_submit] 3/11 split q 点提交失败: dyn0 未完成: {}",
            status.ph.dyn0.as_str()
        );
        println!(
            "[qe_split_submit] 等待 dyn0 完成后重新执行 qe_phonon_split_{mode} -s"
        );
        print_resume_point(&ctx)?;
        return Ok(1);
    }
    let mode_key = if mode == "index" { "split_assign_index" } else { "split_assign_coords" };
    let entries = qf_phonon::generate_split_inputs(&ctx.workspace, &ctx.cfg, &ctx.prefix, mode_key, &ctx.structure)?;
    println!("[qe_split_submit] generated run_split.sh ({} q 点)", entries.len());
    let status = refresh_workflow_status(Path::new(&ctx.workspace), &ctx.prefix)?;
    let tasks = &status.ph.tasks;
    let mut submitted = Vec::new();
    let mut skipped = Vec::new();
    let mut job_ids = Vec::new();
    for entry in &entries {
        let state = tasks.get(&entry.directory).copied().unwrap_or(TaskState::Unstarted);
        if state == TaskState::Finished {
            skipped.push(entry.directory.clone());
            continue;
        }
        let q_dir = format!("{}/{}", ctx.workspace, entry.directory);
        if let Some(job) = qf_runner::submit_via_sbatch(Path::new(&q_dir), "run_split.sh")? {
            job_ids.push(format!("{}={job}", entry.directory));
        }
        submitted.push(entry.directory.clone());
    }
    let mut parts = Vec::new();
    if !submitted.is_empty() {
        parts.push(format!("提交 {}", submitted.join(", ")));
    }
    if !skipped.is_empty() {
        parts.push(format!("跳过 finished: {}", skipped.join(", ")));
    }
    println!(
        "[qe_split_submit] split q 点: {}",
        if parts.is_empty() { "无待提交任务".to_string() } else { parts.join("; ") }
    );
    if !job_ids.is_empty() {
        println!("[qe_split_submit] split jobs: {}", job_ids.join(", "));
    }
    print_resume_point(&ctx)?;
    Ok(0)
}

/// `qe_check`：输出状态检查器。
#[allow(clippy::too_many_arguments)]
fn run_qe_check(
    config: Option<String>,
    scf: bool,
    relax: bool,
    ph: bool,
    q2r: bool,
    matdyn_band: bool,
    matdyn_dos: bool,
    eph: bool,
) -> Result<()> {
    let selected = [scf, relax, ph, q2r, matdyn_band, matdyn_dos, eph];
    let count = selected.iter().filter(|b| **b).count();
    if count != 1 {
        eprintln!("error: 必须且只能指定一个检查开关。");
        std::process::exit(2);
    }
    if eph || matdyn_band || matdyn_dos {
        println!("暂未支持");
        return Ok(());
    }
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let default_work_dir = "qe_phonon".to_string();
    let work_dir_name = cfg.qe_phonon.work_dir.clone().unwrap_or(default_work_dir);
    let current_dir = std::env::current_dir()?;
    let current_name = current_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let in_place = current_name == work_dir_name
        || WORKSPACE_MARKERS.iter().any(|m| Path::new(m).exists());
    let workspace = if in_place {
        current_dir.to_string_lossy().into_owned()
    } else {
        current_dir.join(&work_dir_name).to_string_lossy().into_owned()
    };
    let root_dir = current_dir.to_string_lossy().into_owned();

    if scf {
        // check_scf：workspace 有 scf.out 否则 root_dir。
        let target = if Path::new(&format!("{workspace}/scf.out")).exists() {
            &workspace
        } else {
            &root_dir
        };
        println!("{}", check_scf_state(target));
        return Ok(());
    }
    if relax {
        println!("{}", check_relax_state(&root_dir));
        return Ok(());
    }

    let prefix = match cfg.qe_phonon.prefix.clone().filter(|p| !p.is_empty()) {
        Some(p) => p,
        None => {
            let base = if Path::new(&workspace).is_dir() { &workspace } else { &root_dir };
            let poscar_path = format!("{base}/POSCAR");
            if Path::new(&poscar_path).exists() {
                qf_crystal::parse_poscar(&poscar_path)
                    .map(|s| s.formula)
                    .unwrap_or_else(|_| "qe".to_string())
            } else {
                "qe".to_string()
            }
        }
    };

    if ph {
        let status = refresh_workflow_status(Path::new(&workspace), &prefix)?;
        println!("{}", status.ph.state.as_str());
        return Ok(());
    }
    if q2r {
        let status = refresh_workflow_status(Path::new(&workspace), &prefix)?;
        println!("{}", status.q2r.as_str());
        return Ok(());
    }
    std::process::exit(1);
}

fn check_scf_state(workdir: &str) -> &'static str {
    let path = format!("{workdir}/scf.out");
    if !Path::new(&path).exists() {
        return "unstarted";
    }
    match qf_io::read_scf_output(&path) {
        Ok(out) => {
            if out.finished {
                "finished"
            } else {
                "error"
            }
        }
        Err(_) => "error",
    }
}

fn check_relax_state(workdir: &str) -> &'static str {
    for candidate in ["relax.out", "vc-relax.out"] {
        let path = format!("{workdir}/{candidate}");
        if Path::new(&path).exists() {
            let text = qf_io::read_text_if_exists(&path);
            let finished = text.contains("JOB DONE") && text.contains("End final coordinates");
            let errored = !text.is_empty()
                && (qf_io::has_error_marker(&text) || text.to_lowercase().contains("bfgs failed"));
            return if finished {
                "finished"
            } else if errored {
                "error"
            } else {
                "error"
            };
        }
    }
    "unstarted"
}

fn copy_save_tree(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let dest = target.join(entry.file_name());
        if entry.file_type()?.is_dir() { copy_save_tree(&entry.path(), &dest)?; }
        else { std::fs::copy(entry.path(), dest)?; }
    }
    Ok(())
}

fn run_qe_pw(kind: &str, config: Option<String>, poscar: Option<String>, run: bool, submit: bool) -> Result<()> {
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let source = poscar.unwrap_or_else(|| "POSCAR".into());
    let structure = qf_crystal::parse_poscar(&source)?;
    let workspace = format!("qe_{kind}");
    std::fs::create_dir_all(&workspace)?;
    let target = Path::new(&workspace).join("POSCAR");
    if std::fs::canonicalize(&source).ok() != std::fs::canonicalize(&target).ok() {
        std::fs::copy(&source, &target)?;
    }
    let outdir = qf_input::pw::write_calculation_input(&workspace, &cfg, &structure, kind)?;
    if kind == "nscf" { copy_scf_save(&workspace, &structure.formula, &outdir, false)?; }
    let command = format!("{} < {kind}.in > {kind}.out", qf_runner::resolve_qe_command(cfg.qe.executable_path(), "pw.x"));
    run_qe_steps(&cfg, &workspace, &structure.formula, run, submit,
        vec![(format!("运行 {kind}"), command)], "run_qe.sh", &workspace)
}
