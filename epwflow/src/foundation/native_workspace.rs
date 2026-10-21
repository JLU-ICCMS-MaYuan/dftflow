//! 公开命令手动推进编号工作区；作业内只有科学程序。
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
    let cfg = Config::load(config)?;
    let _guard = workspace::control(&cfg)?;
    workspace::check_active(&cfg, None)?; // 旧包装作业活动时也不能新提交。
    native_jobs::reconcile(&cfg, resume)?;
    layout::initialize(&cfg)?;
    store::atomic_bytes(
        &cfg.work_dir.join("epw-state/current-config.toml"),
        toml::to_string_pretty(&cfg)?.as_bytes(),
    )?;
    for step in target.closure() {
        if step == target || !stages::reusable(&cfg, step)? {
            parallelism::preflight(&cfg, step, false)?;
        }
    }
    for step in target.closure() {
        if stages::reusable(&cfg, step)? {
            println!("reuse: {}", step.name());
            continue;
        }
        parallelism::preflight(&cfg, step, false)?;
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
            Some(p) if p.key == key => {
                ensure!(
                    p.state != "failed" || resume,
                    "execution_failed: {} 校验失败，修复后显式 --resume -s",
                    step.name()
                );
                let native_failed = native_jobs::active_task(&cfg)?
                    .is_some_and(|a| a.generation == p.generation && a.state == "failed");
                if resume
                    && (p.state == "failed" || native_failed)
                    && matches!(step, Step::Scf | Step::Nscf | Step::Wannier)
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
                workspace::archive_step(&cfg, step)?;
                Progress {
                    key: key.clone(),
                    generation: store::token(),
                    state: "running".into(),
                }
            }
        };
        store::write(
            &cfg.work_dir
                .join("epw-state/requests")
                .join(format!("{}.json", step.name())),
            &key,
        )?;
        store::write(&path, &p)?;
        let upstream = stages::upstream(&cfg, step)?;
        let run = native_jobs::scoped(&cfg, &p.generation, submit, resume, || {
            workspace_science::run(&cfg, step)
        });
        match run {
            Err(e) if native_jobs::paused(&e) => {
                pending(&cfg, target)?;
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
            }
        }
    }
    // 图表及独立 W90 后处理也从同一个原生执行边界逐项交回控制权。
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
    match native_jobs::scoped(&cfg, &generation, submit, resume, || {
        workspace_science::refresh_post(&cfg, target)
    }) {
        Err(e) if native_jobs::paused(&e) => {
            pending(&cfg, target)?;
            Ok(())
        }
        Err(e) => {
            native_jobs::reject(&cfg, &generation)?;
            Err(e)
        }
        Ok(()) => {
            pending(&cfg, target)?;
            println!("workflow_status: complete\nworkflow_complete: true");
            Ok(())
        }
    }
}

fn pending(cfg: &Config, target: Step) -> Result<()> {
    let steps = target.closure().into_iter().map(|s| Ok(serde_json::json!({"step":s,"state":if stages::reusable(cfg,s)? {"validated"}else{"pending"}}))).collect::<Result<Vec<_>>>()?;
    let path = cfg.work_dir.join("native-plan.json");
    store::write(
        &path,
        &serde_json::json!({"schema_version":1,"target":target,"task":native_jobs::status(cfg)?,"steps":steps,"advance":"作业结束后再次执行相同公开命令 -s"}),
    )?;
    println!("plan: {}", path.display());
    // 不再产生 submit_all.sh；已有历史文件不修改，清单明确取代它。
    Ok(())
}
