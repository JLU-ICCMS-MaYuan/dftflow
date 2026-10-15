use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use super::manifest::QeManifest;
use super::super::foundation::{config, snapshot, status};

/// qe-data 子命令：独立验收 QE 数据准备阶段边界（FR-001）。
#[derive(Subcommand)]
pub enum QeDataCommand {
    /// 只读核查本地数据、来源与清单；不启动 EPW 读取测试。
    Check {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 校验来源清单，准备隔离副本并登记可复用阶段；不提交计算。
    Import {
        #[arg(short = 'c', long)]
        config: PathBuf,
        #[arg(long)]
        source: PathBuf,
    },
}

/// 入口：由 foundation cli 分派至此。
pub fn run(cmd: QeDataCommand) -> Result<()> {
    match cmd {
        QeDataCommand::Check { work, json } => check(&work, json),
        QeDataCommand::Import { config, source } => import(&config, &source),
    }
}

/// 只读核查 qe-data 清单与来源；缺失则明确报告未知/未完成。
fn check(work: &Path, json: bool) -> Result<()> {
    let manifest_path = work.join("qe-data").join("manifest.json");
    if !manifest_path.exists() {
        if json {
            let v = status::envelope(
                "ok",
                "no_data",
                serde_json::json!({"workflow_complete": false, "reason": "qe-data/manifest.json 不存在"}),
            );
            status::print_json(&v);
        } else {
            println!("workflow_status: no_data");
            println!("reason: qe-data/manifest.json 不存在");
        }
        return Ok(());
    }
    let m: QeManifest = QeManifest::load(&manifest_path)
        .with_context(|| format!("data_invalid: 无法加载 {}", manifest_path.display()))?;
    if json {
        let v = status::envelope(
            "ok",
            if m.validation.all_passed() { "valid" } else { "invalid" },
            serde_json::json!({
                "run_id": m.run_id,
                "schema_version": m.schema_version,
                "n_qpoints": m.qpoints.len(),
                "n_phonon_files": m.phonon_files.len(),
                "validation_status": m.validation.status,
                "checks": m.validation.checks.iter().map(|c| serde_json::json!({"name": c.name, "passed": c.passed})).collect::<Vec<_>>(),
                "workflow_complete": false,
            }),
        );
        status::print_json(&v);
    } else {
        println!("run_id: {}", m.run_id);
        println!("validation_status: {}", m.validation.status);
        println!("n_qpoints: {}", m.qpoints.len());
        println!("n_phonon_files: {}", m.phonon_files.len());
        for c in &m.validation.checks {
            println!("  check {}: {}", c.name, if c.passed { "pass" } else { "fail" });
        }
    }
    Ok(())
}

/// 导入来源清单登记的隔离副本；不提交计算（FR-009/001）。
fn import(config: &Path, source: &Path) -> Result<()> {
    let cfg = config::Config::load(config)?;
    let binding = snapshot::prepare(&cfg, config)?;
    // 来源清单必须为 <source_work>/qe-data/manifest.json。
    let source_work = source
        .parent()
        .and_then(|p| p.parent())
        .context("data_invalid: 来源清单路径须为 <source_work>/qe-data/manifest.json")?;
    let report = super::scheduler::register_import(source_work, &binding.work_dir)?;
    println!(
        "imported: source_run={} roles={} status={}",
        report.source_run,
        report.imported_roles.len(),
        report.status
    );
    Ok(())
}
