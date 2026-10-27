//! 编号工作区阶段循环：普通目标手动推进，声子目标在已有分配内连续运行。
use super::{
    config::Config,
    layout, native_jobs, parallelism,
    stages::{self, Completion, Step},
    store, workspace, workspace_science,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Progress {
    key: String,
    generation: String,
    state: String,
}

pub fn advance(config: &Path, target: Step, submit: bool, resume: bool) -> Result<()> {
    advance_mode(config, target, submit, resume, None)
}

/// 在目标阶段的同一分配内运行既有阶段循环；父尝试已核对资源和提交身份。
pub(super) fn allocated(config: &Path, target: Step, resume: bool, attempt: &str) -> Result<()> {
    advance_mode(config, target, false, resume, Some(attempt))
}

fn advance_mode(config: &Path, target: Step, submit: bool, resume: bool, own: Option<&str>) -> Result<()> {
    let cfg = Config::load(config)?;
    if target == Step::Phonon && super::phonon_import::bound(&cfg) {
        super::phonon_import::require_electronic(&cfg)?;
    }
    super::run_state::ready(&cfg.work_dir)?;
    workspace::check_active(&cfg, own)?;
    if own.is_none() {
        native_jobs::reconcile(&cfg, resume)?;
    }
    // 先阻止其他阶段改写并行方法仍在读取的上游。
    if target != Step::Superconductivity || cfg.superconductivity.task != super::config::SuperconductivityTask::Gap {
        native_jobs::check_groups(&cfg)?;
    }
    layout::initialize(&cfg)?;
    store::atomic_bytes(
        &cfg.work_dir.join("epw-state/current-config.toml"),
        toml::to_string_pretty(&cfg)?.as_bytes(),
    )?;
    cfg.write_parameter_map(&cfg.work_dir.join("epw-state/parameter-map.json"))?;
    for step in stages::execution_steps(&cfg, target) {
        if step == target || !stages::reusable(&cfg, step)? {
            parallelism::preflight(&cfg, step, false)?;
        }
    }
    for step in stages::execution_steps(&cfg, target) {
        // 显式 SCF 恢复提交开启新批次；依赖阶段及分配内接续仍按完成记录复用。
        let restart_scf = own.is_none() && submit && resume && target == Step::Scf;
        if !restart_scf && stages::reusable(&cfg, step)? {
            println!("reuse: {}", step.name());
            if refresh_stage(&cfg,step,submit,resume,own)? {pending(&cfg,target,own.is_some())?;return Ok(());}
            continue;
        }
        parallelism::preflight(&cfg, step, false)?;
        if step != Step::Superconductivity {
            native_jobs::check_groups(&cfg)?;
        }
        let key = stages::scientific_key(&cfg, step)?;
        let path = cfg
            .work_dir
            .join("epw-state/native-progress")
            .join(format!("{}.json", step.name()));
        let old = if path.is_file() {
            Some(store::read::<Progress>(&path)?)
        } else {
            None
        };
        let mut p = match old {
            Some(p) if p.key == key && !restart_scf => {
                ensure!(
                    p.state != "failed" || resume,
                    "execution_failed: {} 校验失败，修复后显式 --resume -s",
                    step.name()
                );
                let native_failed = native_jobs::active_task(&cfg)?
                    .is_some_and(|a| a.generation == p.generation && a.state == "failed");
                if resume
                    && (p.state == "failed" || native_failed)
                    && (matches!(step, Step::Scf | Step::Nscf | Step::Wannier)
                        || (step == Step::Phonon && !super::phonon_import::bound(&cfg) && workspace_science::partial_phonon(&cfg)?.is_none()))
                {
                    workspace::archive_step(&cfg, step)?;
                    Progress {
                        key: key.clone(),
                        generation: store::token(),
                        state: "running".into(),
                    }
                } else {
                    p
                }
            }
            _ => {
                native_jobs::check_groups(&cfg)?;
                workspace::archive_step(&cfg, step)?;
                Progress {
                    key: key.clone(),
                    generation: store::token(),
                    state: "running".into(),
                }
            }
        };
        if resume && step == Step::Superconductivity
            && cfg.superconductivity.task == super::config::SuperconductivityTask::Gap {
            p.state = "running".into();
        }
        store::write(
            &cfg.work_dir
                .join("epw-state/requests")
                .join(format!("{}.json", step.name())),
            &key,
        )?;
        store::write(&path, &p)?;
        let upstream = stages::upstream(&cfg, step)?;
        let run = if step == Step::Superconductivity
            && cfg.superconductivity.task == super::config::SuperconductivityTask::Gap {
            crate::superconductivity::batch::run(&cfg, &p.generation, submit, resume)
        } else if own.is_some() {
            native_jobs::scoped_allocated(&cfg, &p.generation, resume, || workspace_science::run(&cfg, step))
        } else {
            native_jobs::scoped(&cfg, &p.generation, submit, resume, || workspace_science::run(&cfg, step))
        };
        match run {
            Err(e) if native_jobs::paused(&e) => {
                pending(&cfg, target, own.is_some())?;
                return Ok(());
            }
            Err(e) => {
                native_jobs::reject(&cfg, &p.generation)?;
                p.state = "failed".into();
                store::write(&path, &p)?;
                return Err(e);
            }
            Ok(()) => {
                let c = Completion {
                    schema_version: 1,
                    step,
                    scientific_key: key,
                    upstream,
                    outputs: workspace_science::evidence(&cfg, step)?,
                    provenance: None,
                };
                c.verify()?;
                store::write(&stages::receipt(&cfg.work_dir, step), &c)?;
                native_jobs::accept(&cfg, &p.generation)?;
                p.state = "complete".into();
                store::write(&path, &p)?;
                if refresh_stage(&cfg,step,submit,resume,own)? {pending(&cfg,target,own.is_some())?;return Ok(());}
            }
        }
    }
    // 图表及DOS 后处理也从同一个原生执行边界逐项交回控制权。
    let generation = store::hash_bytes(&serde_json::to_vec(&(
        "post",
        target,
        &cfg.comparison,
        target
            .closure()
            .iter()
            .map(|&s| stages::scientific_key(&cfg, s))
            .collect::<Result<Vec<_>>>()?,
    ))?);
    let post = || workspace_science::refresh_post(&cfg, target);
    let result=if own.is_some() {native_jobs::scoped_allocated(&cfg,&generation,resume,post)}else{native_jobs::scoped(&cfg,&generation,submit,resume,post)};
    match result {
        Err(e) if native_jobs::paused(&e) => {
            pending(&cfg, target, own.is_some())?;
            Ok(())
        }
        Err(e) => {
            native_jobs::reject(&cfg, &generation)?;
            Err(e)
        }
        Ok(()) => {
            pending(&cfg, target, own.is_some())?;
            println!("workflow_status: complete\nworkflow_complete: true");
            Ok(())
        }
    }
}

/// 科学回执先保存；派生交付失败不会将已完成的科学阶段改为失败。
fn refresh_stage(cfg:&Config,step:Step,submit:bool,resume:bool,own:Option<&str>)->Result<bool> {
    if !matches!(step,Step::Electronic|Step::Phonon|Step::Wannier|Step::Coarse|Step::Superconductivity) {return Ok(false);}
    let generation=store::hash_bytes(&serde_json::to_vec(&("stage-post",step,
        stages::scientific_key(cfg,step)?,&cfg.comparison))?);
    let result=if own.is_some() {native_jobs::scoped_allocated(cfg,&generation,resume,||workspace_science::refresh_post(cfg,step))}
        else {native_jobs::scoped(cfg,&generation,submit,resume,||workspace_science::refresh_post(cfg,step))};
    match result {
        Err(e) if native_jobs::paused(&e)=>Ok(true),
        Err(e)=>{native_jobs::reject(cfg,&generation)?;Err(e)},
        Ok(())=>{native_jobs::accept(cfg,&generation)?;Ok(false)}
    }
}

fn pending(cfg: &Config, target: Step, allocated: bool) -> Result<()> {
    let steps = target.closure().into_iter().map(|s| Ok(serde_json::json!({"step":s,"state":if stages::reusable(cfg,s)? {"validated"}else{"pending"}}))).collect::<Result<Vec<_>>>()?;
    let path = cfg.work_dir.join("native-plan.json");
    let advance = if allocated {
        "本阶段在分配内自动完成，可用 --status 查看"
    } else if target == Step::Scf {
        "作业结束后执行 --stage scf -s（不带 --resume）归集；--resume -s 会重新计算"
    } else {
        "作业结束后再次执行相同公开命令 -s"
    };
    store::write(
        &path,
        &serde_json::json!({"schema_version":1,"target":target,"task":native_jobs::status(cfg)?,"methods":native_jobs::groups_status(cfg)?,"steps":steps,"advance":advance}),
    )?;
    println!("plan: {}", path.display());
    // 不再产生 submit_all.sh；已有历史文件不修改，清单明确取代它。
    Ok(())
}
