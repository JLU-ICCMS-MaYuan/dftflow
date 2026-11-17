//! 原生计算检查点统一写入阶段日志；现有科学文件由归集器直接检查。
use crate::foundation::{native_jobs, readable_state as log, stages::Step, store::Identity};
use anyhow::{ensure, Result};
use std::path::{Path, PathBuf};

pub fn run<T>(
    work: &Path,
    label: &str,
    dir: &Path,
    digest: String,
    retry: bool,
    prepare: impl FnOnce() -> Result<()>,
    execute: impl FnOnce() -> Result<()>,
    collect: impl Fn() -> Result<T>,
    evidence: impl Fn(&T) -> Result<Vec<Identity>>,
) -> Result<T> {
    let stem = if label == "coarse" { "epw_coarse" }
        else if label.starts_with("fine-smearing-") { "epw_fine" }
        else { "epw" };
    let exit_name = format!("{stem}.exitcode");
    run_with_inputs(
        work,
        label,
        dir,
        digest,
        retry,
        &exit_name,
        None,
        prepare,
        execute,
        collect,
        evidence,
    )
}
pub(super) fn run_with_inputs<T>(
    work: &Path,
    label: &str,
    dir: &Path,
    _digest: String,
    retry: bool,
    exit_code: &str,
    _selected_inputs: Option<&[PathBuf]>,
    prepare: impl FnOnce() -> Result<()>,
    execute: impl FnOnce() -> Result<()>,
    collect: impl Fn() -> Result<T>,
    evidence: impl Fn(&T) -> Result<Vec<Identity>>,
) -> Result<T> {
    ensure!(
        dir.starts_with(work),
        "configuration: Numerical Catalogue does not belong to the workspace"
    );
    let exit_path = || {
        let stem = Path::new(exit_code).file_stem().and_then(|s| s.to_str()).unwrap_or(exit_code);
        crate::foundation::scientific_logs::output(dir, stem).with_extension("exitcode")
    };
    let cfg = if crate::foundation::layout::enabled(work) {
        Some(crate::qe_data::direct::current(work)?)
    } else {
        None
    };
    let step = cfg.as_ref().and_then(|c| {
        Step::ALL
            .into_iter()
            .find(|s| dir.starts_with(s.directory(c)))
    });
    let section = format!("Substep · {label}");
    if let (Some(cfg), Some(step)) = (&cfg, step) {
        if log::completed_files(cfg, step, &section)?.is_some() {
            let compatible = log::checkpoint_config(cfg, step, &section)?
                .map(|old| {
                    Ok::<_, anyhow::Error>(
                        parameters(&old, label, step)? == parameters(cfg, label, step)?,
                    )
                })
                .transpose()?
                .unwrap_or(false);
            if compatible {
                return collect();
            }
            native_jobs::retire(dir)?;
            replace_outputs(dir)?;
            log::update(&cfg.work_dir, step, |doc| {
                doc.remove(&section);
                Ok(())
            })?;
        }
    }
    let continuing = native_jobs::continuing_directory(dir)?;
    if let Some(cfg) = &cfg {
        if !continuing
            && native_jobs::retry_enabled()
            && native_jobs::tasks(cfg)?
                .iter()
                .any(|t| t.directory == dir && t.state == "failed")
        {
            replace_outputs(dir)?;
        }
    } else if dir.exists() {
        let exit = exit_path();
        if exit.is_file() {
            if std::fs::read_to_string(&exit)?.trim() == "0" {
                if let Ok(result) = collect() {
                    for id in evidence(&result)? {
                        id.verify()?;
                    }
                    return Ok(result);
                }
            }
            ensure!(retry, "execution_failed: The birthbaby mission failed. It needs a visible recovery");
            replace_outputs(dir)?;
        } else {
            ensure!(
                std::fs::read_dir(dir)?.next().is_none(),
                "scheduler_unknown: No exit code confirmed for the native directory {}",
                dir.display()
            );
        }
    }
    if !continuing {
        prepare()?;
    }
    execute()?;
    ensure!(
        std::fs::read_to_string(exit_path())?.trim() == "0",
        "execution_failed: native exit failed"
    );
    let result = collect()?;
    let ids = evidence(&result)?;
    for id in &ids {
        id.verify()?;
    }
    if let (Some(cfg), Some(step)) = (&cfg, step) {
        log::complete_files(
            cfg,
            step,
            &section,
            &ids.iter().map(|id| id.path.clone()).collect::<Vec<_>>(),
        )?;
    }
    Ok(result)
}
fn parameters(
    cfg: &crate::foundation::config::Config,
    label: &str,
    step: Step,
) -> Result<serde_json::Value> {
    if let Some(kind) = label.strip_prefix("comparison-") {
        if kind == "epw" {
            return Ok(serde_json::json!([
                cfg.native_value::<serde_json::Value>(&[crate::foundation::native_config::COARSE, "comparison", "epw"])?,
                Some(cfg.native_value::<u32>(&[crate::foundation::native_config::COARSE, "workflow", "points_per_edge"]) ?),
                Option::<String>::None,
                Option::<String>::None,
                crate::foundation::store::hash_file(&cfg.programs.epw.path)?
            ]));
        }
    }
    crate::foundation::stages::parameters(cfg, step)
}
fn replace_outputs(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if native_jobs::is_log(&entry.path())
            || entry.path().extension().is_some_and(|e| e == "jobs")
        {
            continue;
        }
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(entry.path())?;
        } else {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}
