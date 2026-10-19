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
        quality_binding,
    },
    superconductivity::{engine::Engine, scheduler, source},
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
}

fn allocation(cfg: &Config) -> Result<u64> {
    let job: u64 = std::env::var("SLURM_JOB_ID")?.parse()?;
    ensure!(
        slurm::accounting(job)?.state == "RUNNING",
        "真实分配没有运行"
    );
    ensure!(
        std::env::var("SLURM_NTASKS")?.parse::<u32>()? == cfg.resources.mpi,
        "MPI 数不符"
    );
    // 原共享历史账本，不为本项建立新的零消费预算。
    let ledger: serde_json::Value = store::read(&cfg.budget.ledger)?;
    let attempts = ledger["attempts"].as_array().context("缺原共享预算")?;
    let reserved: u64 = attempts
        .iter()
        .map(|a| a["reserved_node_seconds"].as_u64().unwrap_or(0))
        .sum();
    let matching: Vec<_> = attempts
        .iter()
        .filter(|a| a["job_id"].as_u64() == Some(job))
        .collect();
    ensure!(
        ledger["campaign"] == "epic-0007"
            && ledger["limits"]["node_seconds"] == 28800
            && matching.len() == 1
            && matching[0]["reserved_node_seconds"].as_u64().unwrap_or(0)
                >= cfg.resources.wall_seconds
            && ledger["spent_node_seconds"]
                .as_u64()
                .context("缺历史消费")?
                + reserved
                <= 28800
            && attempts
                .iter()
                .filter(|a| a["reserved_node_seconds"].as_u64().unwrap_or(0) > 0)
                .count()
                <= 2,
        "共享预算未登记或超限"
    );
    Ok(job)
}

fn actual() -> Result<()> {
    let path = PathBuf::from(std::env::var_os("EPWFLOW_REAL_TC_CASE").context("指定冻结案例")?);
    let case_id = Identity::capture(&path)?;
    let c: Case = store::read(&path)?;
    ensure!(c.schema_version == 1, "案例版本不符");
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
        ensure!(current[key] == previous[key], "冻结物理来源改变：{key}");
    }
    let mut sc = current["superconductivity"].clone();
    for mode in ["iso", "aniso"] {
        ensure!(
            sc[mode]["min_k"] == 10.0 && sc[mode]["max_k"] == 55.0,
            "本次预先固定搜索范围为 10..55 K"
        );
        sc[mode]["min_k"] = previous["superconductivity"][mode]["min_k"].clone();
        sc[mode]["max_k"] = previous["superconductivity"][mode]["max_k"].clone();
    }
    ensure!(
        sc == previous["superconductivity"],
        "求解器、初值、容差或必需能隙温度改变"
    );
    ensure!(
        cfg.resources.nodes == 1 && cfg.resources.mpi == 24 && cfg.resources.wall_seconds <= 7200,
        "超出本次有界分配"
    );
    let job = allocation(&cfg)?;
    ensure!(
        cfg.work_dir.exists() == c.resume,
        "新运行与显式恢复的目标状态不符"
    );
    let report: serde_json::Value = store::read(&c.fine_report.path)?;
    ensure!(
        report["stage"] == "fine_matrices_verified"
            && report["historical_sources_unchanged"] == true
            && report["automatic_workflow_validated"] == false,
        "上游阶段证据不符"
    );
    let fine_case: Identity = serde_json::from_value(report["case"].clone())?;
    fine_case.verify()?;
    let fine_case_data: serde_json::Value = store::read(&fine_case.path)?;
    let source_config: Identity = serde_json::from_value(fine_case_data["config"].clone())?;
    ensure!(
        serde_json::to_value(&source_config)? == serde_json::to_value(&c.source_config)?,
        "细网格与来源配置没有绑定"
    );
    let historical: Vec<Identity> =
        serde_json::from_value(fine_case_data["source_identities"].clone())?;
    for id in &historical {
        id.verify()?;
    }
    let quality = quality_binding::check(&cfg, &c.fine_root)?;
    ensure!(
        quality.passed && quality.phonon.points == 5 && quality.phonon.modes_per_point == 9,
        "五点质量门没有通过"
    );
    ensure!(
        serde_json::to_value(&quality)? == report["quality"],
        "质量报告与来源冲突"
    );
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
    fs::create_dir_all(&cfg.work_dir)?;
    let root = cfg.work_dir.join("superconductivity");
    fs::create_dir_all(&root)?;
    let mut before = || -> Result<()> {
        allocation(&cfg)?;
        case_id.verify()?;
        c.config.verify()?;
        c.source_config.verify()?;
        c.fine_report.verify()?;
        ensure!(
            quality_binding::check(&cfg, &c.fine_root)?.passed,
            "质量来源改变"
        );
        Ok(())
    };
    let mut modes = BTreeMap::new();
    for (mode, name) in [(Mode::Iso, "iso"), (Mode::Aniso, "aniso")] {
        let dir = root.join(name);
        let mut engine = Engine::open(&cfg, &input, &cfg.programs.launcher, &dir, mode)?;
        if c.resume {
            engine.prepare_resume(&mut |previous_job| {
                ensure!(
                    slurm::accounting(previous_job)?.terminal,
                    "原分配仍活动，不能重提"
                );
                Ok(())
            })?;
        }
        engine.run(&mut before)?;
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
            "schema_version":1, "job_id":job, "case":case_id, "quality":quality,
            "automatic_workflow_validated":false, "historical_sources_unchanged":true,
            "tc_computed":result.converged, "manifest":Identity::capture(&root.join("manifest.json"))?,
        }),
    )?;
    ensure!(
        result.converged,
        "原生两模式仍未完成；保留全部失败与计数，禁止经验 Tc 替代"
    );
    Ok(())
}

#[test]
#[ignore = "真实两模式 Tc；须冻结来源、预留原共享预算并在 Slurm 中显式执行"]
fn actual_lda_native_tc() {
    actual().unwrap();
}
