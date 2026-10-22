//! F3 来源重验后，在同一 F1 分配内执行原生两模式搜索。
use super::{engine::Engine, manifest::SuperconductivityManifest, source};
use crate::{
    foundation::{
        budget,
        config::{self, Mode},
        stage::Backend,
        store,
    },
    interpolation::manifest::InterpolationManifest,
};
use anyhow::{bail, ensure, Context, Result};
use std::{collections::BTreeMap, path::Path};

pub fn run(
    cfg: &config::Config,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<SuperconductivityManifest> {
    run_with_retry(cfg, work, launcher, backend, false)
}

pub fn run_with_retry(
    cfg: &config::Config,
    work: &Path,
    launcher: &[String],
    backend: Backend,
    retry: bool,
) -> Result<SuperconductivityManifest> {
    let work = std::fs::canonicalize(work)?;
    let path = crate::foundation::layout::path(&work, "interpolation/manifest.json");
    let f3 = InterpolationManifest::load(&path)?;
    ensure!(
        f3.validation.all_passed(),
        "data_invalid: 插值数据无效，F4 不得放行"
    );
    let input = source::bind(&work, cfg, &f3)?;
    ensure!(
        backend == Backend::Local,
        "allocation: F4 只在 F1 已登记分配内顺序执行，不另行 sbatch"
    );
    let mut before = || -> Result<()> {
        if !crate::foundation::native_jobs::enabled() {
            let job = std::env::var("SLURM_JOB_ID")
                .context("allocation: 缺少当前 Slurm 分配")?
                .parse::<u64>()?;
            budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
                ledger.require_allocation(job, &cfg.resources).map(|_| ())
            })?;
        }
        ensure!(
            store::hash_file(&path)? == input.interpolation_digest,
            "input_conflict: F3 清单改变"
        );
        Ok(())
    };
    before()?;
    let root = crate::foundation::layout::path(&work, "superconductivity");
    std::fs::create_dir_all(&root)?;
    let modes = match cfg.superconductivity.mode {
        Mode::Iso => vec![Mode::Iso],
        Mode::Aniso => vec![Mode::Aniso],
        Mode::Both => vec![Mode::Iso, Mode::Aniso],
    };
    let mut native_modes = BTreeMap::new();
    for mode in modes {
        let name = if mode == Mode::Iso { "iso" } else { "aniso" };
        let dir = root.join(name);
        let mut engine = Engine::open(cfg, &input, launcher, &dir, mode)?;
        if retry {
            engine.prepare_resume(&mut |job| {
                ensure!(
                    crate::foundation::slurm::accounting(job)?.terminal,
                    "scheduler_unknown: 原分配仍活动，不能重提中断尝试"
                );
                Ok(())
            })?;
        }
        engine.run(&mut before)?;
        native_modes.insert(name.to_string(), engine.result);
    }
    let result = publish(
        &root,
        f3.run_id,
        input.interpolation_digest,
        cfg.superconductivity.mode,
        native_modes,
    )?;
    if !result.converged {
        bail!("partial: 原生求解或搜索尚未完成，已保存分模式区间、尝试和停止原因");
    }
    Ok(result)
}

/// 发布 F4 结构化交付；生产调用者必须先完成来源和预算检查。
pub fn publish(
    root: &Path,
    run_id: String,
    interpolation_digest: String,
    mode: Mode,
    native_modes: BTreeMap<String, super::engine::ModeResult>,
) -> Result<SuperconductivityManifest> {
    let expected = match mode {
        Mode::Iso => vec!["iso"],
        Mode::Aniso => vec!["aniso"],
        Mode::Both => vec!["aniso", "iso"],
    };
    ensure!(
        native_modes.keys().map(String::as_str).collect::<Vec<_>>() == expected,
        "data_invalid: 缺少所选模式，不能发布原生阶段"
    );
    for r in native_modes.values() {
        super::verify::mode(r, &interpolation_digest)?;
    }
    let complete = native_modes.values().all(|r| r.complete);
    let located = native_modes.values().all(|r| r.search.status == "located");
    let mut validation = crate::qe_data::manifest::Validation {
        status: if complete { "valid" } else { "partial" }.into(),
        checks: vec![],
        units: BTreeMap::new(),
        thresholds: BTreeMap::new(),
        evidence: vec![],
    };
    validation.check(
        "native_modes_complete",
        complete,
        "所选模式的原生 Tc 区间、双初值及必需能隙点均须有效",
    );
    validation.units.insert("temperature".into(), "K".into());
    validation.units.insert("gap".into(), "meV".into());
    validation.units.insert("z".into(), "dimensionless".into());
    for r in native_modes.values() {
        for a in &r.attempts {
            if let Some(id) = &a.record {
                validation
                    .evidence
                    .push(id.path.to_string_lossy().into_owned());
            }
        }
    }
    let mut result = SuperconductivityManifest {
        schema_version: 2,
        run_id,
        interpolation_digest,
        mode: match mode {
            Mode::Iso => "iso",
            Mode::Aniso => "aniso",
            Mode::Both => "both",
        }
        .into(),
        tc: None,
        located,
        converged: complete,
        gaps_mev: vec![],
        attempts: vec![],
        validation,
        native_modes,
        artifacts: vec![],
    };
    let summaries: BTreeMap<_,_> = result.native_modes.iter().map(|(name,r)| (name,serde_json::json!({
        "method":r.method,"thresholds":r.thresholds,"status":r.search.status,"complete":r.complete,
        "interval_k":r.search.interval_k(),"estimate_k":r.search.estimate_k(),"half_width_k":r.search.half_width_k(),
        "attempt_count":r.attempts.len(),"reason":r.reason.as_ref().or(r.search.reason.as_ref()),"search":r.search,
    }))).collect();
    store::write(
        &root.join("tc.json"),
        &serde_json::json!({"schema_version":2,"modes":summaries}),
    )?;
    result.artifacts = export_temperatures(&root, &result.native_modes)?;
    result
        .artifacts
        .push(store::Identity::capture(&root.join("tc.json"))?);
    store::write(&root.join("manifest.json"), &result)?;
    Ok(result)
}

fn export_temperatures(
    root: &Path,
    modes: &BTreeMap<String, super::engine::ModeResult>,
) -> Result<Vec<store::Identity>> {
    let mut csv = String::from("mode,temperature_k,kind,seed_mev,status,band,relative_residual,d_inf_mev,d0_inf_mev,signed_mean_mev,absolute_mean_mev,rms_mev,min_absolute_mev,max_absolute_mev,weight_sum,binding_digest\n");
    let mut artifacts = vec![];
    for (mode, result) in modes {
        for (i, entry) in result.attempts.iter().enumerate() {
            let kind = if entry.request.kind == super::attempt::SolveKind::Linear {
                "linear"
            } else {
                "nonlinear"
            };
            let prefix = format!(
                "{mode},{:.2},{kind},{},{}",
                entry.request.temperature_centikelvin as f64 / 100.0,
                entry
                    .request
                    .seed_mev
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                entry.status
            );
            let record = entry
                .record
                .as_ref()
                .map(|id| store::read::<super::attempt::AttemptResult>(&id.path))
                .transpose()?;
            let residual = record
                .as_ref()
                .and_then(|r| r.evidence.as_ref())
                .and_then(|e| e.relative_residual)
                .map(|x| x.to_string())
                .unwrap_or_default();
            if let Some(gap) = record
                .as_ref()
                .filter(|_| entry.status == "valid")
                .and_then(|r| r.gap.as_ref())
            {
                let path = root
                    .join("gaps")
                    .join(format!("{mode}-attempt-{:04}.json", i + 1));
                store::write(&path, gap)?;
                artifacts.push(store::Identity::capture(&path)?);
                for (band, stats) in gap.band_statistics()? {
                    csv.push_str(&format!(
                        "{prefix},{band},{residual},{},{},{},{},{},{},{},{},{}\n",
                        gap.d_inf(),
                        gap.d0_inf(),
                        stats.signed_mean_mev,
                        stats.absolute_mean_mev,
                        stats.rms_mev,
                        stats.min_absolute_mev,
                        stats.max_absolute_mev,
                        stats.weight_sum,
                        result.binding_digest
                    ));
                }
            } else {
                csv.push_str(&format!(
                    "{prefix},,{residual},,,,,,,,,{}\n",
                    result.binding_digest
                ));
            }
        }
    }
    let path = root.join("temperatures.csv");
    store::atomic_bytes(&path, csv.as_bytes())?;
    artifacts.push(store::Identity::capture(&path)?);
    Ok(artifacts)
}

/// 按 TOML 任务选择指定温度能隙或既有 Tc 搜索。
pub fn run_configured(
    cfg: &config::Config,
    work: &Path,
    launcher: &[String],
    backend: Backend,
    retry: bool,
) -> Result<()> {
    if cfg.superconductivity.task == config::SuperconductivityTask::Gap {
        super::points::run(cfg, work, launcher, backend, retry)
    } else {
        run_with_retry(cfg, work, launcher, backend, retry).map(|_| ())
    }
}
