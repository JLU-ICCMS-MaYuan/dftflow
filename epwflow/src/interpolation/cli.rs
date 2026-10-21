use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use super::manifest::InterpolationManifest;
use super::super::foundation::{config, snapshot, status};

/// interpolation 子命令：阶段边界 check / import（FR-001）。
#[derive(Subcommand)]
pub enum InterpolationCommand {
    /// 只读核查插值清单与来源；不启动后续求解。
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
    /// 校验原生来源，导出叠图诊断数据；不执行科学程序。
    ExportComparison {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

/// 入口。
pub fn run(cmd: InterpolationCommand) -> Result<()> {
    match cmd {
        InterpolationCommand::Check { work, json } => check(&work, json),
        InterpolationCommand::Import { config, source } => import(&config, &source),
        InterpolationCommand::ExportComparison { source, output } => super::comparison::export(&source,&output),
    }
}

fn check(work: &Path, json: bool) -> Result<()> {
    let quality_path = crate::foundation::layout::path(&work, "quality-reference.json");
    if quality_path.is_file() {
        let report = super::native_quality::inspect(&quality_path)?;
        if json {
            status::print_json(&status::envelope("ok", if report.passed { "quality_passed" } else { "quality_warning" }, serde_json::to_value(&report)?));
        } else {
            println!("interpolation_quality: {}", if report.passed { "passed" } else { "failed" });
            println!("electron_max_rms_ev: {} {}", report.electron.max_error_ev, report.electron.rms_error_ev);
            println!("phonon_max_rms_cm1: {} {}", report.phonon.max_error_cm1, report.phonon.rms_error_cm1);
            println!("phonon_limits_max_rms_cm1: {} {}", report.phonon.max_error_limit_cm1, report.phonon.rms_error_limit_cm1);
            println!("workflow_complete: false");
        }
        report.warn_if_failed();
        return Ok(());
    }
    let path = crate::foundation::layout::path(&work, "interpolation").join("manifest.json");
    if !path.exists() {
        if work.join("epw.in").is_file() || work.join("epw.out").is_file() {
            let report = super::native::inspect(work)?;
            if json {
                status::print_json(&status::envelope("ok", "native_epc_only", report));
            } else {
                println!("workflow_status: native_epc_only");
                println!("workflow_complete: false");
                println!("interpolation_quality: unverified");
                println!("eliashberg: unverified");
                println!("lambda_sum: {}", report["lambda_sum"]);
            }
            return Ok(());
        }
        if json {
            status::print_json(&status::envelope(
                "ok",
                "no_data",
                serde_json::json!({"workflow_complete": false, "reason": "interpolation/manifest.json 不存在"}),
            ));
        } else {
            println!("workflow_status: no_data");
        }
        return Ok(());
    }
    let m = InterpolationManifest::load(&path)
        .with_context(|| format!("data_invalid: 无法加载 {}", path.display()))?;
    if json {
        status::print_json(&status::envelope(
            "ok",
            if m.validation.all_passed() { "valid" } else { "invalid" },
            serde_json::json!({
                "run_id": m.run_id,
                "f2_digest": m.f2_digest,
                "mode_readiness": m.mode_readiness,
                "validation_status": m.validation.status,
                "checks": m.validation.checks.iter().map(|c| serde_json::json!({"name": c.name, "passed": c.passed})).collect::<Vec<_>>(),
                "workflow_complete": false,
            }),
        ));
    } else {
        println!("run_id: {}", m.run_id);
        println!("validation_status: {}", m.validation.status);
        for (k, v) in &m.mode_readiness {
            println!("  mode {}: {}", k, v);
        }
    }
    Ok(())
}

fn import(config: &Path, source: &Path) -> Result<()> {
    let cfg = config::Config::load(config)?;
    let _binding = snapshot::prepare(&cfg, config)?;
    let source_work = source
        .parent()
        .and_then(|p| p.parent())
        .context("data_invalid: 来源清单路径须为 <source_work>/interpolation/manifest.json")?;
    let report = super::scheduler::register_import(source_work, &_binding.work_dir)?;
    println!("imported: source_run={} roles={} status={}", report.source_run, report.imported_roles.len(), report.status);
    Ok(())
}
