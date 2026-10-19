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
}

pub struct Engine<'a> {
    cfg: &'a Config,
    source: &'a NativeInputs,
    launcher: &'a [String],
    root: &'a Path,
    pub result: ModeResult,
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
                    && saved.binding_digest == binding_digest,
                "input_conflict: 搜索程序/F3/求解配置身份已变"
            );
            for a in &saved.attempts {
                ensure!(
                    a.directory.parent() == Some(root),
                    "data_invalid: 尝试目录逃逸"
                );
                if let Some(record) = &a.record {
                    record.verify()?;
                }
            }
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
            }
        };
        Ok(Self {
            cfg,
            source,
            launcher,
            root,
            result,
        })
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
            .find(|a| serde_json::to_vec(&a.request).is_ok_and(|s| s == encoded))
        {
            ensure!(
                a.status == "valid",
                "{}: 已有未完成或失败尝试，保留现场；不自动重提",
                if a.status == "running" {
                    "scheduler_unknown"
                } else {
                    "solver_failed"
                }
            );
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

    /// 不重复失败尝试；显式恢复的失败重试策略尚未开放，保持计数和未知状态。
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
