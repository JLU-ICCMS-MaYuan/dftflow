//! 编号阶段循环；当前阶段、依赖批次及续算位置只保存在八份日志。
use super::{
    config::Config,
    native_jobs, parallelism, readable_state as log,
    stages::{self, Step},
    store, workspace, workspace_science,
};
use anyhow::Result;
use std::path::Path;

pub fn advance(config: &Path, target: Step, submit: bool, resume: bool) -> Result<()> {
    let result = advance_mode(config, target, submit, resume, None);
    if result.as_ref().is_err_and(|e| !format!("{e:#}").contains("plot_failed:")) && target >= Step::Wannier {
        if let Ok(cfg) = super::readable_state::load_config(config) {
            if Step::Wannier
                .directory(&cfg)
                .join(format!("{}.wout", cfg.qe.prefix))
                .is_file()
            {
                match super::wannier_diagnostics::generate(&cfg) {
                    Ok(p) => println!("diagnostic_plots: {}", p.display()),
                    Err(e) => eprintln!("diagnostic_plots: {e:#}"),
                }
            }
        }
    }
    result
}
pub(super) fn allocated(config: &Path, target: Step, resume: bool, id: &str) -> Result<()> {
    advance_mode(config, target, false, resume, Some(id))
}
fn advance_mode(
    config: &Path,
    target: Step,
    submit: bool,
    resume: bool,
    own: Option<&str>,
) -> Result<()> {
    let cfg = super::readable_state::load_config(config)?;
    let internal_version = own.is_some()
        && config
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|name| {
                name.strip_suffix("-config.toml")
                    .is_some_and(|n| n.len() >= 4 && n.bytes().all(|b| b.is_ascii_digit()))
            });
    let version = if internal_version {
        std::fs::canonicalize(config)?
    } else {
        let version = log::backup_config(config)?;
        log::register_config(&cfg, config, &version)?;
        version
    };
    let _config = log::ExecutionConfig::enter(&cfg.work_dir, &version);
    let _operation = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    let _electronic = super::electronic_reuse::Scope::enter(&cfg)?;
    if target == Step::Phonon && super::phonon_import::bound(&cfg) {
        super::phonon_import::require_electronic(&cfg)?;
    }
    super::run_state::ready(&cfg.work_dir)?;
    workspace::check_active(&cfg, own)?;
    if own.is_none() {
        native_jobs::reconcile(&cfg, resume)?;
    } else {
        native_jobs::check_other_allocations(&cfg)?;
    }
    if target != Step::Superconductivity
        || cfg.superconductivity.task != super::config::SuperconductivityTask::Gap
    {
        native_jobs::check_groups(&cfg)?;
    }
    parallelism::preflight(&cfg, target, own.is_some())?;
    if resume {
        crate::delivery::decay::recover_pending(&cfg)?;
    }
    if target >= Step::Phonon && super::phonon_import::bound(&cfg) {
        anyhow::ensure!(
            super::phonon_import::binding(&cfg).is_file(),
            "data_invalid: external phonon source record missing; specify --import-phonon again; PH will not be rerun"
        );
    }
    // 先检查已有结果的损坏，再提交任何缺失的依赖任务。
    for step in stages::execution_steps(&cfg, target) {
        match stages::reusable(&cfg, step) {
            Err(_) if resume && step == target => (),
            Err(error) => return Err(error),
            Ok(_) => (),
        }
    }
    for step in stages::execution_steps(&cfg, target) {
        let force_scf = own.is_none() && submit && resume && target == Step::Scf;
        let reusable = match stages::reusable(&cfg, step) {
            Ok(value) => value,
            Err(_) if resume && step == target => false,
            Err(error) => return Err(error),
        };
        if !force_scf && reusable {
            if step != Step::Electronic {
                log::report_reuse(&cfg, step, None);
            }
            if refresh_stage(&cfg, step, submit, resume, own)? {
                return pending(&cfg, target);
            }
            if step == Step::Electronic {
                log::report_reuse(&cfg, step, None);
            }
            continue;
        }
        parallelism::preflight(&cfg, step, own.is_some())?;
        let previous = log::read(&cfg.work_dir, step)?;
        if let Some(reason)=previous.get("Plot failure","Reason") {anyhow::bail!("plot_failed: {reason}; automatic retry disabled; request plotting explicitly");}
        let compatible = previous
            .get("Stage", "Calculation configuration")
            .and_then(|p| super::readable_state::load_config(&log::resolve(&cfg.work_dir, p)).ok())
            .map(|old| {
                Ok::<_, anyhow::Error>(
                    stages::parameters(&old, step)? == stages::parameters(&cfg, step)?,
                )
            })
            .transpose()?
            .unwrap_or(false);
        let failed = previous.get("Stage", "Status") == Some("failed");
        let upstream = stages::upstream(&cfg, step)?;
        let dependencies_match = upstream.iter().all(|(name, batch)| {
            previous.get("Dependencies", &format!("{name} batch")) == Some(batch.as_str())
        });
        let completed_but_missing = previous.get("Stage", "Scientific status") == Some("completed")
            && !(step == Step::Phonon
                && compatible
                && dependencies_match
                && workspace_science::partial_phonon(&cfg)?.is_some());
        let generation = if compatible
            && dependencies_match
            && !completed_but_missing
            && !force_scf
            && !(resume && failed && matches!(step, Step::Scf | Step::Nscf | Step::Wannier))
        {
            previous
                .get("Stage", "Batch")
                .map(str::to_owned)
                .unwrap_or_else(store::token)
        } else {
            workspace::archive_step(&cfg, step)?;
            native_jobs::retire_stage(&cfg, step)?;
            store::token()
        };
        log::update(&cfg.work_dir, step, |d| {
            d.set("Stage", "Status", "advancing");
            d.set("Stage", "Scientific status", "validation pending");
            d.set("Stage", "Batch", &generation);
            d.set("Stage", "Calculation configuration", log::relative(&cfg.work_dir, &version));
            d.remove("Problem");
            Ok(())
        })?;
        log::update(&cfg.work_dir, step, |d| {
            d.remove("Dependencies");
            for (name, batch) in &upstream {
                d.set("Dependencies", &format!("{name} batch"), batch);
            }
            Ok(())
        })?;
        let run = if step == Step::Superconductivity
            && cfg.superconductivity.task == super::config::SuperconductivityTask::Gap
        {
            crate::superconductivity::batch::run_mode(
                &cfg,
                &generation,
                submit,
                resume,
                own.is_some(),
            )
        } else if own.is_some() {
            native_jobs::scoped_allocated(&cfg, &generation, resume, || {
                workspace_science::run(&cfg, step)
            })
        } else {
            native_jobs::scoped(&cfg, &generation, submit, resume, || {
                workspace_science::run(&cfg, step)
            })
        };
        match run {
            Err(e) if native_jobs::paused(&e) => return pending(&cfg, target),
            Err(e) => {
                native_jobs::reject(&cfg, &generation)?;
                log::update(&cfg.work_dir, step, |d| {
                    d.set("Stage", "Status", "failed");
                    if format!("{e:#}").contains("plot_failed:") {d.set("Plot failure","Reason",format!("{e:#}"));}
                    d.set("Problem", "Reason", format!("{e:#}"));
                    d.set(
                        "Next step",
                        "Action",
                        format!("After resolving the cause, run: {}", log::next_command(&cfg, step, true)?),
                    );
                    Ok(())
                })?;
                return Err(e);
            }
            Ok(()) => {
                stages::finish(&cfg, step)?;
                native_jobs::accept(&cfg, &generation)?;
                if refresh_stage(&cfg, step, submit, resume, own)? {
                    return pending(&cfg, target);
                }
            }
        }
    }
    let generation = format!(
        "post-{}",
        log::read(&cfg.work_dir, target)?
            .get("Stage", "Batch")
            .unwrap_or("existing")
    );
    let post = || workspace_science::refresh_post(&cfg, target);
    let result = if own.is_some() {
        native_jobs::scoped_allocated(&cfg, &generation, resume, post)
    } else {
        native_jobs::scoped(&cfg, &generation, submit, resume, post)
    };
    match result {
        Err(e) if native_jobs::paused(&e) => pending(&cfg, target),
        Err(e) => Err(e),
        Ok(()) => {
            let valid = log::read(&cfg.work_dir, target)?.get("Stage", "Scientific status") == Some("completed");
            log::update(&cfg.work_dir, target, |d| {
                d.set(
                    "Next step",
                    "Action",
                    if valid {
                        "Stage scientific calculation and derived delivery complete; advance to the next stage"
                    } else {
                        "Solver finished; gap data invalid; inspect convergence and data diagnostics for each temperature"
                    },
                );
                Ok(())
            })?;
            println!(
                "{}\nstage_complete: {}\nworkflow_complete: {}",
                if valid {
                    "Stage complete; scientific result validation passed"
                } else {
                    "Solver finished; gap data are invalid"
                },
                valid,
                Step::ALL
                    .into_iter()
                    .all(|step| stages::reusable(&cfg, step).unwrap_or(false))
            );
            Ok(())
        }
    }
}
fn refresh_stage(
    cfg: &Config,
    step: Step,
    submit: bool,
    resume: bool,
    own: Option<&str>,
) -> Result<bool> {
    if !matches!(
        step,
        Step::Scf | Step::Nscf | Step::Electronic | Step::Phonon | Step::Wannier | Step::Coarse | Step::Superconductivity
    ) {
        return Ok(false);
    }
    let generation = format!(
        "post-{}",
        log::read(&cfg.work_dir, step)?
            .get("Stage", "Batch")
            .unwrap_or("existing")
    );
    let run = || workspace_science::refresh_post(cfg, step);
    let result = if own.is_some() {
        native_jobs::scoped_allocated(cfg, &generation, resume, run)
    } else {
        native_jobs::scoped(cfg, &generation, submit, resume, run)
    };
    match result {
        Err(e) if native_jobs::paused(&e) => Ok(true),
        Err(e) => {
            native_jobs::reject(cfg, &generation)?;
            log::update(&cfg.work_dir, step, |doc| {
                doc.set("Stage", "Status", "Postprocessing failed; scientific results preserved");
                doc.set("Problem", "Reason", format!("{e:#}"));
                doc.set(
                    "Next step",
                    "Action",
                    format!("After resolving the cause, run: {}", log::next_command(cfg, step, true)?),
                );
                Ok(())
            })?;
            Err(e)
        }
        Ok(()) => {
            native_jobs::accept(cfg, &generation)?;
            Ok(false)
        }
    }
}
fn pending(cfg: &Config, target: Step) -> Result<()> {
    log::update(&cfg.work_dir, target, |d| {
        d.set(
            "Next step",
            "Action",
            format!("After the job finishes, run: {}", log::next_command(cfg, target, false)?),
        );
        Ok(())
    })?;
    println!("progress: {}", log::path(&cfg.work_dir, target)?.display());
    Ok(())
}
