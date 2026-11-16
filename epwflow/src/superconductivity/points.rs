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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batches: Vec<super::batch::Batch>,
    pub execution_complete: bool,
    pub data_complete: bool,
}

pub fn plan(cfg: &Config) -> Result<Vec<Request>> {
    let modes = match cfg.superconductivity.mode {
        Mode::Iso => vec![Mode::Iso],
        Mode::Aniso => vec![Mode::Aniso],
        Mode::Both => vec![Mode::Iso, Mode::Aniso],
    };
    let mut requests = Vec::new();
    for mode in modes {
        let settings = cfg.solver_for(mode)?;
        for &t in &settings.gap_temperatures_k {
            requests.push(Request::from_config(
                cfg,
                mode,
                SolveKind::Gap,
                crate::foundation::config::temperature(t, "gap_temperature")?,
                Some(settings.initial_gap()),
            )?);
        }
    }
    Ok(requests)
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
    pub(crate) fn refresh(&mut self) {
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
            matches!(r.schema_version, 3 | 4) && r.task == "gap" && !r.plan.is_empty(),
            "data_invalid: specified-temperature manifest version or plan invalid"
        );
        for (i, q) in r.plan.iter().enumerate() {
            q.validate()?;
            ensure!(
                q.kind == SolveKind::Gap
                    && !r.plan[..i].iter().any(|p| p.mode == q.mode
                        && p.temperature_centikelvin == q.temperature_centikelvin),
                "data_invalid: duplicate or incorrectly typed specified-temperature plan"
            );
        }
        ensure!(
            r.schema_version == 4 || r.batches.is_empty(),
            "data_invalid: legacy point manifest contains a new batch"
        );
        for (i, b) in r.batches.iter().enumerate() {
            ensure!(
                !b.requests.is_empty()
                    && b.requests.len() <= 50
                    && b.mode != Mode::Both
                    && b.task.group.as_deref()
                        == Some(if b.mode == Mode::Iso { "iso" } else { "aniso" })
                    && b.task.directory.parent() == path.parent()
                    && !r.batches[..i].iter().any(|old| old.task.id == b.task.id),
                "data_invalid: method batch identity or directory conflict"
            );
            b.executable.verify()?;
            for (j, q) in b.requests.iter().enumerate() {
                ensure!(
                    q.mode == b.mode
                        && r.plan.iter().any(|p| same(p, q).unwrap_or(false))
                        && !b.requests[..j]
                            .iter()
                            .any(|p| p.temperature_centikelvin == q.temperature_centikelvin),
                    "data_invalid: batch request duplicated or outside the fixed plan"
                );
            }
        }
        for a in &r.attempts {
            ensure!(
                r.plan.iter().any(|q| same(q, &a.request).unwrap_or(false)),
                "input_conflict: attempt is outside the fixed-temperature plan"
            );
            ensure!(
                a.directory.parent() == path.parent(),
                "data_invalid: attempt directory escapes its root"
            );
            ensure!(
                matches!(
                    a.status.as_str(),
                    "running" | "failed" | "valid" | "completed"
                ),
                "data_invalid: invalid attempt status"
            );
            if let Some(id) = &a.record {
                id.verify()?;
                ensure!(
                    id.path.parent() == Some(a.directory.as_path())
                        && (id.path == a.directory.join("attempt.json")
                            || (r.schema_version == 4
                                && id
                                    .path
                                    .file_name()
                                    .is_some_and(|n| n.to_string_lossy().starts_with("point-")))),
                    "data_invalid: attempt-record path conflict"
                );
                let record = super::verify::record(&id.path)?;
                if let Some(evidence) = &record.batch {
                    ensure!(
                        evidence.log.path.parent() == Some(a.directory.as_path())
                            && evidence
                                .gap_file
                                .as_ref()
                                .is_none_or(|f| f.path.parent() == Some(a.directory.as_path()))
                            && (crate::foundation::layout::enabled(
                                path.parent().unwrap().parent().unwrap()
                            ) || r.batches.iter().any(|b| b.task.directory == a.directory
                                && b.requests
                                    .iter()
                                    .any(|q| same(q, &a.request).unwrap_or(false))
                                && b.requests
                                    .iter()
                                    .map(|q| q.temperature_centikelvin)
                                    .collect::<Vec<_>>()
                                    == evidence.temperatures
                                && b.executable.sha256 == record.executable.sha256)),
                        "input_conflict: point differs from method batch evidence"
                    );
                }
                ensure!(
                    same(&record.request, &a.request)?
                        && record.status == a.status
                        && record.source.interpolation_digest == r.interpolation_digest,
                    "input_conflict: attempt record conflicts with plan/source"
                );
            } else {
                ensure!(
                    !matches!(a.status.as_str(), "valid" | "completed"),
                    "data_invalid: completed point has no native evidence"
                );
            }
        }
        let mut checked = r.clone();
        checked.refresh();
        ensure!(
            checked.execution_complete == r.execution_complete
                && checked.data_complete == r.data_complete,
            "data_invalid: specified-temperature completion status conflict"
        );
        Ok(r)
    }
}
pub(crate) fn binding(cfg: &Config, input: &NativeInputs, launcher: &[String]) -> Result<String> {
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
    if crate::foundation::layout::enabled(work) {
        let f3 = crate::interpolation::direct::fine(cfg)?;
        let source = super::source::bind(work, cfg, &f3)?;
        return super::batch::current(cfg, &source, run_id);
    }
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
        "input_conflict: specified-temperature plan/program/source changed"
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
        "allocation: single-point solving requires a registered allocation"
    );
    let f3_path = crate::foundation::layout::path(work, "interpolation/manifest.json");
    let f3 = crate::interpolation::manifest::InterpolationManifest::load(&f3_path)?;
    ensure!(f3.validation.all_passed(), "data_invalid: invalid interpolation data");
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
            batches: vec![],
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
                    "scheduler_unknown: interrupted point has no confirmed failed native task"
                );
                None
            }
        } else if saved.exists() && store::read::<AttemptResult>(&saved)?.status != "running" {
            Some(super::verify::record(&saved)?)
        } else {
            ensure!(retry, "scheduler_unknown: interrupted attempt requires explicit --resume");
            let job = r.attempts[i]
                .job_id
                .context("scheduler_unknown: interrupted attempt has no original allocation ID")?;
            ensure!(
                crate::foundation::slurm::accounting(job)?.terminal,
                "scheduler_unknown: original allocation remains active"
            );
            None
        };
        if let Some(record) = record {
            r.attempts[i].status = record.status;
            r.attempts[i].reason = record.reason;
            r.attempts[i].record = Some(Identity::capture(&saved)?);
        } else {
            r.attempts[i].status = "failed".into();
            r.attempts[i].reason = Some("execution_failed: native attempt confirmed interrupted".into());
            if saved.exists() {
                let mut record: AttemptResult = store::read(&saved)?;
                ensure!(
                    same(&record.request, &r.attempts[i].request)?,
                    "input_conflict: interrupted point request changed"
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
                "partial: point failed; fix the cause and use --resume; original attempt preserved"
            );
        }
        if !crate::foundation::native_jobs::enabled() {
            let job = std::env::var("SLURM_JOB_ID")
                .context("allocation: current Slurm allocation missing")?
                .parse::<u64>()?;
            crate::foundation::budget::with_ledger(
                &cfg.budget.ledger,
                &cfg.budget.campaign_id,
                |ledger| ledger.require_allocation(job).map(|_| ()),
            )?;
        }
        ensure!(
            store::hash_file(&f3_path)? == source.interpolation_digest,
            "input_conflict: F3 manifest changed"
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
                    "execution_failed: native single-point run exited abnormally; evidence preserved"
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
    ensure!(r.execution_complete, "partial: specified-temperature execution incomplete");
    Ok(())
}
