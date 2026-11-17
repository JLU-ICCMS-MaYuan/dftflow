//! 重验原生交付的数值证据；不能只相信 complete 或 validation 布尔值。
use super::{
    attempt::{AttemptResult, SolveKind},
    engine::ModeResult,
    search::{Search, Side},
};
use crate::foundation::{
    config::{Config, Mode},
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use std::path::Path;

pub fn record(path: &Path) -> Result<AttemptResult> {
    let r: AttemptResult = store::read(path)?;
    ensure!(
        matches!(r.schema_version, 1 | 2),
        "unsupported_version: attempt version"
    );
    ensure!(
        (r.schema_version == 2) == r.batch.is_some()
            && (r.batch.is_none() || r.request.kind == SolveKind::Gap),
        "data_invalid: batch evidence version conflict"
    );
    r.request.validate()?;
    r.executable.verify()?;
    r.source.verify(r.request.mode)?;
    for a in &r.artifacts {
        a.verify()?;
    }
    if r.request.kind == SolveKind::Gap && matches!(r.status.as_str(), "valid" | "completed") {
        let dir = path.parent().context("data_invalid: attempt root directory missing")?;
        let (status, evidence, gap) = if r.batch.is_some() {
            super::batch::inspect(&r)?
        } else {
            super::attempt::inspect_gap(&r, dir)?
        };
        ensure!(
            r.gap_status.as_ref() == Some(&status)
                && serde_json::to_value(&r.evidence)? == serde_json::to_value(evidence)?
                && serde_json::to_value(&r.gap)? == serde_json::to_value(&gap)?
                && (r.status == "valid") == gap.is_some(),
            "data_invalid: single-point status conflicts with native evidence"
        );
        return Ok(r);
    }
    if r.status == "valid" {
        ensure!(r.exit_status == Some(0), "data_invalid: valid attempt has a failed exit status");
        let root = path.parent().context("data_invalid: attempt root directory missing")?;
        let evidence =
            super::parse::solve_log(&std::fs::read_to_string(root.join("epw.out"))?, &r.request)?;
        ensure!(
            serde_json::to_value(&evidence)? == serde_json::to_value(&r.evidence)?,
            "data_invalid: solver evidence conflicts with native log"
        );
        if r.request.kind == SolveKind::Nonlinear {
            let g = r.gap.as_ref().context("data_invalid: complete gap data missing")?;
            g.validate()?;
            let name = format!(
                "{}.imag_{}_{:06.2}",
                r.source.prefix,
                if r.request.mode == Mode::Iso {
                    "iso"
                } else {
                    "aniso"
                },
                r.request.temperature_centikelvin as f64 / 100.0
            );
            let parsed = super::gap::parse_native(
                &std::fs::read_to_string(root.join(name))?,
                r.request.temperature_centikelvin,
                evidence.nfreq,
                if r.request.mode == Mode::Iso {
                    vec![]
                } else {
                    r.source.states.clone()
                },
            )?;
            ensure!(
                serde_json::to_value(&parsed)? == serde_json::to_value(g)?,
                "data_invalid: complete gap data conflict with raw output"
            );
        } else {
            ensure!(r.gap.is_none(), "data_invalid: linear solve cannot produce a nonlinear gap");
        }
    }
    Ok(r)
}

pub fn mode(result: &ModeResult, interpolation_digest: &str) -> Result<()> {
    ensure!(
        result.schema_version == 1 && result.mode != Mode::Both,
        "data_invalid: mode version/identity mismatch"
    );
    ensure!(
        result.search.min_centikelvin
            == crate::foundation::config::temperature(result.thresholds.min_k, "min_k")?
            && result.search.max_centikelvin
                == crate::foundation::config::temperature(result.thresholds.max_k, "max_k")?
            && result.search.precision_centikelvin
                == crate::foundation::config::temperature(
                    result.thresholds.precision_k,
                    "precision_k"
                )?,
        "data_invalid: search interval conflicts with fixed thresholds"
    );
    let mut records = vec![];
    for a in &result.attempts {
        ensure!(a.request.mode == result.mode, "data_invalid: attempt mode conflict");
        if a.status == "valid" {
            ensure!(a.record.is_some(), "data_invalid: valid attempt record missing");
        }
        if let Some(id) = &a.record {
            id.verify()?;
            ensure!(
                id.path == a.directory.join("attempt.json"),
                "data_invalid: attempt-record alignment mismatch"
            );
            let r = record(&id.path)?;
            ensure!(
                serde_json::to_value(&r.request)? == serde_json::to_value(&a.request)?
                    && r.source.interpolation_digest == interpolation_digest,
                "input_conflict: attempt request/F3 identity conflict"
            );
            if a.status == "valid" {
                ensure!(r.status == "valid", "data_invalid: valid-state conflict");
                records.push(r);
            }
        }
    }
    ensure!(
        result.attempts.len() <= result.thresholds.max_solves as usize,
        "data_invalid: solve count exceeds the fixed limit"
    );
    let lookup = |t: i64, kind: SolveKind, seed: Option<f64>| -> Result<&AttemptResult> {
        records
            .iter()
            .rev()
            .find(|r| {
                r.request.temperature_centikelvin == t
                    && r.request.kind == kind
                    && r.request.seed_mev == seed
            })
            .context("data_invalid: classification/mandatory gap lacks valid solver evidence")
    };
    let nonlinear = |t| -> Result<Side> {
        let a = lookup(t, SolveKind::Nonlinear, Some(result.initial_gaps_mev[0]))?;
        let b = lookup(t, SolveKind::Nonlinear, Some(result.initial_gaps_mev[1]))?;
        super::aniso::classify(
            a.gap.as_ref().context("Delta/Z missing")?,
            b.gap.as_ref().context("Delta/Z missing")?,
            result.thresholds.normal_gap_mev,
            result.thresholds.super_gap_mev,
        )
    };
    let linear = |t| -> Result<Side> {
        let r = lookup(t, SolveKind::Linear, None)?;
        super::iso::classify(
            r.evidence
                .as_ref()
                .and_then(|e| e.maximum_eigenvalue)
                .context("data_invalid: native eigenvalue missing")?,
            result.thresholds.eigen_margin,
        )
    };
    let s = &result.search;
    let mut replay = Search::new(
        s.min_centikelvin,
        s.max_centikelvin,
        s.precision_centikelvin,
    )?;
    for (&t, &side) in &s.points {
        let actual = if result.mode == Mode::Iso {
            linear(t)?
        } else {
            nonlinear(t)?
        };
        ensure!(actual == side, "data_invalid: temperature classification contradicts native evidence");
        replay.observe(t, side)?;
    }
    ensure!(
        replay.interval_centikelvin == s.interval_centikelvin,
        "data_invalid: Tc interval conflicts with endpoint evidence"
    );
    if result.complete || s.status == "located" {
        let [lo, hi] = s.interval_centikelvin.context("data_invalid: located status has no interval")?;
        ensure!(
            s.status == "located"
                && replay.status != "inconsistent"
                && hi - lo <= s.precision_centikelvin
                && s.points.get(&lo) == Some(&Side::Superconducting)
                && s.points.get(&hi) == Some(&Side::Normal),
            "data_invalid: valid interval-location evidence missing"
        );
        if result.complete {
            ensure!(
                !result.gap_temperatures_centikelvin.is_empty(),
                "data_invalid: mandatory gap plan empty"
            );
            for &t in result
                .gap_temperatures_centikelvin
                .iter()
                .chain(std::iter::once(&lo))
            {
                let n = nonlinear(t)?;
                ensure!(
                    n == Side::Superconducting,
                    "data_invalid: mandatory gap is not a valid superconducting state"
                );
                if result.mode == Mode::Iso {
                    let l = linear(t)?;
                    ensure!(
                        n == Side::Uncertain || l == Side::Uncertain || l == n,
                        "data_invalid: linear/nonlinear classifications disagree at the same temperature"
                    );
                }
            }
        }
    }
    Ok(())
}

/// 统一阶段边界完成门：与固定配置和当前 F3 身份联合验证。
pub fn complete(work: &Path, cfg: &Config, run_id: &str) -> Result<Vec<Identity>> {
    if cfg.superconductivity.task == crate::foundation::config::SuperconductivityTask::Gap {
        let r = super::points::published(work, cfg, run_id)?;
        ensure!(r.execution_complete, "partial: specified-temperature execution incomplete");
        return Ok(r.attempts.into_iter().filter_map(|a| a.record).collect());
    }
    let (m, artifacts) = published(work, cfg, run_id)?;
    ensure!(
        m.converged && m.located && m.validation.all_passed(),
        "data_invalid: native superconductivity stage incomplete"
    );
    Ok(artifacts)
}

/// 重验已发布证据；false 表示合法 partial，可交回原有 Engine 显式恢复。
/// 损坏、来源或配置冲突仍返回错误，检查不创建缺失的搜索状态。
pub fn published_complete(work: &Path, cfg: &Config, run_id: &str) -> Result<bool> {
    if cfg.superconductivity.task == crate::foundation::config::SuperconductivityTask::Gap {
        return Ok(super::points::published(work, cfg, run_id)?.execution_complete);
    }
    let (m, _) = published(work, cfg, run_id)?;
    Ok(m.converged)
}

fn published(
    work: &Path,
    cfg: &Config,
    run_id: &str,
) -> Result<(super::manifest::SuperconductivityManifest, Vec<Identity>)> {
    let path = crate::foundation::layout::path(&work, "superconductivity/manifest.json");
    let m = super::manifest::SuperconductivityManifest::load(&path)?;
    ensure!(
        m.schema_version == 2 && m.run_id == run_id,
        "input_conflict: native superconductivity version/run identity mismatch"
    );
    ensure!(
        !m.converged || (m.located && m.validation.all_passed()),
        "data_invalid: native superconductivity completion status conflicts with validation"
    );
    ensure!(
        (if crate::foundation::layout::enabled(work) {
            let f3 = crate::interpolation::direct::fine(cfg)?;
            super::source::bind(work, cfg, &f3)?.interpolation_digest
        } else {
            store::hash_file(&crate::foundation::layout::path(
                work,
                "interpolation/manifest.json",
            ))?
        }) == m.interpolation_digest,
        "input_conflict: current F3 changed"
    );
    let expected = match cfg.superconductivity.mode {
        Mode::Iso => vec!["iso"],
        Mode::Aniso => vec!["aniso"],
        Mode::Both => vec!["aniso", "iso"],
    };
    ensure!(
        m.native_modes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            == expected,
        "data_invalid: selected mode missing"
    );
    let mut artifacts = m.artifacts.clone();
    ensure!(!artifacts.is_empty(), "data_invalid: native delivery artifacts missing");
    for r in m.native_modes.values() {
        let source = if let Some(id) = r.attempts.iter().find_map(|a| a.record.as_ref()) {
            record(&id.path)?.source
        } else {
            // 前置检查错误也可留下零尝试模式，来源仍须绑定当前原生 F3。
            let f3 = crate::interpolation::manifest::InterpolationManifest::load(
                &crate::foundation::layout::path(&work, "interpolation/manifest.json"),
            )?;
            super::source::bind(work, cfg, &f3)?
        };
        let root = path
            .parent()
            .unwrap()
            .join(if r.mode == Mode::Iso { "iso" } else { "aniso" });
        ensure!(
            crate::foundation::layout::enabled(work) || root.join("search.json").is_file(),
            "data_invalid: published search record missing"
        );
        source.verify(r.mode)?;
        if root.join("search.json").is_file() {
            let saved:super::engine::ModeResult=store::read(&root.join("search.json"))?;
            ensure!(serde_json::to_value(saved)?==serde_json::to_value(r)?,"input_conflict: historical search result differs from delivery");
        }
        for a in &r.attempts {
            if let Some(id) = &a.record {
                artifacts.push(id.clone());
            }
        }
    }
    Ok((m, artifacts))
}
