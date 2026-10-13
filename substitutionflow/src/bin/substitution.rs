//! `substitution` 单一静态二进制入口（对应 `substitutionflow` 的 7 个 CLI）。
//!
//! 兼容入口（argv[0]）：
//! - `substitution_structure`      → structure
//! - `substitution`                → workflow（直接 `-c/-o/-r/-s`，与 Python 一致）
//! - `substitution_continue`       → continue
//! - `substitution_prepare_potcar` → prepare-potcar
//! - `substitution_prepare_qe_pseudo` → prepare-qe-pseudo
//! - `substitution_plot_eledos`    → plot-eledos
//! - `substitution_check_phonon_stability` → check-phonon-stability

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::ffi::OsString;

#[derive(Parser)]
#[command(
    name = "substitution",
    version,
    about = "XA/XB 占位结构高通量替换工作流（Rust 重写）"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 准备 XA/XB 占位结构的高通量工作流输入目录（substitution_structure）
    Structure {
        #[arg(short, long, default_value = "input.toml")]
        config: String,
        #[arg(short = 'o', long = "output-dir")]
        output_dir: String,
    },
    /// XA/XB 占位结构高通量替换工作流（substitution）
    Workflow {
        #[arg(short, long, default_value = "input.toml")]
        config: String,
        #[arg(short = 'o', long = "output-dir")]
        output_dir: String,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long, conflicts_with = "run")]
        submit: bool,
    },
    /// 检查并续算 substitutionflow 工作流（substitution_continue）
    Continue {
        #[arg(short = 'o', long = "output-dir")]
        output_dir: String,
        #[arg(short, long)]
        config: String,
        #[arg(long = "refresh-status", alias = "rfs")]
        refresh_status: bool,
        #[arg(long = "rewrite-states", alias = "rws")]
        rewrite_states: bool,
        #[arg(short = 'r', long)]
        run: bool,
        #[arg(short = 's', long, conflicts_with = "run")]
        submit: bool,
    },
    /// 为 substitutionflow 准备本地 potcar_lib
    PreparePotcar {
        #[arg(short, long, default_value = "input.toml")]
        config: String,
        #[arg(long)]
        pot: String,
    },
    /// 为 substitutionflow 准备本地 qe_pseudo_lib
    PrepareQePseudo {
        #[arg(short, long, default_value = "input.toml")]
        config: String,
        #[arg(long = "pp", alias = "pseudo-dir")]
        pseudo_dir: String,
    },
    /// 批量 DOS 结果周期表总览图
    PlotEledos {
        #[arg(short = 'd', long, num_args = 0.., default_value = "")]
        directory: Vec<String>,
    },
    /// 批量声子稳定性检查与总览图
    CheckPhononStability {
        #[arg(short = 'd', long, num_args = 0.., default_value = "")]
        directory: Vec<String>,
        #[arg(short = 'j', long)]
        jobs: Option<usize>,
    },
}

/// 单横线长选项兼容（Python argparse 的 `-rfs`/`-rws`）。
fn normalize_args(raw: Vec<OsString>) -> Vec<OsString> {
    raw.into_iter()
        .map(|a| {
            let s = a.to_string_lossy().into_owned();
            match s.as_str() {
                "-rfs" => OsString::from("--rfs"),
                "-rws" => OsString::from("--rws"),
                _ => a,
            }
        })
        .collect()
}

fn main() -> Result<()> {
    let mut args: Vec<OsString> = std::env::args_os().collect();
    let entry = args
        .first()
        .map(|a| {
            Path::new(a)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| a.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let known = [
        "structure",
        "workflow",
        "continue",
        "prepare-potcar",
        "prepare-qe-pseudo",
        "plot-eledos",
        "check-phonon-stability",
    ];
    // 旧入口名 → 子命令；`substitution` 无子命令参数时视为 workflow。
    match entry.as_str() {
        "substitution_structure" => args.insert(1, OsString::from("structure")),
        "substitution_continue" => args.insert(1, OsString::from("continue")),
        "substitution_prepare_potcar" => args.insert(1, OsString::from("prepare-potcar")),
        "substitution_prepare_qe_pseudo" => args.insert(1, OsString::from("prepare-qe-pseudo")),
        "substitution_plot_eledos" => args.insert(1, OsString::from("plot-eledos")),
        "substitution_check_phonon_stability" => {
            args.insert(1, OsString::from("check-phonon-stability"))
        }
        "substitution" => {
            let is_subcommand = args
                .get(1)
                .map(|a| known.contains(&a.to_string_lossy().as_ref()))
                .unwrap_or(false);
            if !is_subcommand {
                args.insert(1, OsString::from("workflow"));
            }
        }
        _ => {}
    }

    let cli = Cli::parse_from(normalize_args(args));
    match cli.command {
        Commands::Structure { config, output_dir } => {
            let cfg = df_substitution::structure::load_substitutionflow_config(&config)?;
            let ht = cfg.get("substitutionflow").cloned().unwrap_or(Value::Null);
            let preview = ht
                .get("preview")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if preview {
                let xa = ht.get("XA").and_then(|v| v.as_str()).unwrap_or_default();
                let xb = ht.get("XB").and_then(|v| v.as_str()).unwrap_or_default();
                println!("{}", df_substitution::selection::render_selection_preview("XA", xa)?);
                println!();
                println!("{}", df_substitution::selection::render_selection_preview("XB", xb)?);
                println!();
            }
            let input_path = df_substitution::structure::resolve_input_structure_path(&config)?;
            let result = df_substitution::structure::generate_cases(&input_path, &config, &output_dir)?;
            df_substitution::status::create_initial_status(&output_dir)?;
            let abs = std::fs::canonicalize(&output_dir)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| output_dir.clone());
            println!("已生成 {} 个替换结构目录于 {abs}", result.cases.len());
            Ok(())
        }
        Commands::Workflow { config, output_dir, run, submit } => {
            let result =
                df_substitution::workflow::run_workflow(&config, &output_dir, run, submit)?;
            println!("{result}");
            Ok(())
        }
        Commands::Continue { output_dir, config, refresh_status, rewrite_states, run, submit } => {
            if refresh_status && rewrite_states {
                eprintln!("error: -rfs/--refresh-status 不能与 -rws/--rewrite-states 同时使用。");
                std::process::exit(2);
            }
            let result = if rewrite_states && !run && !submit {
                df_substitution::workflow::rewrite_workflow_states(&output_dir, &config)?
            } else {
                df_substitution::workflow::continue_workflow(
                    &output_dir,
                    run,
                    submit,
                    &config,
                    rewrite_states,
                )?
            };
            println!("{result}");
            Ok(())
        }
        Commands::PreparePotcar { config, pot } => {
            df_substitution::prepare::prepare_potcar_library(&config, &pot)?;
            Ok(())
        }
        Commands::PrepareQePseudo { config, pseudo_dir } => {
            df_substitution::prepare::prepare_qe_pseudo_library(&config, &pseudo_dir)?;
            Ok(())
        }
        Commands::PlotEledos { directory } => {
            let dirs: Vec<String> = directory.into_iter().filter(|s| !s.is_empty()).collect();
            let outputs = df_substitution::plot_eledos::run(&dirs)?;
            for path in outputs {
                println!("{path}");
            }
            Ok(())
        }
        Commands::CheckPhononStability { directory, jobs } => {
            let dirs: Vec<String> = directory.into_iter().filter(|s| !s.is_empty()).collect();
            let outputs = df_substitution::check_phonon::run(&dirs, jobs)?;
            for path in outputs {
                println!("{path}");
            }
            Ok(())
        }
    }
}

use std::path::Path;
