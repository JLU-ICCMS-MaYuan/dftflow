//! 独立的 XML → 不可约 q 网格工具，不参与 dftflow 工作流。
mod dyn0;
mod qgrid;
mod xml;

use anyhow::{ensure, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "从 QE SCF XML 独立生成 dyn0 与 q_points.json")]
struct Args {
    #[arg(long)]
    xml: PathBuf,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i32).range(1..))]
    nq1: i32,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i32).range(1..))]
    nq2: i32,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i32).range(1..))]
    nq3: i32,
    #[arg(long)]
    prefix: String,
    #[arg(long, default_value = ".")]
    out_dir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        !args.prefix.is_empty()
            && !args.prefix.contains(['/', '\\'])
            && args.prefix != "."
            && args.prefix != "..",
        "prefix 必须是非空文件名前缀"
    );
    ensure!(
        args.nq1
            .checked_mul(args.nq2)
            .and_then(|n| n.checked_mul(args.nq3))
            .is_some(),
        "q 网格乘积超出 i32 范围"
    );
    let info = xml::parse_qe_save_xml(&args.xml)?;
    let params = qgrid::QGridParams {
        nk1: args.nq1,
        nk2: args.nq2,
        nk3: args.nq3,
        ..Default::default()
    };
    let points = qgrid::generate_q_points(&info, &params);
    std::fs::create_dir_all(&args.out_dir)?;
    dyn0::write_dyn0(
        &args.out_dir.join(format!("{}.dyn0", args.prefix)),
        &points,
        args.nq1,
        args.nq2,
        args.nq3,
    )?;
    dyn0::write_q_points(&args.out_dir.join("q_points.json"), &points, &params)?;
    println!(
        "生成 {} 个不可约 q 点 → {}",
        points.len(),
        args.out_dir.display()
    );
    Ok(())
}
