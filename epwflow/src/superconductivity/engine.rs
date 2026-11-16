//! 持久化的原生尝试计数、复用与两模式独立搜索。
//! 不申请资源；每次新调用由 F1 检查同一已登记分配。
use super::{
    attempt::{self, AttemptResult, NativeInputs, Request, SolveKind},
    search::{Search, Side},
};
use crate::foundation::{
    config::{self, Config, Mode},
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub request: Request,
    pub directory: std::path::PathBuf,
    pub record: Option<Identity>,
    pub status: String,
    pub reason: Option<String>,
    #[serde(default)]
    pub job_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeResult {
    pub schema_version: u32,
    pub mode: Mode,
    pub binding_digest: String,
    pub method: String,
    pub thresholds: config::Tc,
    pub search: Search,
    pub attempts: Vec<Entry>,
    #[serde(default)]
    pub solve_count: u32,
    pub complete: bool,
    pub reason: Option<String>,
    #[serde(default)]
    pub gap_temperatures_centikelvin: Vec<i64>,
    #[serde(default = "default_seeds")]
    pub initial_gaps_mev: [f64; 2],
}

fn default_seeds() -> [f64; 2] {
    [1.0, 10.0]
}

pub struct Engine<'a> {
    cfg: &'a Config,
    settings: config::Superconductivity,
    source: &'a NativeInputs,
    launcher: &'a [String],
    root: &'a Path,
    pub result: ModeResult,
    retry_failed: bool,
}

impl<'a> Engine<'a> {
    pub fn open(
        cfg: &'a Config,
        source: &'a NativeInputs,
        launcher: &'a [String],
        root: &'a Path,
        mode: Mode,
    ) -> Result<Self> {
        super::solver_config::SolverConfig::from_config(cfg)?;
        ensure!(mode != Mode::Both, "configuration: each mode requires an independent search");
        source.verify(mode)?;
        let settings = cfg.solver_for(mode)?;
        let tc = match mode {
            Mode::Iso => settings.iso.as_ref(),
            _ => settings.aniso.as_ref(),
        }
        .context("configuration: mode search configuration missing")?;
        let id = Identity::capture(&cfg.programs.epw.path)?;
        let binding_digest = if crate::foundation::layout::enabled(&cfg.work_dir) {
            store::hash_bytes(&serde_json::to_vec(&(
                crate::foundation::stages::scientific_key(
                    cfg,
                    crate::foundation::stages::Step::Superconductivity,
                )?,
                source,
                &id,
            ))?)
        } else {
            store::hash_bytes(&serde_json::to_vec(&(
                source,
                &settings,
                &cfg.programs.epw,
                launcher,
                &id,
            ))?)
        };
        fs::create_dir_all(root)?;
        let file = root.join("search.json");
        let mut result: ModeResult =
            if !crate::foundation::layout::enabled(&cfg.work_dir) && file.exists() {
                let saved: ModeResult = store::read(&file)?;
                ensure!(
                    saved.schema_version == 1
                        && saved.mode == mode
                        && saved.binding_digest == binding_digest
                        && serde_json::to_value(&saved.thresholds)? == serde_json::to_value(tc)?
                        && saved.initial_gaps_mev == settings.initial_gaps_mev
                        && saved.gap_temperatures_centikelvin
                            == settings
                                .gap_temperatures_k
                                .iter()
                                .map(|&t| config::temperature(t, "gap_temperature"))
                                .collect::<Result<Vec<_>>>()?,
                    "input_conflict: search program/F3/solver configuration identity changed"
                );
                for a in &saved.attempts {
                    let expected = Request::from_config(
                        cfg,
                        mode,
                        a.request.kind,
                        a.request.temperature_centikelvin,
                        a.request.seed_mev,
                    )?;
                    ensure!(
                        serde_json::to_value(&expected)? == serde_json::to_value(&a.request)?,
                        "input_conflict: saved attempt parameters differ from the fixed configuration"
                    );
                    ensure!(
                        a.directory.parent() == Some(root),
                        "data_invalid: attempt directory escapes its root"
                    );
                    if let Some(record) = &a.record {
                        record.verify()?;
                    }
                }
                ensure!(
                    saved.search.min_centikelvin == config::temperature(tc.min_k, "min_k")?
                        && saved.search.max_centikelvin == config::temperature(tc.max_k, "max_k")?
                        && saved.search.precision_centikelvin
                            == config::temperature(tc.precision_k, "precision_k")?,
                    "input_conflict: saved search interval differs from the fixed configuration"
                );
                saved
            } else {
                ModeResult {
                    schema_version: 1,
                    mode,
                    binding_digest,
                    method: if mode == Mode::Iso {
                        "epw5.9_iso_lapack"
                    } else {
                        "epw5.9_aniso_nonlinear_dual_seed"
                    }
                    .into(),
                    thresholds: tc.clone(),
                    search: Search::new(
                        config::temperature(tc.min_k, "min_k")?,
                        config::temperature(tc.max_k, "max_k")?,
                        config::temperature(tc.precision_k, "precision_k")?,
                    )?,
                    attempts: vec![],
                    solve_count: 0,
                    complete: false,
                    reason: None,
                    gap_temperatures_centikelvin: settings
                        .gap_temperatures_k
                        .iter()
                        .map(|&t| config::temperature(t, "gap_temperature"))
                        .collect::<Result<_>>()?,
                    initial_gaps_mev: settings.initial_gaps_mev,
                }
            };
        if crate::foundation::layout::enabled(&cfg.work_dir) {
            load_log(cfg, &mut result)?;
        }
        Ok(Self {
            cfg,
            settings,
            source,
            launcher,
            root,
            result,
            retry_failed: false,
        })
    }

    /// 仅显式 resume 调用。未知进程须由原分配的终态证据解除，原目录与次数不变。
    pub fn prepare_resume(&mut self, terminal: &mut impl FnMut(u64) -> Result<()>) -> Result<()> {
        for entry in &mut self.result.attempts {
            if entry.status != "running" {
                continue;
            }
            if crate::foundation::native_jobs::enabled() {
                if crate::foundation::native_jobs::continuing(&entry.directory, "epw")? {
                    let record = attempt::run_attempt(
                        &entry.request,
                        self.source,
                        &self.cfg.programs.epw,
                        self.launcher,
                        &entry.directory,
                    )?;
                    entry.status = record.status;
                    entry.reason = record.reason;
                    entry.record = Some(Identity::capture(&entry.directory.join("attempt.json"))?);
                } else {
                    entry.status = "failed".into();
                    entry.reason = Some("execution_failed: native job confirmed failed".into());
                }
                continue;
            }
            let path = entry.directory.join("attempt.json");
            if path.exists() {
                let record: AttemptResult = store::read(&path)?;
                ensure!(
                    serde_json::to_vec(&record.request)? == serde_json::to_vec(&entry.request)?,
                    "input_conflict: interrupted attempt identity changed"
                );
                if record.status != "running" {
                    ensure!(
                        matches!(record.status.as_str(), "valid" | "failed"),
                        "data_invalid: invalid attempt status"
                    );
                    record.executable.verify()?;
                    record.source.verify(record.request.mode)?;
                    for a in &record.artifacts {
                        a.verify()?;
                    }
                    entry.status = record.status;
                    entry.reason = record.reason;
                    entry.record = Some(Identity::capture(&path)?);
                    continue;
                }
            }
            terminal(
                entry
                    .job_id
                    .context("scheduler_unknown: interrupted attempt has no original allocation ID; resubmission prohibited")?,
            )?;
            entry.status = "failed".into();
            entry.reason = Some("execution_failed: original allocation terminated; native attempt interrupted".into());
            if path.exists() {
                entry.record = Some(Identity::capture(&path)?);
            }
        }
        if matches!(
            self.result.search.status.as_str(),
            "solver_failed" | "scheduler_unknown" | "resource_limit"
        ) {
            let old = &self.result.search;
            let mut restored = Search::new(
                old.min_centikelvin,
                old.max_centikelvin,
                old.precision_centikelvin,
            )?;
            for (&t, &side) in &old.points {
                restored.observe(t, side)?;
            }
            self.result.search = restored;
        }
        self.result.reason = None;
        self.retry_failed = !crate::foundation::native_jobs::enabled()
            || crate::foundation::native_jobs::retry_enabled();
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        if crate::foundation::layout::enabled(&self.cfg.work_dir) {
            save_log(self.cfg, &self.result)
        } else {
            store::write(&self.root.join("search.json"), &self.result)
        }
    }

    pub fn attempt(
        &mut self,
        t: i64,
        kind: SolveKind,
        seed: Option<f64>,
        before: &mut impl FnMut() -> Result<()>,
    ) -> Result<AttemptResult> {
        let request = Request::from_config(self.cfg, self.result.mode, kind, t, seed)?;
        let encoded = serde_json::to_vec(&request)?;
        if let Some(a) = self
            .result
            .attempts
            .iter()
            .rev()
            .find(|a| serde_json::to_vec(&a.request).is_ok_and(|s| s == encoded))
        {
            ensure!(
                a.status == "valid" || (a.status == "failed" && self.retry_failed),
                "{}: incomplete or failed attempt exists; data preserved; no automatic resubmission",
                if a.status == "running" {
                    "scheduler_unknown"
                } else {
                    "solver_failed"
                }
            );
            if a.status == "valid" {
                let record = a
                    .record
                    .as_ref()
                    .context("data_invalid: valid attempt has no native record")?;
                record.verify()?;
                let r: AttemptResult = store::read(&record.path)?;
                ensure!(
                    r.schema_version == 1
                        && r.status == "valid"
                        && serde_json::to_vec(&r.request)? == encoded,
                    "data_invalid: attempt status/identity conflict"
                );
                r.executable.verify()?;
                r.source.verify(r.request.mode)?;
                for x in &r.artifacts {
                    x.verify()?;
                }
                if let Some(g) = &r.gap {
                    g.validate()?;
                }
                return Ok(r);
            }
        }
        let tc = match self.result.mode {
            Mode::Iso => self.settings.iso.as_ref(),
            _ => self.settings.aniso.as_ref(),
        }
        .unwrap();
        let numbered = crate::foundation::layout::enabled(&self.cfg.work_dir);
        let used = if numbered {
            self.result.solve_count as usize
        } else {
            self.result.attempts.len()
        };
        ensure!(
            used < tc.max_solves as usize,
            "attempt_limit: external solver-call limit exhausted, including failures and mandatory gap checks"
        );
        before()?;
        let dir = if numbered {
            let kind = match kind {
                SolveKind::Linear => "linear",
                SolveKind::Nonlinear => "nonlinear",
                SolveKind::Gap => "gap",
            };
            let name = format!(
                "{kind}-{:.2}K-seed{}",
                t as f64 / 100.,
                seed.map(|s| s.to_string()).unwrap_or_else(|| "none".into())
            );
            let dir = self.root.join(name);
            if let Some(index) = self
                .result
                .attempts
                .iter()
                .position(|a| a.directory == dir && a.status == "failed")
            {
                self.result.attempts.remove(index);
                if dir.exists() {
                    fs::remove_dir_all(&dir)?;
                }
            }
            dir
        } else {
            self.root
                .join(format!("attempt-{:04}", self.result.attempts.len() + 1))
        };
        self.result.solve_count += 1;
        self.result.attempts.push(Entry {
            request: request.clone(),
            directory: dir.clone(),
            record: None,
            status: "running".into(),
            reason: None,
            job_id: std::env::var("SLURM_JOB_ID")
                .ok()
                .map(|s| s.parse())
                .transpose()?,
        });
        self.save()?; // 次数在启动前落盘；异常窗口保留为未知。
        let run = attempt::run_attempt(
            &request,
            self.source,
            &self.cfg.programs.epw,
            self.launcher,
            &dir,
        );
        let entry = self.result.attempts.last_mut().unwrap();
        match &run {
            Ok(r) => {
                entry.status = r.status.clone();
                entry.reason = r.reason.clone();
                entry.record = Some(Identity::capture(&dir.join("attempt.json"))?);
            }
            Err(e) if crate::foundation::native_jobs::paused(e) => (),
            Err(e) => {
                entry.status = "failed".into();
                entry.reason = Some(format!("{e:#}"));
            }
        }
        self.save()?;
        let r = run?;
        ensure!(
            r.status == "valid",
            "solver_failed: {}",
            r.reason.as_deref().unwrap_or("Valid native results missing")
        );
        Ok(r)
    }

    fn nonlinear(&mut self, t: i64, before: &mut impl FnMut() -> Result<()>) -> Result<Side> {
        let seeds = self.settings.initial_gaps_mev;
        let a = self.attempt(t, SolveKind::Nonlinear, Some(seeds[0]), before)?;
        let b = self.attempt(t, SolveKind::Nonlinear, Some(seeds[1]), before)?;
        let tc = match self.result.mode {
            Mode::Iso => self.settings.iso.as_ref(),
            _ => self.settings.aniso.as_ref(),
        }
        .unwrap();
        super::aniso::classify(
            a.gap.as_ref().context("data_invalid: Delta/Z missing for the first initial gap")?,
            b.gap.as_ref().context("data_invalid: Delta/Z missing for the second initial gap")?,
            tc.normal_gap_mev,
            tc.super_gap_mev,
        )
    }

    fn linear(&mut self, t: i64, before: &mut impl FnMut() -> Result<()>) -> Result<Side> {
        let r = self.attempt(t, SolveKind::Linear, None, before)?;
        let rho = r
            .evidence
            .context("data_invalid: native linear-solver evidence missing")?
            .maximum_eigenvalue
            .context("data_invalid: eigenvalue missing")?;
        super::iso::classify(
            rho,
            self.cfg
                .superconductivity
                .iso
                .as_ref()
                .unwrap()
                .eigen_margin,
        )
    }

    /// 先验证搜索上端点，及早发现正常侧求解不收敛。所有调用仍计入同一尝试账本。
    pub fn probe_upper_endpoint(
        &mut self,
        before: &mut impl FnMut() -> Result<()>,
    ) -> Result<Side> {
        let t = self.result.search.max_centikelvin;
        if let Some(side) = self.result.search.points.get(&t) {
            return Ok(*side);
        }
        ensure!(
            matches!(self.result.search.status.as_str(), "pending" | "running"),
            "input_conflict: explicitly resume the stopped search"
        );
        let side = if self.result.mode == Mode::Iso {
            self.linear(t, before)?
        } else {
            self.nonlinear(t, before)?
        };
        self.result.search.observe(t, side)?;
        self.save()?;
        Ok(side)
    }

    /// 只有 prepare_resume 明确授权后，才在原规则和剩余次数内重试已确认失败项。
    pub fn run(&mut self, before: &mut impl FnMut() -> Result<()>) -> Result<&ModeResult> {
        let outcome = (|| -> Result<()> {
            if matches!(self.result.search.status.as_str(), "pending" | "running") {
                self.probe_upper_endpoint(before)?;
            }
            while let Some(t) = self.result.search.next_temperature() {
                let side = if self.result.mode == Mode::Iso {
                    self.linear(t, before)?
                } else {
                    self.nonlinear(t, before)?
                };
                self.result.search.observe(t, side)?;
                self.save()?;
            }
            self.save()?;
            if self.result.search.status != "located" {
                return Ok(());
            }
            let mut required = self
                .cfg
                .superconductivity
                .gap_temperatures_k
                .iter()
                .map(|t| config::temperature(*t, "gap_temperature"))
                .collect::<Result<Vec<_>>>()?;
            required.push(self.result.search.interval_centikelvin.unwrap()[0]);
            required.sort();
            required.dedup();
            for t in required {
                let nonlinear = self.nonlinear(t, before)?;
                ensure!(
                    nonlinear == Side::Superconducting,
                    "classification_uncertain: mandatory gap temperature lacks a superconducting distribution consistent across both initial gaps"
                );
                let classified = if self.result.mode == Mode::Iso {
                    let linear = self.linear(t, before)?;
                    ensure!(
                        linear == Side::Uncertain
                            || nonlinear == Side::Uncertain
                            || linear == nonlinear,
                        "inconsistent: linear classification contradicts the valid nonlinear distribution at the same temperature"
                    );
                    linear
                } else {
                    nonlinear
                };
                if !self.result.search.points.contains_key(&t) {
                    self.result.search.observe(t, classified)?;
                }
                ensure!(
                    self.result.search.status != "inconsistent",
                    "inconsistent: mandatory gap temperatures reveal nonmonotonic classifications"
                );
            }
            self.result.complete = true;
            Ok(())
        })();
        if let Err(e) = outcome {
            if crate::foundation::native_jobs::paused(&e) {
                self.save()?;
                return Err(e);
            }
            let reason = format!("{e:#}");
            let state = if reason.contains("scheduler_unknown:") {
                "scheduler_unknown"
            } else if reason.contains("attempt_limit:") {
                "attempt_limit"
            } else if reason.contains("budget_denied:") || reason.contains("allocation:") {
                "resource_limit"
            } else if reason.contains("classification_uncertain:") {
                "classification_uncertain"
            } else if reason.contains("inconsistent:") {
                "inconsistent"
            } else {
                "solver_failed"
            };
            if self.result.search.status != "located" {
                self.result.search.stop(state, &reason);
            }
            self.result.reason = Some(reason);
            self.result.complete = false;
        }
        self.save()?;
        Ok(&self.result)
    }
}

fn mode_name(mode: Mode) -> &'static str {
    if mode == Mode::Iso {
        "iso"
    } else {
        "aniso"
    }
}
const LOG_STATES: &[(&str, &str)] = &[
    ("pending", "not started"),
    ("running", "running"),
    ("located", "interval located"),
    ("not_bracketed", "no valid interval established"),
    ("classification_uncertain", "classification undetermined"),
    ("inconsistent", "inconsistent criteria"),
    ("scheduler_unknown", "scheduler status unknown"),
    ("attempt_limit", "solver call limit exhausted"),
    ("resource_limit", "resource conditions unmet"),
    ("solver_failed", "solver failed"),
    ("valid", "scientific data valid"),
    ("completed", "exited; data validation pending"),
    ("failed", "failed"),
];
fn log_status(status: &str) -> &str {
    LOG_STATES
        .iter()
        .find(|(code, _)| *code == status)
        .map_or(status, |(_, text)| *text)
}
fn status_code(status: &str) -> &str {
    LOG_STATES
        .iter()
        .find(|(_, text)| *text == status)
        .map_or(status, |(code, _)| *code)
}
fn load_log(cfg: &Config, result: &mut ModeResult) -> Result<()> {
    use crate::foundation::{readable_state as log, stages::Step};
    let doc = log::read(&cfg.work_dir, Step::Superconductivity)?;
    let label = format!("Tc search · {}", mode_name(result.mode));
    let Some(status) = doc.get(&label, "Status") else {
        return Ok(());
    };
    if doc.get(&label, "Owning batch") != doc.get("Stage", "Batch") {
        return Ok(());
    }
    result.search.status = status_code(status).into();
    result.search.reason = doc.get(&label, "Reason").map(str::to_owned);
    result.reason = doc.get(&label, "Stop reason").map(str::to_owned);
    result.complete = doc.get(&label, "completed") == Some("yes");
    result.solve_count = doc
        .get(&label, "External solver calls")
        .map(str::parse)
        .transpose()?
        .unwrap_or(0);
    if let (Some(a), Some(b)) = (doc.get(&label, "Interval lower bound/K"), doc.get(&label, "Interval upper bound/K"))
    {
        result.search.interval_centikelvin = Some([
            config::temperature(a.parse()?, "Tc lower")?,
            config::temperature(b.parse()?, "Tc upper")?,
        ]);
    }
    let tasks = crate::foundation::native_jobs::tasks(cfg)?;
    for (name, fields) in &doc.sections {
        if name.starts_with(&format!("Tc criterion · {} · ", mode_name(result.mode))) {
            let t = config::temperature(
                fields
                    .get("Temperature/K")
                    .context("data_invalid: Tc criterion has no temperature")?
                    .parse()?,
                "Tc point",
            )?;
            let side = match fields.get("Criterion").map(String::as_str) {
                Some("superconducting side") => Side::Superconducting,
                Some("normal side") => Side::Normal,
                _ => Side::Uncertain,
            };
            result.search.points.insert(t, side);
        }
        if name.starts_with(&format!("Tc solve · {} · ", mode_name(result.mode))) {
            let field = |k: &str| {
                fields
                    .get(k)
                    .with_context(|| format!("data_invalid: Tc solve missing {k}"))
            };
            let kind = match field("Type")?.as_str() {
                "linear" => SolveKind::Linear,
                "gap" => SolveKind::Gap,
                _ => SolveKind::Nonlinear,
            };
            let t = config::temperature(field("Temperature/K")?.parse()?, "Tc solve")?;
            let seed = fields.get("Initial gap/meV").map(|s| s.parse()).transpose()?;
            let directory = log::resolve(&cfg.work_dir, field("Calculation directory")?);
            let job_id = tasks
                .iter()
                .find(|task| task.directory == directory && task.stem == "epw")
                .and_then(|task| task.job);
            let record = fields
                .get("Scientific result")
                .map(|p| log::resolve(&cfg.work_dir, p))
                .filter(|p| p.is_file())
                .map(|p| Identity::capture(&p))
                .transpose()?;
            result.attempts.push(Entry {
                request: Request::from_config(cfg, result.mode, kind, t, seed)?,
                directory,
                record,
                status: status_code(field("Execution status")?).into(),
                reason: fields.get("Reason").cloned(),
                job_id,
            });
        }
    }
    Ok(())
}
fn save_log(cfg: &Config, result: &ModeResult) -> Result<()> {
    use crate::foundation::{readable_state as log, stages::Step};
    log::update(&cfg.work_dir, Step::Superconductivity, |doc| {
        let label = format!("Tc search · {}", mode_name(result.mode));
        let batch = doc.get("Stage", "Batch").unwrap_or("existing").to_owned();
        doc.remove(&label);
        doc.set(&label, "Owning batch", batch);
        doc.set(&label, "External solver calls", result.solve_count);
        doc.set(&label, "Status", log_status(&result.search.status));
        doc.set(&label, "completed", if result.complete { "yes" } else { "no" });
        if let Some(reason) = &result.search.reason {
            doc.set(&label, "Reason", reason);
        }
        if let Some(reason) = &result.reason {
            doc.set(&label, "Stop reason", reason);
        }
        if let Some([a, b]) = result.search.interval_k() {
            doc.set(&label, "Interval lower bound/K", a);
            doc.set(&label, "Interval upper bound/K", b);
        }
        for (t, side) in &result.search.points {
            let label = format!(
                "Tc criterion · {} · {:.2} K",
                mode_name(result.mode),
                *t as f64 / 100.
            );
            doc.set(&label, "Temperature/K", *t as f64 / 100.);
            doc.set(
                &label,
                "Criterion",
                match side {
                    Side::Superconducting => "superconducting side",
                    Side::Normal => "normal side",
                    Side::Uncertain => "undetermined",
                },
            );
        }
        let prefix = format!("Tc solve · {} · ", mode_name(result.mode));
        doc.sections.retain(|name, _| !name.starts_with(&prefix));
        for (i, entry) in result.attempts.iter().enumerate() {
            let label = format!("Tc solve · {} · {i:04}", mode_name(result.mode));
            doc.remove(&label);
            doc.set(
                &label,
                "Temperature/K",
                entry.request.temperature_centikelvin as f64 / 100.,
            );
            doc.set(
                &label,
                "Type",
                match entry.request.kind {
                    SolveKind::Linear => "linear",
                    SolveKind::Nonlinear => "nonlinear",
                    SolveKind::Gap => "gap",
                },
            );
            if let Some(seed) = entry.request.seed_mev {
                doc.set(&label, "Initial gap/meV", seed);
            }
            doc.set(
                &label,
                "Calculation directory",
                log::relative(&cfg.work_dir, &entry.directory),
            );
            doc.set(&label, "Execution status", log_status(&entry.status));
            if let Some(record) = &entry.record {
                doc.set(
                    &label,
                    "Scientific result",
                    log::relative(&cfg.work_dir, &record.path),
                );
            }
            if let Some(reason) = &entry.reason {
                doc.set(&label, "Reason", reason);
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod readable_tests {
    use super::*;
    use crate::foundation::{readable_state as log, stages::Step};

    #[test]
    fn tc_resume_reads_chinese_progress_and_job_id_only_from_jobs() {
        let root = std::env::temp_dir().join(format!("epw-tc-log-{}", store::token()));
        let mut cfg: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.work_dir = root.clone();
        cfg.layout = Some("numbered".into());
        log::initialize(&cfg).unwrap();
        let directory = Step::Superconductivity
            .directory(&cfg)
            .join("iso/linear-20K");
        let task_label = format!("Task · {} / epw", log::relative(&root, &directory));
        log::update(&root, Step::Superconductivity, |doc| {
            doc.set("Stage", "Batch", "test-batch");
            for (key, value) in [
                ("Submission ID", "test-task"),
                ("Batch", "test-batch"),
                ("Status", "running"),
                ("Calculation directory", "8.epw-superconductivity/iso/linear-20K"),
                ("Script", "8.epw-superconductivity/iso/linear-20K/epw.sh"),
                ("Output name", "epw"),
                ("Run command", "epw.x -inp epw.in"),
                ("Duration seconds", "0"),
            ] {
                doc.set(&task_label, key, value);
            }
            Ok(())
        })
        .unwrap();
        log::set_job(&root, Step::Superconductivity, &task_label, Some(42)).unwrap();
        let thresholds = cfg.superconductivity.iso.clone().unwrap();
        let mut search = Search::new(500, 5000, 50).unwrap();
        search.status = "running".into();
        let result = ModeResult {
            schema_version: 1,
            mode: Mode::Iso,
            binding_digest: String::new(),
            method: "iso".into(),
            thresholds,
            search,
            attempts: vec![Entry {
                request: Request::from_config(&cfg, Mode::Iso, SolveKind::Linear, 2000, None)
                    .unwrap(),
                directory,
                record: None,
                status: "running".into(),
                reason: None,
                job_id: Some(999),
            }],
            solve_count: 3,
            complete: false,
            reason: None,
            gap_temperatures_centikelvin: vec![],
            initial_gaps_mev: default_seeds(),
        };
        save_log(&cfg, &result).unwrap();
        let text = fs::read_to_string(log::path(&root, Step::Superconductivity).unwrap()).unwrap();
        assert!(text.contains("running") && !text.contains("Allocation job ID"));
        let mut loaded = result.clone();
        loaded.attempts.clear();
        loaded.solve_count = 0;
        loaded.search.status = "pending".into();
        load_log(&cfg, &mut loaded).unwrap();
        assert_eq!(loaded.search.status, "running");
        assert_eq!(loaded.solve_count, 3);
        assert_eq!(loaded.attempts.len(), 1);
        assert_eq!(loaded.attempts[0].status, "running");
        assert_eq!(loaded.attempts[0].job_id, Some(42));
        fs::remove_dir_all(root).unwrap();
    }
}
