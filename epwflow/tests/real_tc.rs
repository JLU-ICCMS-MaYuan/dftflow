//! 固定 LDA 原生细网格的 F4 验收；复用生产 Engine，不伪造完整 F3 自动链清单。
use anyhow::{ensure, Context, Result};
use epwflow::{
    foundation::{
        config::{Config, Mode},
        slurm,
        store::{self, Identity},
    },
    interpolation::{
        manifest::{FineData, Spectrum},
        native::NativeSpectrum,
    },
    superconductivity::{
        aniso, attempt::SolveKind, engine::Engine, manifest::SuperconductivityManifest, scheduler,
        search::Side, source,
    },
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    schema_version: u32,
    config: Identity,
    source_config: Identity,
    fine_report: Identity,
    fine_root: PathBuf,
    #[serde(default)]
    resume: bool,
    #[serde(default)]
    diagnostic: bool,
    parent_manifest: Option<Identity>,
}

fn allocation(cfg: &Config) -> Result<u64> {
    let job: u64 = std::env::var("SLURM_JOB_ID")?.parse()?;
    ensure!(
        slurm::running_allocation(job)?.state == "RUNNING",
        "Real distribution is not running"
    );
    ensure!(
        std::env::var("SLURM_NTASKS")?.parse::<u32>()? == epwflow::foundation::parallelism::mpi_count(&cfg).unwrap(),
        "MPI Numbers do not match"
    );
    // 原共享历史账本，不为本项建立新的零消费预算。
    let ledger: serde_json::Value = store::read(&cfg.budget.ledger)?;
    let attempts = ledger["attempts"].as_array().context("Lack of original shared budget")?;
    let matching: Vec<_> = attempts
        .iter()
        .filter(|a| a["job_id"].as_u64() == Some(job))
        .collect();
    ensure!(
        ledger["campaign"] == "epic-0007"
            && matching.len() == 1
            && attempts
                .iter()
                .filter(|a| a["reserved_node_seconds"].as_u64().unwrap_or(0) > 0)
                .count()
                <= 2,
        "Shared books are not registered or issued out of limit"
    );
    Ok(job)
}

fn actual(diagnostic: bool) -> Result<()> {
    let path = PathBuf::from(std::env::var_os("EPWFLOW_REAL_TC_CASE").context("_Other Organiser")?);
    let case_id = Identity::capture(&path)?;
    let c: Case = store::read(&path)?;
    ensure!(
        c.schema_version == 1 && c.diagnostic == diagnostic,
        "Unsatisfactory version of case or type of execution"
    );
    ensure!(!diagnostic || !c.resume, "Fixed diagnostics run only once, and no hidden renewals");
    for id in [&c.config, &c.source_config, &c.fine_report] {
        id.verify()?;
    }
    let cfg = Config::load(&c.config.path)?;
    let old = Config::load(&c.source_config.path)?;
    let current = serde_json::to_value(&cfg)?;
    let previous = serde_json::to_value(&old)?;
    for key in [
        "structure",
        "pseudos",
        "atomic_masses",
        "programs",
        "qe",
        "phonon",
        "wannier",
        "interpolation",
        "budget",
    ] {
        ensure!(current[key] == previous[key], "Freezing changes in physical sources:{key}");
    }
    let mut sc = current["superconductivity"].clone();
    for mode in ["iso", "aniso"] {
        ensure!(
            sc[mode]["min_k"] == 10.0 && sc[mode]["max_k"] == if diagnostic { 80.0 } else { 55.0 },
            "This pre-fixed temperature range does not match"
        );
        sc[mode]["min_k"] = previous["superconductivity"][mode]["min_k"].clone();
        sc[mode]["max_k"] = previous["superconductivity"][mode]["max_k"].clone();
        if diagnostic {
            ensure!(sc[mode]["max_solves"] == 2, "A single diagnosis can only have two starting values");
            sc[mode]["max_solves"] = previous["superconductivity"][mode]["max_solves"].clone();
        }
    }
    if diagnostic {
        for (key, expected) in [
            ("max_iterations", serde_json::json!(2000)),
            ("mixing_beta", serde_json::json!(0.7)),
            ("mixing_history", serde_json::json!(8)),
        ] {
            ensure!(sc[key] == expected, "Non-conformity of fixed diagnostic numerical parameters:{key}");
            sc[key] = previous["superconductivity"][key].clone();
        }
    }
    ensure!(
        sc == previous["superconductivity"],
        "Solver, starting value, tolerance difference, or need to be able to change the temperature of the gap"
    );
    ensure!(
        epwflow::foundation::parallelism::mpi_count(&cfg).unwrap() == 24,
        "Over and above the distribution"
    );
    let job = allocation(&cfg)?;
    ensure!(
        cfg.work_dir.exists() == c.resume,
        "New operations do not match the target state for visible recovery"
    );
    let report: serde_json::Value = store::read(&c.fine_report.path)?;
    ensure!(
        report["stage"] == "fine_matrices_verified"
            && report["historical_sources_unchanged"] == true
            && report["automatic_workflow_validated"] == false,
        "Evidence at the upstream stage does not match"
    );
    let fine_case: Identity = serde_json::from_value(report["case"].clone())?;
    fine_case.verify()?;
    let fine_case_data: serde_json::Value = store::read(&fine_case.path)?;
    let source_config: Identity = serde_json::from_value(fine_case_data["config"].clone())?;
    ensure!(
        serde_json::to_value(&source_config)? == serde_json::to_value(&c.source_config)?,
        "The fine grid and source configuration are not tied"
    );
    let historical: Vec<Identity> =
        serde_json::from_value(fine_case_data["source_identities"].clone())?;
    for id in &historical {
        id.verify()?;
    }
    let fine: FineData = serde_json::from_value(report["fine"].clone())?;
    let spectrum: NativeSpectrum = serde_json::from_value(report["spectrum"].clone())?;
    let spectra = vec![Spectrum {
        smearing_id: "native-0.5meV".into(),
        energy_mev: spectrum.energy_mev,
        a2f: spectrum.a2f[0].clone(),
        lambda_integral: spectrum.lambda_integral[0],
        lambda_sum: spectrum.lambda_sum,
        cutoff_ev: cfg.superconductivity.frequency_cutoff_ev,
    }];
    let input = source::bind_native_roles(&c.fine_root, &cfg, &fine, &spectra, &c.fine_report)?;
    let previous_counts = if diagnostic {
        let parent = c.parent_manifest.as_ref().context("Diagnosis lacked historical sources")?;
        parent.verify()?;
        let m = SuperconductivityManifest::load(&parent.path)?;
        ensure!(
            m.interpolation_digest == c.fine_report.sha256,
            "History counts are not the same fine grid"
        );
        let counts: BTreeMap<_, _> = m
            .native_modes
            .iter()
            .map(|(name, r)| (name.clone(), r.attempts.len()))
            .collect();
        for (name, n) in &counts {
            ensure!(
                n + 2 <= 64 && (name == "iso" || name == "aniso"),
                "More diagnostics than the original residuals"
            );
        }
        ensure!(counts.len() == 2, "History mode count missing");
        counts
    } else {
        BTreeMap::new()
    };
    fs::create_dir_all(&cfg.work_dir)?;
    let root = cfg.work_dir.join("superconductivity");
    fs::create_dir_all(&root)?;
    let mut before = || -> Result<()> {
        allocation(&cfg)?;
        case_id.verify()?;
        c.config.verify()?;
        c.source_config.verify()?;
        c.fine_report.verify()?;
        if let Some(parent) = &c.parent_manifest {
            parent.verify()?;
        }
        Ok(())
    };
    let mut modes = BTreeMap::new();
    let mut diagnostic_sides = BTreeMap::new();
    for (mode, name) in [(Mode::Iso, "iso"), (Mode::Aniso, "aniso")] {
        let dir = root.join(name);
        let mut engine = Engine::open(&cfg, &input, &cfg.programs.launcher, &dir, mode)?;
        if c.resume {
            engine.prepare_resume(&mut |previous_job| {
                ensure!(
                    slurm::accounting(previous_job)?.terminal,
                    "The original allocation is still active and cannot be revisited"
                );
                Ok(())
            })?;
        }
        if diagnostic {
            let t = if mode == Mode::Iso { 1000 } else { 8000 };
            let mut solutions = vec![];
            // 一次诊断事先授权两初值；首个失败仍保留并执行第二个，不扩展温度或参数。
            for seed in cfg.superconductivity.initial_gaps_mev {
                let n = engine.result.attempts.len();
                match engine.attempt(t, SolveKind::Nonlinear, Some(seed), &mut before) {
                    Ok(record) => solutions.push(record.gap.context("Effective non-linear results are missing Δ/Z")?),
                    Err(e) => {
                        ensure!(
                            engine.result.attempts.len() == n + 1
                                && engine
                                    .result
                                    .attempts
                                    .last()
                                    .is_some_and(|a| a.status == "failed"),
                            "Diagnosis ceased before startup or unknown state:{e:#}"
                        );
                        println!("{name} {seed} meV failed: {e:#}");
                    }
                }
            }
            let side = if solutions.len() == 2 {
                let tc = if mode == Mode::Iso {
                    cfg.superconductivity.iso.as_ref()
                } else {
                    cfg.superconductivity.aniso.as_ref()
                }
                .unwrap();
                match aniso::classify(
                    &solutions[0],
                    &solutions[1],
                    tc.normal_gap_mev,
                    tc.super_gap_mev,
                ) {
                    Ok(side) => Some(side),
                    Err(e) => {
                        println!("{name} dual-seed classification failed: {e:#}");
                        None
                    }
                }
            } else {
                None
            };
            // iso 的 Tc 分类必须来自线性本征值；非线性诊断仅保存在独立结果中。
            if let Some(side) = side.filter(|_| mode == Mode::Aniso) {
                engine.result.search.observe(t, side)?;
                engine.save()?;
            }
            diagnostic_sides.insert(name.to_string(), side);
        } else {
            engine.run(&mut before)?;
        }
        println!(
            "{name}: {} {:?} {:?}",
            engine.result.search.status,
            engine.result.search.interval_k(),
            engine.result.reason
        );
        modes.insert(name.to_string(), engine.result);
    }
    let result = scheduler::publish(
        &root,
        "native-lda-stage-validation".into(),
        input.interpolation_digest,
        Mode::Both,
        modes,
    )?;
    for id in &historical {
        id.verify()?;
    }
    c.fine_report.verify()?;
    store::write(
        &cfg.work_dir.join("native-tc-validation.json"),
        &serde_json::json!({
            "schema_version":1, "job_id":job, "case":case_id,
            "automatic_workflow_validated":false, "historical_sources_unchanged":true,
            "tc_computed":result.converged, "diagnostic_only":diagnostic,
            "previous_attempt_counts":previous_counts, "diagnostic_sides":diagnostic_sides,
            "manifest":Identity::capture(&root.join("manifest.json"))?,
        }),
    )?;
    ensure!(
        if diagnostic {
            diagnostic_sides.get("iso") == Some(&Some(Side::Superconducting))
                && diagnostic_sides.get("aniso") == Some(&Some(Side::Normal))
        } else {
            result.converged
        },
        "Two native models are still not satisfied with this acceptance; all failures and counting are retained and experience is banned Tc Alternative"
    );
    Ok(())
}

#[test]
#[ignore = "Real two patterns Tc ;Requires the freezing of sources, pre-encumbering of shared budgets and Slurm Medium-Specific Execution"]
fn actual_lda_native_tc() {
    actual(false).unwrap();
}

#[test]
#[ignore = "Single fixed non-linear diagnosis; user confirmation, number of history and original shared budget"]
fn actual_lda_native_nonlinear_diagnostic() {
    actual(true).unwrap();
}
