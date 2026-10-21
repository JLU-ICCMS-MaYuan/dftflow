//! vaspflow 二进制：VASP 各 setup 子命令（生成 INCAR/KPOINTS/POTCAR 并产出运行脚本）。
//! 后处理（vasp_process_*）为后续阶段。

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use qf_config::{Config, SectionMap};
use std::path::Path;

#[derive(Parser)]
#[command(name = "vaspflow", about = "VASP 工作流（Rust 迁移）")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// VASP SCF 输入
    VaspScf {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP SCF converge 输入
    VaspScfConverge {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP 能带输入
    VaspEband {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP 态密度输入
    VaspEledos {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP Fermi 面输入
    VaspFermi {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP LOCPOT 输入
    VaspLocpot {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP 电荷输入
    VaspCharge {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP ELF 输入
    VaspElf {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP COHP uniform 输入
    VaspCohpUniform {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP COHP fatband 输入
    VaspCohpFatband {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'p', long)]
        poscar: Option<String>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP 结构优化输入（rv4/rv1/rvf）
    VaspOpt {
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'i', long)]
        poscar: Option<String>,
        #[arg(short = 'm', long, default_value = "rv4")]
        mode: String,
        #[arg(short = 'p', long, num_args = 1.., allow_hyphen_values = true)]
        pressure: Vec<f64>,
        #[arg(short = 'w', long)]
        work_dir: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// 生成 LOBSTER 输入文件并准备运行目录
    VaspLobsterin {
        #[arg(short = 'e', long, num_args = 2, allow_hyphen_values = true)]
        cohp_energy: Vec<f64>,
        #[arg(long, num_args = 2)]
        species_custom: Vec<String>,
        #[arg(short = 'z', long, num_args = 1..)]
        zvalances: Vec<String>,
        #[arg(short = 'd', long, num_args = 2, allow_hyphen_values = true)]
        d_limit: Vec<f64>,
        #[arg(long)]
        orbital_wise: bool,
        #[arg(long)]
        k_space_cohp: bool,
        #[arg(short = 'm', long, default_value_t = 5)]
        mode: i64,
        #[arg(long, num_args = 1..)]
        custom_basis: Vec<String>,
        #[arg(short, long)]
        config: Option<String>,
        #[arg(short = 'r')]
        run: bool,
        #[arg(short = 's')]
        submit: bool,
    },
    /// VASP 能带后处理（读 BAND.dat/PBAND_*.dat）
    VaspProcessEband {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(short = 'e', long, num_args = 1..)]
        elements: Vec<String>,
        #[arg(long)]
        fermi: Option<f64>,
        #[arg(long, default_value = "pband_merged")]
        out_prefix: String,
    },
    /// VASP DOS 后处理（读 TDOS.dat/PDOS_*.dat）
    VaspProcessEledos {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(short = 'e', long, num_args = 1..)]
        elements: Vec<String>,
        #[arg(long)]
        per_volume: bool,
        #[arg(long, default_value = "pdos_merged")]
        out_prefix: String,
    },
    /// VASP COHP 后处理（读 COHPCAR.lobster；对应 vasp_plot_cohp）
    VaspProcessCohp {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(short = 'f', long, default_value = "COHPCAR.lobster")]
        file: String,
        #[arg(long = "interaction", num_args = 1.., alias = "i")]
        interactions: Vec<String>,
        /// ICOHP 能窗分析的下限
        #[arg(short = 'l', long = "lower_energy", alias = "low")]
        lower_energy: Option<f64>,
        /// ICOHP 能窗分析的上限
        #[arg(short = 'u', long = "upper_energy", alias = "up")]
        upper_energy: Option<f64>,
        #[arg(long)]
        emin: Option<f64>,
        #[arg(long)]
        emax: Option<f64>,
        #[arg(long = "out-prefix")]
        out_prefix: Option<String>,
    },
    /// Band-resolved pCOHP（读 KspaceCOHPBand*.lobster；对应 vasp_plot_kcohp）
    VaspPlotKCohp {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(long = "interaction", num_args = 1.., alias = "i")]
        interactions: Vec<String>,
        #[arg(long)]
        fermi: Option<f64>,
        #[arg(long)]
        emin: Option<f64>,
        #[arg(long)]
        emax: Option<f64>,
        #[arg(long, default_value_t = 1.0)]
        vmax: f64,
        #[arg(short = 'o', long = "output-prefix")]
        output_prefix: Option<String>,
    },
    /// VASP 电荷密度差分 (A - B)
    VaspChargeDiff {
        /// 电荷密度 A 文件路径 (PARCHG/CHGCAR)
        a: String,
        /// 电荷密度 B 文件路径 (PARCHG/CHGCAR)
        b: String,
        #[arg(short = 'o', long, default_value = "A_minus_B.vasp")]
        output: String,
    },
    /// 指定能量区间内的 VASP TDOS/PDOS IDOS（对应 vasp_idos_computed）
    VaspIdosComputed {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(short = 'e', long, num_args = 1..)]
        elements: Vec<String>,
        #[arg(long)]
        low: f64,
        #[arg(long)]
        up: f64,
        #[arg(long, default_value = "../vasp_scf/OUTCAR")]
        scf_outcar: String,
        #[arg(long, default_value = "DOSCAR")]
        doscar: String,
        #[arg(long)]
        per_volume: bool,
        #[arg(long)]
        skip_vaspkit: bool,
        #[arg(long, default_value = "idos_computed")]
        out_prefix: String,
    },
    /// VASP LOCPOT 平面平均后处理（读 LOCPOT 体积场）
    VaspProcessLocpot {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(short = 'f', long, default_value = "LOCPOT")]
        file: String,
        #[arg(long, default_value_t = 2)]
        axis: usize,
        #[arg(long, default_value = "locpot")]
        out_prefix: String,
    },
    /// VASP CHGCAR 平面平均后处理（读 CHGCAR 体积场）
    VaspProcessCharge {
        #[arg(short = 'd', long, default_value = ".")]
        directory: String,
        #[arg(short = 'f', long, default_value = "CHGCAR")]
        file: String,
        #[arg(long, default_value_t = 2)]
        axis: usize,
        #[arg(long, default_value = "charge")]
        out_prefix: String,
    },
}

fn user_section<'a>(cfg: &'a Config, name: &str) -> &'a SectionMap {
    match name {
        "vasp_scf" => &cfg.vasp_scf,
        "vasp_scf_converge" => &cfg.vasp_scf_converge,
        "vasp_eband" => &cfg.vasp_eband,
        "vasp_eledos" => &cfg.vasp_eledos,
        "vasp_fermi" => &cfg.vasp_fermi,
        "vasp_locpot" => &cfg.vasp_locpot,
        "vasp_charge" => &cfg.vasp_charge,
        "vasp_elf" => &cfg.vasp_elf,
        "vasp_cohp_uniform" => &cfg.vasp_cohp_uniform,
        "vasp_cohp_fatband" => &cfg.vasp_cohp_fatband,
        "vasp_opt" => &cfg.vasp_opt,
        _ => &cfg.vasp_scf,
    }
}

/// 需要从 `chgcar_path`/`wavecar_path` 拷贝 SCF 产物的续算流程（对齐 Python：
/// `cohp_common.copy_scf_files` 及 eband/eledos/fermisurface/locpot/charge/elf 各自实现）。
const COPY_SCF_NAMES: [&str; 8] = [
    "vasp_eband",
    "vasp_eledos",
    "vasp_fermi",
    "vasp_locpot",
    "vasp_charge",
    "vasp_elf",
    "vasp_cohp_uniform",
    "vasp_cohp_fatband",
];

/// 把根配置 `chgcar_path`/`wavecar_path` 指向的文件拷入工作目录（ICHARG=11/ISTART=1 需要）。
/// 对齐 Python `shutil.copy2 + expanduser`：源缺失仅打印警告，不中断。
fn copy_scf_files(cfg: &qf_config::Config, out_dir: &str) {
    for (src, dest_name) in [
        (cfg.vasp.chgcar_path(), "CHGCAR"),
        (cfg.vasp.wavecar_path(), "WAVECAR"),
    ] {
        if src.is_empty() {
            continue;
        }
        let expanded = if let Some(rest) = src.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                Path::new(&home).join(rest)
            } else {
                Path::new(src).to_path_buf()
            }
        } else {
            Path::new(src).to_path_buf()
        };
        if expanded.exists() {
            let dest = format!("{out_dir}/{dest_name}");
            match std::fs::copy(&expanded, &dest) {
                Ok(_) => println!("[{out_dir}] 已从 {src} 拷贝 {dest_name}"),
                Err(e) => println!("[{out_dir}] 警告: 拷贝 {dest_name} 失败: {e}"),
            }
        } else {
            println!("[{out_dir}] 警告: 找不到 {dest_name} 源文件 {src}");
        }
    }
}

fn run_setup(
    name: &str,
    config: Option<String>,
    poscar: Option<String>,
    work_dir: Option<String>,
    run: bool,
    submit: bool,
) -> Result<()> {
    let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
    let poscar_path = poscar.unwrap_or_else(|| "POSCAR".to_string());
    let out_dir = work_dir.unwrap_or_else(|| name.to_string());
    let potcar_dir = cfg.vasp.potcar_dir();
    let (incar, kpoints, potcar_path_out) =
        df_vasp::setup::build_vasp_inputs(name, user_section(&cfg, name), &poscar_path, potcar_dir, &out_dir)?;
    println!("[{name}] wrote {incar}, {kpoints}");
    if !potcar_path_out.is_empty() {
        println!("[{name}] wrote {potcar_path_out}");
    }
    if COPY_SCF_NAMES.contains(&name) {
        copy_scf_files(&cfg, &out_dir);
    }

    let vasp_exec = if cfg.vasp.executable_path().is_empty() {
        "vasp_std".to_string()
    } else {
        qf_runner::plain_scientific_command(cfg.vasp.executable_path())
    };
    let steps = vec![(
        format!("运行 {name}"),
        format!("cd {out_dir} && {vasp_exec} > vasp.log && cd .."),
    )];
    let slurm_header = cfg.slurm.header.as_deref().unwrap_or("");
    let script = qf_runner::ScriptContext {
        workspace: Path::new("."),
        prefix: name,
        slurm_header,
    };
    qf_runner::write_labeled_script(
        &script,
        &format!("run_{name}.sh"),
        &steps,
        1,
        steps.len(),
        None,
        Some(&format!("{name} 完成")),
    )
    .with_context(|| "write run script")?;
    qf_runner::submit_or_run(run, submit, Path::new("."), &format!("run_{name}.sh"))?;
    Ok(())
}

/// 旧命令名 → 子命令（argv[0] 多调用分派，配合 install-compat.sh 的硬链接）。
const LEGACY_ENTRY_NAMES: &[(&str, &str)] = &[
    ("vasp_scf", "vasp-scf"),
    ("vasp_scf_converge", "vasp-scf-converge"),
    ("vasp_eband", "vasp-eband"),
    ("vasp_process_eband", "vasp-process-eband"),
    ("vasp_eledos", "vasp-eledos"),
    ("vasp_process_eledos", "vasp-process-eledos"),
    ("vasp_idos_computed", "vasp-idos-computed"),
    ("vasp_fermi", "vasp-fermi"),
    ("vasp_cohp_uniform", "vasp-cohp-uniform"),
    ("vasp_cohp_fatband", "vasp-cohp-fatband"),
    ("vasp_lobsterin", "vasp-lobsterin"),
    ("vasp_plot_cohp", "vasp-process-cohp"),
    ("vasp_plot_kcohp", "vasp-plot-k-cohp"),
    ("vasp_locpot", "vasp-locpot"),
    ("vasp_process_locpot", "vasp-process-locpot"),
    ("vasp_charge", "vasp-process-charge"),
    ("vasp_charge_diff", "vasp-charge-diff"),
    ("vasp_opt", "vasp-opt"),
    ("vasp_elf", "vasp-elf"),
];

fn main() -> Result<()> {
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
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
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::VaspScf { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_scf", config, poscar, work_dir, run, submit)
        }
        Commands::VaspScfConverge { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_scf_converge", config, poscar, work_dir, run, submit)
        }
        Commands::VaspEband { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_eband", config, poscar, work_dir, run, submit)
        }
        Commands::VaspEledos { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_eledos", config, poscar, work_dir, run, submit)
        }
        Commands::VaspFermi { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_fermi", config, poscar, work_dir, run, submit)
        }
        Commands::VaspLocpot { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_locpot", config, poscar, work_dir, run, submit)
        }
        Commands::VaspCharge { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_charge", config, poscar, work_dir, run, submit)
        }
        Commands::VaspElf { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_elf", config, poscar, work_dir, run, submit)
        }
        Commands::VaspCohpUniform { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_cohp_uniform", config, poscar, work_dir, run, submit)
        }
        Commands::VaspCohpFatband { config, poscar, work_dir, run, submit } => {
            run_setup("vasp_cohp_fatband", config, poscar, work_dir, run, submit)
        }
        Commands::VaspOpt { config, poscar, mode, pressure, work_dir, run, submit } => {
            let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let vasp_exec = if cfg.vasp.executable_path().is_empty() {
                "vasp_std".to_string()
            } else {
                qf_runner::plain_scientific_command(cfg.vasp.executable_path())
            };
            let potcar_dir = cfg.vasp.potcar_dir().to_string();
            let slurm_header = cfg.slurm.header.as_deref().unwrap_or("").to_string();
            let run_opt_one = |pressure_gpa: Option<f64>, wd: &str| -> Result<String> {
                let args = df_vasp::opt::VaspOptArgs {
                    work_dir: wd.to_string(),
                    struct_file: poscar.clone(),
                    mode: mode.clone(),
                    pressure_gpa,
                    vasp_execmd: vasp_exec.clone(),
                    potcar_dir: potcar_dir.clone(),
                    slurm_header: slurm_header.clone(),
                };
                df_vasp::opt::run_vasp_opt(&cfg.vasp_opt, &cfg.opt_stage_templates, &args)
            };
            if pressure.is_empty() {
                let wd = work_dir.clone().unwrap_or_else(|| "vasp_opt".to_string());
                let script = run_opt_one(None, &wd)?;
                qf_runner::submit_or_run(run, submit, Path::new(&wd), &script)?;
            } else if pressure.len() == 1 {
                let p = pressure[0];
                let wd = format!("{}/vasp_opt", p);
                let script = run_opt_one(Some(p), &wd)?;
                qf_runner::submit_or_run(run, submit, Path::new(&wd), &script)?;
            } else {
                let mut cases: Vec<(String, Vec<String>)> = Vec::new();
                for &p in &pressure {
                    let wd = format!("{}/vasp_opt", p);
                    run_opt_one(Some(p), &wd)?;
                    let abs = std::fs::canonicalize(&wd)?.to_string_lossy().into_owned();
                    cases.push((abs, df_vasp::opt::build_mode_commands(&mode, &vasp_exec)));
                }
                let batch = df_vasp::opt::write_batch_script("submit_all_vasp_opt.sh", &slurm_header, &cases)?;
                qf_runner::submit_or_run(run, submit, Path::new("."), &batch)?;
            }
            Ok(())
        }
        Commands::VaspLobsterin { cohp_energy, species_custom, zvalances, d_limit, orbital_wise, k_space_cohp, mode, custom_basis, config, run, submit } => {
            if species_custom.len() != 2 || d_limit.len() != 2 || cohp_energy.len() != 2 {
                anyhow::bail!("--species_custom/-d/-e 均需提供 2 个值");
            }
            let cfg = qf_config::load(config.as_deref(), config.as_deref())?;
            let lobster_exec = if cfg.vasp.lobster_executable_path().is_empty() {
                String::new()
            } else {
                qf_runner::plain_scientific_command(cfg.vasp.lobster_executable_path())
            };
            let slurm_header = cfg.slurm.header.as_deref().unwrap_or("").to_string();
            let args = df_vasp::lobsterin::LobsterinArgs {
                cohp_start: cohp_energy[0],
                cohp_end: cohp_energy[1],
                species_custom: [species_custom[0].clone(), species_custom[1].clone()],
                zvalances,
                d_limit: [d_limit[0], d_limit[1]],
                orbital_wise,
                k_space_cohp,
                mode,
                custom_basis,
                lobster_execmd: lobster_exec,
                slurm_header,
            };
            let wd = df_vasp::lobsterin::run_lobsterin(&args)?;
            qf_runner::submit_or_run(run, submit, Path::new(&wd), "run_lobster.sh")?;
            Ok(())
        }
        Commands::VaspProcessEband { directory, elements, fermi, out_prefix } => {
            let specs = df_vasp::process::parse_element_specs(&elements)?;
            let (csv, png) = df_vasp::process::process_eband(&directory, &specs, fermi, &out_prefix)?;
            println!("已生成: {}", csv.display());
            println!("已生成: {}", png.display());
            Ok(())
        }
        Commands::VaspProcessEledos { directory, elements, per_volume, out_prefix } => {
            let specs = df_vasp::process::parse_element_specs(&elements)?;
            let (csv, at_ef, png) =
                df_vasp::process::process_eledos(&directory, &specs, per_volume, &out_prefix)?;
            println!("已生成: {}", csv.display());
            println!("已生成: {}", at_ef.display());
            println!("已生成: {}", png.display());
            Ok(())
        }
        Commands::VaspProcessCohp { directory, file, interactions, lower_energy, upper_energy, emin, emax, out_prefix } => {
            let (avg, avg_png, proj, proj_png) = df_vasp::process::process_cohp(
                &directory,
                &file,
                &interactions,
                lower_energy,
                upper_energy,
                emin,
                emax,
                out_prefix.as_deref(),
            )?;
            println!("已生成: {}", proj.display());
            println!("已生成: {}", avg.display());
            println!("已生成: {}", proj_png.display());
            println!("已生成: {}", avg_png.display());
            Ok(())
        }
        Commands::VaspPlotKCohp { directory, interactions, fermi, emin, emax, vmax, output_prefix } => {
            let (dat, png) = df_vasp::process::plot_kcohp(
                &directory,
                &interactions,
                fermi,
                emin,
                emax,
                vmax,
                output_prefix.as_deref(),
            )?;
            println!("数据已保存至: {}", dat.display());
            println!("绘图已保存为: {}", png.display());
            Ok(())
        }
        Commands::VaspChargeDiff { a, b, output } => {
            let path = df_vasp::process::charge_diff(&a, &b, &output)?;
            println!("已生成差分电荷密度: {path}");
            Ok(())
        }
        Commands::VaspIdosComputed { directory, elements, low, up, scf_outcar, doscar, per_volume, skip_vaspkit, out_prefix } => {
            df_vasp::process::idos_computed(
                &directory,
                &elements,
                low,
                up,
                &scf_outcar,
                &doscar,
                per_volume,
                skip_vaspkit,
                &out_prefix,
            )?;
            Ok(())
        }
        Commands::VaspProcessLocpot { directory, file, axis, out_prefix } => {
            let (dat, png) =
                df_vasp::process::process_locpot(&directory, &file, axis, &out_prefix)?;
            println!("已生成: {}", dat.display());
            println!("已生成: {}", png.display());
            Ok(())
        }
        Commands::VaspProcessCharge { directory, file, axis, out_prefix } => {
            let (dat, png) =
                df_vasp::process::process_charge(&directory, &file, axis, &out_prefix)?;
            println!("已生成: {}", dat.display());
            println!("已生成: {}", png.display());
            Ok(())
        }
    }
}
