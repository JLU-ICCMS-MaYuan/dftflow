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
        ensure!(mode != Mode::Both, "configuration: 每种模式独立搜索");
        source.verify(mode)?;
        let tc = match mode {
            Mode::Iso => cfg.superconductivity.iso.as_ref(),
            _ => cfg.superconductivity.aniso.as_ref(),
        }
        .context("configuration: 缺少模式搜索配置")?;
        let id = Identity::capture(&cfg.programs.epw.path)?;
        let binding_digest = store::hash_bytes(&serde_json::to_vec(&(
            source,
            &cfg.superconductivity,
            &cfg.programs.epw,
            launcher,
            &id,
        ))?);
        fs::create_dir_all(root)?;
        let file = root.join("search.json");
        let result: ModeResult = if file.exists() {
            let saved: ModeResult = store::read(&file)?;
            ensure!(
                saved.schema_version == 1
                    && saved.mode == mode
                    && saved.binding_digest == binding_digest
                    && serde_json::to_value(&saved.thresholds)? == serde_json::to_value(tc)?
                    && saved.initial_gaps_mev == cfg.superconductivity.initial_gaps_mev
                    && saved.gap_temperatures_centikelvin
                        == cfg
                            .superconductivity
                            .gap_temperatures_k
                            .iter()
                            .map(|&t| config::temperature(t, "gap_temperature"))
                            .collect::<Result<Vec<_>>>()?,
                "input_conflict: 搜索程序/F3/求解配置身份已变"
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
                    "input_conflict: 保存的尝试参数与固定配置不一致"
                );
                ensure!(
                    a.directory.parent() == Some(root),
                    "data_invalid: 尝试目录逃逸"
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
                "input_conflict: 保存的搜索区间与固定配置不一致"
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
                complete: false,
                reason: None,
                gap_temperatures_centikelvin: cfg
                    .superconductivity
                    .gap_temperatures_k
                    .iter()
                    .map(|&t| config::temperature(t, "gap_temperature"))
                    .collect::<Result<_>>()?,
                initial_gaps_mev: cfg.superconductivity.initial_gaps_mev,
            }
        };
        Ok(Self {
            cfg,
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
            let path = entry.directory.join("attempt.json");
            if path.exists() {
                let record: AttemptResult = store::read(&path)?;
                ensure!(
                    serde_json::to_vec(&record.request)? == serde_json::to_vec(&entry.request)?,
                    "input_conflict: 中断尝试身份改变"
                );
                if record.status != "running" {
                    ensure!(
                        matches!(record.status.as_str(), "valid" | "failed"),
                        "data_invalid: 尝试状态非法"
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
                    .context("scheduler_unknown: 中断尝试缺少原分配号，不能重提")?,
            )?;
            entry.status = "failed".into();
            entry.reason = Some("execution_failed: 原分配已终止，原生尝试被中断".into());
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
        self.retry_failed = true;
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        store::write(&self.root.join("search.json"), &self.result)
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
                "{}: 已有未完成或失败尝试，保留现场；不自动重提",
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
                    .context("data_invalid: 有效尝试缺少原生记录")?;
                record.verify()?;
                let r: AttemptResult = store::read(&record.path)?;
                ensure!(
                    r.schema_version == 1
                        && r.status == "valid"
                        && serde_json::to_vec(&r.request)? == encoded,
                    "data_invalid: 尝试状态/身份冲突"
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
            Mode::Iso => self.cfg.superconductivity.iso.as_ref(),
            _ => self.cfg.superconductivity.aniso.as_ref(),
        }
        .unwrap();
        ensure!(
            self.result.attempts.len() < tc.max_solves as usize,
            "attempt_limit: 外部求解次数用尽（含失败及必需能隙复核）"
        );
        before()?;
        let dir = self
            .root
            .join(format!("attempt-{:04}", self.result.attempts.len() + 1));
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
            r.reason.as_deref().unwrap_or("缺少有效原生结果")
        );
        Ok(r)
    }

    fn nonlinear(&mut self, t: i64, before: &mut impl FnMut() -> Result<()>) -> Result<Side> {
        let seeds = self.cfg.superconductivity.initial_gaps_mev;
        let a = self.attempt(t, SolveKind::Nonlinear, Some(seeds[0]), before)?;
        let b = self.attempt(t, SolveKind::Nonlinear, Some(seeds[1]), before)?;
        let tc = match self.result.mode {
            Mode::Iso => self.cfg.superconductivity.iso.as_ref(),
            _ => self.cfg.superconductivity.aniso.as_ref(),
        }
        .unwrap();
        super::aniso::classify(
            a.gap.as_ref().context("data_invalid: 缺少第一初值 Δ/Z")?,
            b.gap.as_ref().context("data_invalid: 缺少第二初值 Δ/Z")?,
            tc.normal_gap_mev,
            tc.super_gap_mev,
        )
    }

    fn linear(&mut self, t: i64, before: &mut impl FnMut() -> Result<()>) -> Result<Side> {
        let r = self.attempt(t, SolveKind::Linear, None, before)?;
        let rho = r
            .evidence
            .context("data_invalid: 缺少原生线性证据")?
            .maximum_eigenvalue
            .context("data_invalid: 缺少本征值")?;
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

    /// 只有 prepare_resume 明确授权后，才在原规则和剩余次数内重试已确认失败项。
    pub fn run(&mut self, before: &mut impl FnMut() -> Result<()>) -> Result<&ModeResult> {
        let outcome = (|| -> Result<()> {
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
                    "classification_uncertain: 必需能隙温度未获得双初值一致的超导态分布"
                );
                let classified = if self.result.mode == Mode::Iso {
                    let linear = self.linear(t, before)?;
                    ensure!(
                        linear == Side::Uncertain
                            || nonlinear == Side::Uncertain
                            || linear == nonlinear,
                        "inconsistent: 同温度线性分类与有效非线性分布矛盾"
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
                    "inconsistent: 必需能隙温度揭示非单调分类"
                );
            }
            self.result.complete = true;
            Ok(())
        })();
        if let Err(e) = outcome {
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
