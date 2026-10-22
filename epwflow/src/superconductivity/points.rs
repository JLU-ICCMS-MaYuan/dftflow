//! 指定温度单初值求解：执行完成与能隙有效性独立记录，不执行 Tc 搜索。
use super::{
    attempt::{self, AttemptResult, NativeInputs, Request, SolveKind},
    engine::Entry,
};
use crate::foundation::{
    config::{Config, Mode},
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub schema_version: u32,
    pub task: String,
    pub run_id: String,
    pub interpolation_digest: String,
    pub binding_digest: String,
    pub plan: Vec<Request>,
    pub attempts: Vec<Entry>,
    pub execution_complete: bool,
    pub data_complete: bool,
}

pub fn plan(cfg: &Config) -> Result<Vec<Request>> {
    let modes = match cfg.superconductivity.mode {
        Mode::Iso => vec![Mode::Iso],
        Mode::Aniso => vec![Mode::Aniso],
        Mode::Both => vec![Mode::Iso, Mode::Aniso],
    };
    modes
        .into_iter()
        .flat_map(|mode| {
            cfg.superconductivity
                .gap_temperatures_k
                .iter()
                .map(move |&t| (mode, t))
        })
        .map(|(mode, t)| {
            Request::from_config(
                cfg,
                mode,
                SolveKind::Gap,
                crate::foundation::config::temperature(t, "gap_temperature")?,
                Some(cfg.superconductivity.initial_gap()),
            )
        })
        .collect()
}
fn same(a: &Request, b: &Request) -> Result<bool> {
    Ok(serde_json::to_value(a)? == serde_json::to_value(b)?)
}
impl Run {
    pub fn latest(&self, request: &Request) -> Option<&Entry> {
        self.attempts.iter().rev().find(|a| {
            a.request.mode == request.mode
                && a.request.temperature_centikelvin == request.temperature_centikelvin
        })
    }
    fn refresh(&mut self) {
        self.execution_complete = self.plan.iter().all(|r| {
            self.latest(r)
                .is_some_and(|a| matches!(a.status.as_str(), "valid" | "completed"))
        });
        self.data_complete = self
            .plan
            .iter()
            .all(|r| self.latest(r).is_some_and(|a| a.status == "valid"));
    }
    pub fn load(path: &Path) -> Result<Self> {
        let r: Self = store::read(path)?;
        ensure!(
            r.schema_version == 3 && r.task == "gap" && !r.plan.is_empty(),
            "data_invalid: 指定温度清单版本或计划无效"
        );
        for (i, q) in r.plan.iter().enumerate() {
            q.validate()?;
            ensure!(
                q.kind == SolveKind::Gap
                    && !r.plan[..i].iter().any(|p| p.mode == q.mode
                        && p.temperature_centikelvin == q.temperature_centikelvin),
                "data_invalid: 指定温度计划重复或类型错误"
            );
        }
        for a in &r.attempts {
            ensure!(
                r.plan.iter().any(|q| same(q, &a.request).unwrap_or(false)),
                "input_conflict: 尝试不属于固定温度计划"
            );
            ensure!(
                a.directory.parent() == path.parent(),
                "data_invalid: 尝试目录逃逸"
            );
            ensure!(
                matches!(
                    a.status.as_str(),
                    "running" | "failed" | "valid" | "completed"
                ),
                "data_invalid: 尝试状态无效"
            );
            if let Some(id) = &a.record {
                id.verify()?;
                ensure!(
                    id.path == a.directory.join("attempt.json"),
                    "data_invalid: 尝试记录路径冲突"
                );
                let record = super::verify::record(&id.path)?;
                ensure!(
                    same(&record.request, &a.request)?
                        && record.status == a.status
                        && record.source.interpolation_digest == r.interpolation_digest,
                    "input_conflict: 尝试记录与计划/来源冲突"
                );
            } else {
                ensure!(
                    !matches!(a.status.as_str(), "valid" | "completed"),
                    "data_invalid: 完成点缺少原生证据"
                );
            }
        }
        let mut checked = r.clone();
        checked.refresh();
        ensure!(
            checked.execution_complete == r.execution_complete
                && checked.data_complete == r.data_complete,
            "data_invalid: 指定温度完成状态冲突"
        );
        Ok(r)
    }
}
fn binding(cfg: &Config, input: &NativeInputs, launcher: &[String]) -> Result<String> {
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        crate::foundation::stages::scientific_key(
            cfg,
            crate::foundation::stages::Step::Superconductivity,
        )?,
        input,
        Identity::capture(&cfg.programs.epw.path)?,
        launcher,
    ))?))
}
pub fn published(work: &Path, cfg: &Config, run_id: &str) -> Result<Run> {
    let path = crate::foundation::layout::path(work, "superconductivity/manifest.json");
    let r = Run::load(&path)?;
    let f3 = crate::interpolation::manifest::InterpolationManifest::load(
        &crate::foundation::layout::path(work, "interpolation/manifest.json"),
    )?;
    let input = super::source::bind(work, cfg, &f3)?;
    ensure!(
        r.run_id == run_id
            && r.interpolation_digest == input.interpolation_digest
            && r.binding_digest == binding(cfg, &input, &cfg.programs.launcher)?
            && serde_json::to_value(&r.plan)? == serde_json::to_value(plan(cfg)?)?,
        "input_conflict: 指定温度计划/程序/来源改变"
    );
    Ok(r)
}

pub fn run(
    cfg: &Config,
    work: &Path,
    launcher: &[String],
    backend: crate::foundation::stage::Backend,
    retry: bool,
) -> Result<()> {
    ensure!(
        backend == crate::foundation::stage::Backend::Local,
        "allocation: 单点求解须使用已登记分配"
    );
    let f3_path = crate::foundation::layout::path(work, "interpolation/manifest.json");
    let f3 = crate::interpolation::manifest::InterpolationManifest::load(&f3_path)?;
    ensure!(f3.validation.all_passed(), "data_invalid: 插值数据无效");
    let source = super::source::bind(work, cfg, &f3)?;
    let root = crate::foundation::layout::path(work, "superconductivity");
    std::fs::create_dir_all(&root)?;
    let path = root.join("manifest.json");
    let mut r = if path.exists() {
        published(work, cfg, &f3.run_id)?
    } else {
        Run {
            schema_version: 3,
            task: "gap".into(),
            run_id: f3.run_id,
            interpolation_digest: source.interpolation_digest.clone(),
            binding_digest: binding(cfg, &source, launcher)?,
            plan: plan(cfg)?,
            attempts: vec![],
            execution_complete: false,
            data_complete: false,
        }
    };
    // 持久记录先于提交；中断后仅凭原生任务或原分配终态恢复。
    for i in 0..r.attempts.len() {
        if r.attempts[i].status != "running" {
            continue;
        }
        let dir = r.attempts[i].directory.clone();
        let saved = dir.join("attempt.json");
        let record = if crate::foundation::native_jobs::enabled() {
            if crate::foundation::native_jobs::continuing(&dir, "epw")? {
                Some(attempt::run_attempt(
                    &r.attempts[i].request,
                    &source,
                    &cfg.programs.epw,
                    launcher,
                    &dir,
                )?)
            } else {
                ensure!(
                    crate::foundation::native_jobs::failed(&dir)?,
                    "scheduler_unknown: 中断点缺少已确认失败的原生任务"
                );
                None
            }
        } else if saved.exists() && store::read::<AttemptResult>(&saved)?.status != "running" {
            Some(super::verify::record(&saved)?)
        } else {
            ensure!(retry, "scheduler_unknown: 中断尝试需要显式 --resume");
            let job = r.attempts[i]
                .job_id
                .context("scheduler_unknown: 中断尝试缺少原分配号")?;
            ensure!(
                crate::foundation::slurm::accounting(job)?.terminal,
                "scheduler_unknown: 原分配仍活动"
            );
            None
        };
        if let Some(record) = record {
            r.attempts[i].status = record.status;
            r.attempts[i].reason = record.reason;
            r.attempts[i].record = Some(Identity::capture(&saved)?);
        } else {
            r.attempts[i].status = "failed".into();
            r.attempts[i].reason = Some("execution_failed: 原生尝试已确认中断".into());
            if saved.exists() {
                let mut record: AttemptResult = store::read(&saved)?;
                ensure!(
                    same(&record.request, &r.attempts[i].request)?,
                    "input_conflict: 中断点请求改变"
                );
                record.source.verify(record.request.mode)?;
                record.executable.verify()?;
                for id in &record.artifacts {
                    id.verify()?;
                }
                record.status = "failed".into();
                record.reason = r.attempts[i].reason.clone();
                let code = dir.join("epw.exitcode");
                if crate::foundation::native_jobs::enabled() && code.is_file() {
                    record.exit_status = Some(std::fs::read_to_string(&code)?.trim().parse()?);
                    record.artifacts.push(Identity::capture(&code)?);
                }
                for name in ["epw.out", "epw.err"] {
                    let log = dir.join(name);
                    if log.is_file() && !record.artifacts.iter().any(|id| id.path == log) {
                        record.artifacts.push(Identity::capture(&log)?);
                    }
                }
                store::write(&saved, &record)?;
                r.attempts[i].record = Some(Identity::capture(&saved)?);
            }
        }
        r.refresh();
        store::write(&path, &r)?;
    }
    for request in r.plan.clone() {
        if let Some(old) = r.latest(&request) {
            if matches!(old.status.as_str(), "valid" | "completed") {
                continue;
            }
            ensure!(
                retry
                    && (!crate::foundation::native_jobs::enabled()
                        || crate::foundation::native_jobs::retry_enabled()),
                "partial: 点失败，修复后使用 --resume；原尝试已保留"
            );
        }
        if !crate::foundation::native_jobs::enabled() {
            let job = std::env::var("SLURM_JOB_ID")
                .context("allocation: 缺少当前 Slurm 分配")?
                .parse::<u64>()?;
            crate::foundation::budget::with_ledger(
                &cfg.budget.ledger,
                &cfg.budget.campaign_id,
                |ledger| ledger.require_allocation(job, &cfg.resources).map(|_| ()),
            )?;
        }
        ensure!(
            store::hash_file(&f3_path)? == source.interpolation_digest,
            "input_conflict: F3 清单改变"
        );
        let mode = if request.mode == Mode::Iso {
            "iso"
        } else {
            "aniso"
        };
        let dir = root.join(format!(
            "{mode}-{:06.2}K-attempt-{:04}",
            request.temperature_centikelvin as f64 / 100.0,
            r.attempts.len() + 1
        ));
        r.attempts.push(Entry {
            request: request.clone(),
            directory: dir.clone(),
            record: None,
            status: "running".into(),
            reason: None,
            job_id: std::env::var("SLURM_JOB_ID")
                .ok()
                .and_then(|j| j.parse().ok()),
        });
        r.refresh();
        store::write(&path, &r)?;
        let result = attempt::run_attempt(&request, &source, &cfg.programs.epw, launcher, &dir);
        match result {
            Ok(record) => {
                let a = r.attempts.last_mut().unwrap();
                a.status = record.status;
                a.reason = record.reason;
                a.record = Some(Identity::capture(&dir.join("attempt.json"))?);
                r.refresh();
                store::write(&path, &r)?;
                ensure!(
                    matches!(
                        r.attempts.last().unwrap().status.as_str(),
                        "valid" | "completed"
                    ),
                    "execution_failed: 原生单点异常退出；证据已保留"
                );
            }
            Err(error) => {
                if !crate::foundation::native_jobs::paused(&error) {
                    let a = r.attempts.last_mut().unwrap();
                    a.status = "failed".into();
                    a.reason = Some(format!("{error:#}"));
                    r.refresh();
                    store::write(&path, &r)?;
                }
                return Err(error);
            }
        }
    }
    r.refresh();
    store::write(&path, &r)?;
    ensure!(r.execution_complete, "partial: 指定温度尚未全部执行完成");
    Ok(())
}
