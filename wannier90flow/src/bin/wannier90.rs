//! wannier90flow 单一二进制入口（P4）。
//!
//! 子命令映射现有 wannier_* CLI：
//! - `wannier90 init`   ← wannier_init（生成 .win + run_wannier90.sh，可选运行/提交）
//! - `wannier90 post`   ← wannier_post（postw90.x 流程）
//! - `wannier90 window` ← wannier_window（能量窗口解析）
//! - `wannier90 plot`   ← wannier90_plot（QE vs Wannier90 能带对比图）

use anyhow::Result;
use clap::{Parser, Subcommand};
use wannier90flow::{init, plot, post, window};

#[derive(Parser)]
#[command(name = "wannier90", version, about = "Rust rewrite of wannier90flow")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 初始化 Wannier90 .win 输入（wannier_init）
    Init {
        /// 结构文件 (VASP POSCAR)
        #[arg(short, long, default_value = "POSCAR")]
        input: String,
        /// 配置文件 (TOML/JSON)
        #[arg(short, long)]
        config: String,
        /// 生成文件后直接执行 Wannier90 流程
        #[arg(short, long)]
        run: bool,
        /// 生成文件后提交 run_wannier90.sh
        #[arg(short, long)]
        submit: bool,
    },
    /// postw90.x 流程（wannier_post）
    Post {
        /// 配置文件路径（TOML/JSON）
        #[arg(short, long, default_value = "inputwannier.toml")]
        config: String,
        /// 输入 .win 文件路径（或不带扩展名的前缀）
        #[arg(short, long)]
        input: String,
        /// 生成脚本后直接执行 postw90.x
        #[arg(short, long)]
        run: bool,
        /// 生成脚本后提交 run_postw90.sh
        #[arg(short, long)]
        submit: bool,
    },
    /// 解析能量窗口（wannier_window）
    Window {
        /// 能量文件：EIGENVAL 或 QE/FLEUR XML（含能带信息）
        xml: String,
        /// 能带序号（1 或 2 个整数）
        #[arg(short, long, num_args = 1..=2, value_parser = clap::value_parser!(usize))]
        n: Option<Vec<usize>>,
        /// 能量范围（1 或 2 个浮点）
        #[arg(short, long, num_args = 1..=2, allow_hyphen_values = true, value_parser = clap::value_parser!(f64))]
        e: Option<Vec<f64>>,
        /// 自动估计能窗：nbnd1 num_wann nbnd3
        #[arg(long, num_args = 3, value_names = ["nbnd1", "num_wann", "nbnd3"])]
        auto: Option<Vec<usize>>,
        /// 自动能窗边界偏移（eV）
        #[arg(long, default_value_t = 0.02)]
        auto_delta: f64,
    },
    /// QE 与 Wannier90 能带对比图（wannier90_plot）
    Plot {
        /// QE 数据目录
        #[arg(long, default_value = ".")]
        qe: String,
        /// Wannier90 数据目录
        #[arg(long, default_value = ".")]
        w90: String,
        /// 对齐用 Fermi 能量 (eV)
        #[arg(long, default_value_t = 23.3313)]
        fermi: f64,
        /// 输出图像文件名
        #[arg(long, default_value = "band_comparison.png")]
        out: String,
    },
}

/// 旧命令名 → 子命令（argv[0] 多调用分派）。
const LEGACY_ENTRY_NAMES: &[(&str, &str)] = &[
    ("wannier_init", "init"),
    ("wannier_post", "post"),
    ("wannier_window", "window"),
    ("wannier_plot", "plot"),
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
        Commands::Init {
            input,
            config,
            run,
            submit,
        } => init::run(&input, &config, run, submit),
        Commands::Post {
            config,
            input,
            run,
            submit,
        } => post::run(&config, &input, run, submit),
        Commands::Window {
            xml,
            n,
            e,
            auto,
            auto_delta,
        } => {
            let mode = if let Some(n) = n {
                window::WindowMode::Bands(n)
            } else if let Some(e) = e {
                window::WindowMode::Energies(e)
            } else if let Some(a) = auto {
                let (nbnd1, num_wann, nbnd3) = (a[0], a[1], a[2]);
                if nbnd1 < 1 || num_wann < 1 {
                    anyhow::bail!("--auto 参数必须满足：nbnd1>=1, num_wann>=1, nbnd3>=0");
                }
                window::WindowMode::Auto {
                    nbnd1,
                    num_wann,
                    nbnd3,
                    delta: auto_delta,
                }
            } else {
                anyhow::bail!("-n / -e / --auto 必须提供其一");
            };
            window::run(&xml, mode)
        }
        Commands::Plot { qe, w90, fermi, out } => plot::run(&qe, &w90, fermi, &out),
    }
}
