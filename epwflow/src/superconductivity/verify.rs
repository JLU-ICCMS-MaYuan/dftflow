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
    ensure!(r.schema_version == 1, "unsupported_version: 尝试版本");
    r.request.validate()?;
    r.executable.verify()?;
    r.source.verify(r.request.mode)?;
    for a in &r.artifacts {
        a.verify()?;
    }
    if r.status == "valid" {
        ensure!(r.exit_status == Some(0), "data_invalid: 有效尝试退出失败");
        let root = path.parent().context("data_invalid: 尝试根目录")?;
        let evidence =
            super::parse::solve_log(&std::fs::read_to_string(root.join("epw.out"))?, &r.request)?;
        ensure!(
            serde_json::to_value(&evidence)? == serde_json::to_value(&r.evidence)?,
            "data_invalid: 求解证据与原生日志冲突"
        );
        if r.request.kind == SolveKind::Nonlinear {
            let g = r.gap.as_ref().context("data_invalid: 缺少完整能隙")?;
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
                "data_invalid: 完整能隙与原始输出冲突"
            );
        } else {
            ensure!(r.gap.is_none(), "data_invalid: 线性求解不能产生非线性能隙");
        }
    }
    Ok(r)
}

pub fn mode(result: &ModeResult, interpolation_digest: &str) -> Result<()> {
    ensure!(
        result.schema_version == 1 && result.mode != Mode::Both,
        "data_invalid: 模式版本/身份"
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
        "data_invalid: 搜索区间与固定阈值冲突"
    );
    let mut records = vec![];
    for a in &result.attempts {
        ensure!(a.request.mode == result.mode, "data_invalid: 尝试模式冲突");
        if a.status == "valid" {
            ensure!(a.record.is_some(), "data_invalid: 有效尝试缺少记录");
        }
        if let Some(id) = &a.record {
            id.verify()?;
            ensure!(
                id.path == a.directory.join("attempt.json"),
                "data_invalid: 尝试记录错位"
            );
            let r = record(&id.path)?;
            ensure!(
                serde_json::to_value(&r.request)? == serde_json::to_value(&a.request)?
                    && r.source.interpolation_digest == interpolation_digest,
                "input_conflict: 尝试请求/F3 身份冲突"
            );
            if a.status == "valid" {
                ensure!(r.status == "valid", "data_invalid: 有效状态冲突");
                records.push(r);
            }
        }
    }
    ensure!(
        result.attempts.len() <= result.thresholds.max_solves as usize,
        "data_invalid: 次数超过固定上限"
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
            .context("data_invalid: 分类/必需能隙缺少有效求解证据")
    };
    let nonlinear = |t| -> Result<Side> {
        let a = lookup(t, SolveKind::Nonlinear, Some(result.initial_gaps_mev[0]))?;
        let b = lookup(t, SolveKind::Nonlinear, Some(result.initial_gaps_mev[1]))?;
        super::aniso::classify(
            a.gap.as_ref().context("缺 Δ/Z")?,
            b.gap.as_ref().context("缺 Δ/Z")?,
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
                .context("data_invalid: 缺原生本征值")?,
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
        ensure!(actual == side, "data_invalid: 温度分类与原生证据矛盾");
        replay.observe(t, side)?;
    }
    ensure!(
        replay.interval_centikelvin == s.interval_centikelvin,
        "data_invalid: Tc 区间与端点证据冲突"
    );
    if result.complete || s.status == "located" {
        let [lo, hi] = s.interval_centikelvin.context("data_invalid: 定位缺区间")?;
        ensure!(
            s.status == "located"
                && replay.status != "inconsistent"
                && hi - lo <= s.precision_centikelvin
                && s.points.get(&lo) == Some(&Side::Superconducting)
                && s.points.get(&hi) == Some(&Side::Normal),
            "data_invalid: 无有效定位证据"
        );
        if result.complete {
            ensure!(
                !result.gap_temperatures_centikelvin.is_empty(),
                "data_invalid: 必需能隙计划为空"
            );
            for &t in result
                .gap_temperatures_centikelvin
                .iter()
                .chain(std::iter::once(&lo))
            {
                let n = nonlinear(t)?;
                ensure!(
                    n == Side::Superconducting,
                    "data_invalid: 必需能隙不是有效超导态"
                );
                if result.mode == Mode::Iso {
                    let l = linear(t)?;
                    ensure!(
                        n == Side::Uncertain || l == Side::Uncertain || l == n,
                        "data_invalid: 同温度线性/非线性分类矛盾"
                    );
                }
            }
        }
    }
    Ok(())
}

/// 统一阶段边界完成门：与固定配置和当前 F3 身份联合验证。
pub fn complete(work: &Path, cfg: &Config, run_id: &str) -> Result<Vec<Identity>> {
    let path = work.join("superconductivity/manifest.json");
    let m = super::manifest::SuperconductivityManifest::load(&path)?;
    ensure!(
        m.schema_version == 2
            && m.run_id == run_id
            && m.converged
            && m.located
            && m.validation.all_passed(),
        "data_invalid: 原生超导阶段尚未完成"
    );
    ensure!(
        store::hash_file(&work.join("interpolation/manifest.json"))? == m.interpolation_digest,
        "input_conflict: 当前 F3 改变"
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
        "data_invalid: 所选模式缺失"
    );
    let mut artifacts = m.artifacts.clone();
    ensure!(!artifacts.is_empty(), "data_invalid: 缺少原生交付产物");
    for r in m.native_modes.values() {
        ensure!(r.complete, "data_invalid: 某模式未完成");
        let source = &record(
            &r.attempts
                .iter()
                .find_map(|a| a.record.as_ref())
                .context("缺尝试")?
                .path,
        )?
        .source;
        let root = path
            .parent()
            .unwrap()
            .join(if r.mode == Mode::Iso { "iso" } else { "aniso" });
        let current =
            super::engine::Engine::open(cfg, source, &cfg.programs.launcher, &root, r.mode)?;
        ensure!(
            serde_json::to_value(current.result)? == serde_json::to_value(r)?,
            "input_conflict: 搜索记录与固定配置/交付清单冲突"
        );
        for a in &r.attempts {
            if let Some(id) = &a.record {
                artifacts.push(id.clone());
            }
        }
    }
    Ok(artifacts)
}
