//! F3 质量门之后，在同一 F1 分配内执行原生两模式搜索。
use super::{engine::Engine, manifest::SuperconductivityManifest, source};
use crate::{
    foundation::{
        budget,
        config::{self, Mode},
        stage::Backend,
        store,
    },
    interpolation::{manifest::InterpolationManifest, native_quality},
};
use anyhow::{bail, ensure, Context, Result};
use std::{collections::BTreeMap, path::Path};

pub fn run(
    cfg: &config::Config,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<SuperconductivityManifest> {
    let work = std::fs::canonicalize(work)?;
    let path = work.join("interpolation/manifest.json");
    let f3 = InterpolationManifest::load(&path)?;
    ensure!(
        f3.validation.all_passed() && f3.quality.as_ref().is_some_and(|q| q.status == "valid"),
        "data_invalid: 插值质量尚未通过，F4 不得放行"
    );
    let quality = cfg
        .interpolation
        .quality
        .reference_manifest
        .as_ref()
        .context("data_invalid: 缺少独立插值质量来源")?;
    let checked = native_quality::check_current_coarse(quality, &work.join("coarse"))?;
    ensure!(
        checked.passed,
        "data_invalid: 独立插值质量失败，禁止原生超导"
    );
    let input = source::bind(&work, cfg, &f3)?;
    ensure!(
        backend == Backend::Local,
        "allocation: F4 只在 F1 已登记分配内顺序执行，不另行 sbatch"
    );
    let mut before = || -> Result<()> {
        let job = std::env::var("SLURM_JOB_ID")
            .context("allocation: 缺少当前 Slurm 分配")?
            .parse::<u64>()?;
        budget::with_ledger(&cfg.budget.ledger, &cfg.budget.campaign_id, |ledger| {
            ledger.require_allocation(job, &cfg.resources).map(|_| ())
        })?;
        ensure!(
            store::hash_file(&path)? == input.interpolation_digest,
            "input_conflict: F3 清单改变"
        );
        ensure!(
            native_quality::check_current_coarse(quality, &work.join("coarse"))?.passed,
            "data_invalid: F3 质量来源改变"
        );
        Ok(())
    };
    before()?;
    let root = work.join("superconductivity");
    std::fs::create_dir_all(&root)?;
    let _lock = store::Lock::acquire(&root.join("execution"))?;
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
        engine.run(&mut before)?;
        native_modes.insert(name.to_string(), engine.result);
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
        run_id: f3.run_id,
        interpolation_digest: input.interpolation_digest,
        mode: match cfg.superconductivity.mode {
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
    if !complete {
        bail!("partial: 原生求解或搜索尚未完成，已保存分模式区间、尝试和停止原因");
    }
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
