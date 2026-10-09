//! `qf-gen-dyn0` —— 从 SCF save 的 `data-file-schema.xml` 直接计算不可约 q 点网格，
//! 写出与 `ph.x` 字节一致的 `prefix.dyn0` 以及拆分用的 `q_index_map.json`。
//!
//! 用法：
//! ```text
//! qf-gen-dyn0 --xml /path/to/data-file-schema.xml --nq1 2 --nq2 2 --nq3 1 \
//!              --prefix Mg1B2 --out-dir ./phonon
//! ```
//! 这一步替代了原先“运行 ph.x 仅生成 dyn0 再杀掉”的尴尬环节。

use clap::Parser;
use qf_phonon::{
    generate_dyn0, QGridParams,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "从 SCF save XML 生成与 ph.x 一致的 dyn0 与 q_index_map.json")]
struct Cli {
    /// SCF save 目录下的 data-file-schema.xml 路径。
    #[arg(long)]
    xml: PathBuf,
    #[arg(long)]
    nq1: i32,
    #[arg(long)]
    nq2: i32,
    #[arg(long)]
    nq3: i32,
    /// Monkhorst-Pack 半偏移（默认 0）。
    #[arg(long, default_value = "0")]
    k1: i32,
    #[arg(long, default_value = "0")]
    k2: i32,
    #[arg(long, default_value = "0")]
    k3: i32,
    #[arg(long, default_value = "prefix")]
    prefix: String,
    #[arg(long, default_value = ".")]
    out_dir: PathBuf,
    /// 覆盖 time_reversal（缺省使用 XML 推导值，非磁性默认 true）。
    #[arg(long)]
    time_reversal: Option<bool>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let params = QGridParams {
        nk1: cli.nq1,
        nk2: cli.nq2,
        nk3: cli.nq3,
        k1: cli.k1,
        k2: cli.k2,
        k3: cli.k3,
        time_reversal: cli.time_reversal,
    };
    let qpoints = generate_dyn0(&cli.xml, &cli.out_dir, &cli.prefix, &params)?;
    println!(
        "生成 {} 个不可约 q 点 -> {}/{}.dyn0, q_points.json",
        qpoints.len(),
        cli.out_dir.display(),
        cli.prefix
    );
    for q in &qpoints {
        println!(
            "  #{:<3} crystal=({:.6}, {:.6}, {:.6})  cart=({:.8}, {:.8}, {:.8})  w={:.6}",
            q.index,
            q.q_crystal[0],
            q.q_crystal[1],
            q.q_crystal[2],
            q.q_cart[0],
            q.q_cart[1],
            q.q_cart[2],
            q.weight
        );
    }
    Ok(())
}
