//! `qf-gen-dyn0` —— 生成 `prefix.dyn0` 与 `q_points.json`（替代“跑 ph.x 仅出 dyn0 再杀掉”）。
//!
//! 两种用法（对齐 `qeflow/phonon/ph_step.build_qf_gen_dyn0_command` 与旧 CLI）：
//! ```text
//! qf-gen-dyn0 --xml <outdir>/<prefix>.save/data-file-schema.xml --nq1 N --nq2 N --nq3 N --prefix P --out-dir W
//! qf-gen-dyn0 -c input.toml --work-dir ./phonon
//! ```
//! XML 路径按 SCF save 的对称信息枚举不可约 q 网格（复刻 `PHonon/PH/q_points.f90`），
//! 并以晶体坐标写出 `q_points.json` 作为 split 步骤的规范来源。

use clap::Parser;
use std::path::Path;

#[derive(Parser)]
#[command(about = "生成 prefix.dyn0 与 q_points.json（split 工作流的 dyn0 预处理）")]
struct Cli {
    /// SCF save 的 data-file-schema.xml；提供时按对称性枚举不可约 q 点。
    #[arg(long)]
    xml: Option<String>,
    #[arg(long, default_value_t = 1)]
    nq1: i32,
    #[arg(long, default_value_t = 1)]
    nq2: i32,
    #[arg(long, default_value_t = 1)]
    nq3: i32,
    #[arg(long)]
    prefix: Option<String>,
    /// 输出目录（workspace）。
    #[arg(long, default_value = ".")]
    out_dir: String,
    /// 旧接口：从 input.toml 读取 q 网格（与 --xml 互斥，--xml 优先）。
    #[arg(short, long, default_value = "input.toml")]
    config: Option<String>,
    /// 旧接口：工作目录（--out-dir 的别名）。
    #[arg(long, default_value = ".")]
    work_dir: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let work_dir = if cli.out_dir != "." { cli.out_dir.clone() } else { cli.work_dir.clone() };
    std::fs::create_dir_all(&work_dir)?;

    if let Some(xml) = cli.xml.as_deref() {
        // 新接口：对齐 ph_step.build_qf_gen_dyn0_command 的调用契约。
        let prefix = cli
            .prefix
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--xml 模式必须提供 --prefix"))?;
        let info = qf_phonon::xml::parse_qe_save_xml(Path::new(xml))?;
        let params = qf_phonon::qgrid::QGridParams {
            nk1: cli.nq1,
            nk2: cli.nq2,
            nk3: cli.nq3,
            ..Default::default()
        };
        let qpoints = qf_phonon::qgrid::generate_q_points(&info, &params);
        let dyn0_path = Path::new(&work_dir).join(format!("{prefix}.dyn0"));
        qf_phonon::dyn0::write_dyn0(&dyn0_path, &qpoints, cli.nq1, cli.nq2, cli.nq3)?;
        let qp_path = Path::new(&work_dir).join("q_points.json");
        qf_phonon::dyn0::write_q_points(&qp_path, &qpoints, &params)?;
        println!("生成 dyn0 -> {}", dyn0_path.display());
        println!("生成 q_points.json -> {}", qp_path.display());
        if dyn0_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Ok(());
        }
        anyhow::bail!("dyn0 文件为空");
    }

    // 旧接口：从 input.toml 的 [qe_phonon.ph] qpoints 枚举全网格。
    let cfg = qf_config::load(cli.config.as_deref(), cli.config.as_deref())?;
    let prefix = cli
        .prefix
        .clone()
        .or_else(|| cfg.qe_phonon.prefix.clone())
        .or_else(|| cfg.qe_phonon.scf.get("prefix").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "struct".to_string());
    let path = qf_phonon::split::write_dyn0(&work_dir, &cfg, &prefix)?;
    println!("生成 dyn0 -> {path}");
    Ok(())
}
