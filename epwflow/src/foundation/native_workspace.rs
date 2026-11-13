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
    if result.is_err() && target >= Step::Wannier {
        if let Ok(cfg) = super::readable_state::load_config(config) {
            if Step::Wannier
                .directory(&cfg)
                .join(format!("{}.wout", cfg.qe.prefix))
                .is_file()
            {
                match super::wannier_diagnostics::generate(&cfg) {
                    Ok(p) => println!("诊断图：{}", p.display()),
                    Err(e) => eprintln!("诊断图：{e:#}"),
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
            "data_invalid: 外部声子来源记录缺失，请重新指定 --import-phonon；不会重新执行 PH"
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
            println!("复用：{}", log::stage_name(step, log::mode(&cfg)));
            if refresh_stage(&cfg, step, submit, resume, own)? {
                return pending(&cfg, target);
            }
            continue;
        }
        parallelism::preflight(&cfg, step, own.is_some())?;
        let previous = log::read(&cfg.work_dir, step)?;
        let compatible = previous
            .get("阶段", "计算配置")
            .and_then(|p| super::readable_state::load_config(&log::resolve(&cfg.work_dir, p)).ok())
            .map(|old| {
                Ok::<_, anyhow::Error>(
                    stages::parameters(&old, step)? == stages::parameters(&cfg, step)?,
                )
            })
            .transpose()?
            .unwrap_or(false);
        let failed = previous.get("阶段", "状态") == Some("失败");
        let upstream = stages::upstream(&cfg, step)?;
        let dependencies_match = upstream.iter().all(|(name, batch)| {
            previous.get("依赖", &format!("{name} 批次")) == Some(batch.as_str())
        });
        let completed_but_missing = previous.get("阶段", "科学状态") == Some("已完成")
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
                .get("阶段", "批次")
                .map(str::to_owned)
                .unwrap_or_else(store::token)
        } else {
            workspace::archive_step(&cfg, step)?;
            native_jobs::retire_stage(&cfg, step)?;
            store::token()
        };
        log::update(&cfg.work_dir, step, |d| {
            d.set("阶段", "状态", "推进中");
            d.set("阶段", "科学状态", "待检查");
            d.set("阶段", "批次", &generation);
            d.set("阶段", "计算配置", log::relative(&cfg.work_dir, &version));
            d.remove("问题");
            Ok(())
        })?;
        log::update(&cfg.work_dir, step, |d| {
            d.remove("依赖");
            for (name, batch) in &upstream {
                d.set("依赖", &format!("{name} 批次"), batch);
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
                    d.set("阶段", "状态", "失败");
                    d.set("问题", "原因", format!("{e:#}"));
                    d.set(
                        "下一步",
                        "操作",
                        format!("修复原因后执行：{}", log::next_command(&cfg, step, true)?),
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
            .get("阶段", "批次")
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
            let valid = log::read(&cfg.work_dir, target)?.get("阶段", "科学状态") == Some("已完成");
            log::update(&cfg.work_dir, target, |d| {
                d.set(
                    "下一步",
                    "操作",
                    if valid {
                        "本阶段科学计算及派生交付已完成；可以推进下一阶段"
                    } else {
                        "求解已结束，能隙数据无效；请查看各温度的收敛及数据原因"
                    },
                );
                Ok(())
            })?;
            println!(
                "{}\nstage_complete: {}\nworkflow_complete: {}",
                if valid {
                    "本阶段已完成，科学结果检查通过"
                } else {
                    "求解已结束，能隙数据无效"
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
            .get("阶段", "批次")
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
                doc.set("阶段", "状态", "后处理失败，科学结果保留");
                doc.set("问题", "原因", format!("{e:#}"));
                doc.set(
                    "下一步",
                    "操作",
                    format!("修复原因后执行：{}", log::next_command(cfg, step, true)?),
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
            "下一步",
            "操作",
            format!("作业结束后执行：{}", log::next_command(cfg, target, false)?),
        );
        Ok(())
    })?;
    println!("进度：{}", log::path(&cfg.work_dir, target)?.display());
    Ok(())
}
