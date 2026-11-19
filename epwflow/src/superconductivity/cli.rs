use super::super::foundation::status;
use super::manifest::SuperconductivityManifest;
use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};

/// superconductivity 子命令：阶段边界 check / import（FR-001）。
#[derive(Subcommand)]
pub enum SuperconductivityCommand {
    /// 只读核查超导清单与模式分离；不启动后续求解。
    #[command(about = "Validate the superconductivity manifest and mode separation read-only without starting downstream solvers", long_about = "Validate the superconductivity manifest and mode separation read-only without starting downstream solvers")]
    Check {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 校验前置超导清单，登记可复用阶段；不提交计算。
    #[command(about = "Validate the preceding superconductivity manifest and register reusable stages without submitting calculations", long_about = "Validate the preceding superconductivity manifest and register reusable stages without submitting calculations")]
    Import {
        #[arg(long)]
        source: PathBuf,
    },
}

/// 入口。
pub fn run(cmd: SuperconductivityCommand) -> Result<()> {
    match cmd {
        SuperconductivityCommand::Check { work, json } => check(&work, json),
        SuperconductivityCommand::Import { source } => import(&source),
    }
}

fn check(work: &Path, json: bool) -> Result<()> {
    let path = crate::foundation::layout::path(&work, "superconductivity").join("manifest.json");
    if !path.exists() {
        if json {
            status::print_json(&status::envelope(
                "ok",
                "no_data",
                serde_json::json!({"workflow_complete": false, "reason": "superconductivity/manifest.json does not exist"}),
            ));
        } else {
            println!("workflow_status: no_data");
        }
        return Ok(());
    }
    if crate::foundation::store::read::<serde_json::Value>(&path)?
        .get("task")
        .and_then(|v| v.as_str())
        == Some("gap")
    {
        let r = super::points::Run::load(&path)?;
        if json {
            status::print_json(&status::envelope(
                "ok",
                if r.execution_complete {
                    "completed"
                } else {
                    "partial"
                },
                serde_json::to_value(r)?,
            ));
        } else {
            println!(
                "task: gap\nexecution_complete: {}\ndata_complete: {}",
                r.execution_complete, r.data_complete
            );
        }
        return Ok(());
    }
    let m = SuperconductivityManifest::load(&path)
        .with_context(|| format!("data_invalid: Cannot Load {}", path.display()))?;
    if json {
        status::print_json(&status::envelope(
            "ok",
            if m.validation.all_passed() {
                "valid"
            } else {
                "partial"
            },
            serde_json::json!({
                "run_id": m.run_id,
                "mode": m.mode,
                "tc_k": m.tc,
                "located": m.located,
                "converged": m.converged,
                "gaps_mev": m.gaps_mev,
                "validation_status": m.validation.status,
                "native_modes": m.native_modes,
                "workflow_complete": false,
            }),
        ));
    } else {
        println!("run_id: {}", m.run_id);
        println!("mode: {}", m.mode);
        println!("tc_k: {:?}", m.tc);
        println!("located: {}", m.located);
        println!("converged: {}", m.converged);
        println!("validation_status: {}", m.validation.status);
        for (mode, result) in &m.native_modes {
            println!(
                "{mode}: status={} interval_centikelvin={:?} complete={} attempts={}",
                result.search.status,
                result.search.interval_centikelvin,
                result.complete,
                result.attempts.len()
            );
        }
    }
    Ok(())
}

fn import(source: &Path) -> Result<()> {
    if crate::foundation::store::read::<serde_json::Value>(source)?
        .get("task")
        .and_then(|v| v.as_str())
        == Some("gap")
    {
        let r = super::points::Run::load(source)?;
        anyhow::ensure!(r.execution_complete, "partial: specified-temperature execution incomplete");
        println!(
            "imported: run={} task=gap execution_complete={} data_complete={}",
            r.run_id, r.execution_complete, r.data_complete
        );
        return Ok(());
    }
    let m = SuperconductivityManifest::load(source)
        .with_context(|| format!("data_invalid: Cannot Load {}", source.display()))?;
    if !m.validation.all_passed() {
        anyhow::bail!("data_invalid: preceding superconductivity manifest validation failed; import refused");
    }
    println!(
        "imported: run={} mode={} tc={:?} located={}",
        m.run_id, m.mode, m.tc, m.located
    );
    Ok(())
}
