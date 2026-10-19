//! 本机真实生产函数验收；显式选择、已记账 Slurm 分配、固定来源三者缺一不可。
//! 不发布 F2/F3 完整工作流清单，不把此项阶段验收冒充自动链完成。
use anyhow::{ensure, Context, Result};
use epwflow::{
    foundation::{
        config::Config,
        slurm,
        stage::Backend,
        store::{self, Identity},
    },
    interpolation::{self, epw_config::EpwConfig},
};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    schema_version: u32,
    config: Identity,
    source: PathBuf,
    source_out: PathBuf,
    nscf_xml: Identity,
    source_identities: Vec<Identity>,
}

fn actual() -> Result<()> {
    let case_path = PathBuf::from(
        std::env::var_os("EPWFLOW_REAL_FINE_CASE").context("指定冻结的真实测试清单")?,
    );
    let case_id = Identity::capture(&case_path)?;
    let c: Case = store::read(&case_path)?;
    ensure!(
        c.schema_version == 1 && c.source_identities.len() >= 8,
        "真实来源不完整"
    );
    c.config.verify()?;
    c.nscf_xml.verify()?;
    for id in &c.source_identities {
        id.verify()?;
    }
    let cfg = Config::load(&c.config.path)?;
    ensure!(
        cfg.wannier.num_wann == 5
            && cfg.qe.nscf.k_grid == [6; 3]
            && cfg.phonon.q_grid == [6; 3]
            && cfg.interpolation.k_grid == [40; 3]
            && cfg.interpolation.q_grid == [20; 3]
            && cfg.interpolation.phonon_smearings_mev == [0.5]
            && cfg.interpolation.quality.k_points_crystal.len() == 6
            && cfg.interpolation.quality.q_points_crystal.len() == 5,
        "本次有界验收参数不符"
    );
    let job: u64 = std::env::var("SLURM_JOB_ID")?.parse()?;
    ensure!(
        slurm::running_allocation(job)?.state == "RUNNING",
        "必须在实际运行的 Slurm 分配内"
    );
    ensure!(
        std::env::var("SLURM_NTASKS")?.parse::<u32>()? == cfg.resources.mpi,
        "MPI 数不符"
    );
    // 本机历史试验仍共用原账本；不得为测试建立零消费的新账本。
    let ledger: serde_json::Value = store::read(&cfg.budget.ledger)?;
    let attempts = ledger["attempts"].as_array().context("缺原共享预算记录")?;
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
        "共享预算未登记本次分配或超限"
    );
    ensure!(!cfg.work_dir.exists(), "目标已存在，禁止覆盖或暗中重试");
    let states = epwflow::qe_data::nscf::parse_xml(&c.nscf_xml.path)?;
    ensure!(
        states.nbnd == cfg.qe.nscf.nbnd as usize && states.kpoints.len() == 216,
        "原始 NSCF 不符"
    );
    let frozen = interpolation::quality_binding::freeze(&cfg, states.fermi)?;
    let structure = epwflow::qe_data::qe_config::QeSettings::from_config(&cfg)?;
    let ep = EpwConfig::from_config(&cfg)?
        .with_fermi(states.fermi)?
        .with_species(&structure.cell.elements)?;
    fs::create_dir_all(&cfg.work_dir)?;
    let report = cfg.work_dir.join("native-fine-validation.json");
    store::write(
        &report,
        &serde_json::json!({"schema_version":1,"stage":"quality_running","job_id":job,"case":case_id}),
    )?;
    let coarse = interpolation::coarse::reread_quality(
        &ep,
        &c.source,
        &c.source_out,
        &cfg.work_dir,
        &cfg.programs.launcher,
        Backend::Local,
    )?;
    let quality = frozen.pair(&cfg.work_dir)?;
    store::write(
        &report,
        &serde_json::json!({"schema_version":1,"stage":"quality_checked","job_id":job,"quality":quality,"case":case_id}),
    )?;
    ensure!(
        quality.passed && quality.phonon.points == 5 && quality.phonon.modes_per_point == 9,
        "独立五点质量未通过"
    );
    for id in &c.source_identities {
        id.verify()?;
    }
    let fine = interpolation::fine::run_fine_at(
        &ep,
        &cfg.work_dir,
        &cfg.work_dir.join("fine/smearing-0001"),
        0.5,
        &cfg.programs.launcher,
        Backend::Local,
    )?;
    publish(&c, &case_id, &cfg, job, &coarse, &fine, false)
}

fn publish(
    c: &Case,
    case_id: &Identity,
    cfg: &Config,
    job: u64,
    coarse: &interpolation::coarse::CoarseResult,
    fine: &interpolation::fine::FineResult,
    readback_only: bool,
) -> Result<()> {
    let report = cfg.work_dir.join("native-fine-validation.json");
    let quality = interpolation::quality_binding::check(cfg, &cfg.work_dir)?;
    let spectrum = interpolation::native::NativeSpectrum::read(
        &cfg.work_dir.join("fine/smearing-0001/mgb2.a2f"),
        500,
    )?;
    ensure!(
        interpolation::quality_binding::check(&cfg, &cfg.work_dir)?.passed,
        "执行中质量身份变化"
    );
    case_id.verify()?;
    c.config.verify()?;
    for id in &c.source_identities {
        id.verify()?;
    }
    let fine_data = interpolation::fine::into_fine_data(&fine);
    store::write(
        &report,
        &serde_json::json!({"schema_version":1,"stage":"fine_matrices_verified","job_id":job,"case":case_id,"quality":quality,"readback_only":readback_only,
        "coarse":interpolation::coarse::into_coarse_data(&coarse),"fine":fine_data,
        "spectrum":{"energy_mev":spectrum.energy_mev,"a2f":spectrum.a2f,"lambda_integral":spectrum.lambda_integral,"lambda_sum":spectrum.lambda_sum},
        "historical_sources_unchanged":true,"automatic_workflow_validated":false,"tc_computed":false}),
    )?;
    Ok(())
}

#[test]
#[ignore = "真实 40³/20³ EPC；须显式配置、预留原共享预算并在 Slurm 中执行"]
fn actual_lda_fine_matrices() {
    actual().unwrap();
}

#[test]
#[ignore = "只读重验已经终止的真实 EPC；不启动科学程序"]
fn actual_lda_fine_readback() {
    let path = PathBuf::from(std::env::var_os("EPWFLOW_REAL_FINE_CASE").expect("指定原冻结清单"));
    let id = Identity::capture(&path).unwrap();
    let c: Case = store::read(&path).unwrap();
    c.config.verify().unwrap();
    c.nscf_xml.verify().unwrap();
    let cfg = Config::load(&c.config.path).unwrap();
    let report = cfg.work_dir.join("native-fine-validation.json");
    let old: serde_json::Value = store::read(&report).unwrap();
    let job = old["job_id"].as_u64().unwrap();
    assert!(slurm::accounting(job).unwrap().terminal);
    for sub in ["coarse", "fine/smearing-0001"] {
        assert_eq!(
            fs::read_to_string(cfg.work_dir.join(sub).join("epw.exitcode"))
                .unwrap()
                .trim(),
            "0"
        );
    }
    let ep = EpwConfig::from_config(&cfg).unwrap();
    let coarse = interpolation::coarse::collect_coarse(&cfg.work_dir, &ep).unwrap();
    let fine = interpolation::fine::collect_native_fine_at(
        &cfg.work_dir,
        &cfg.work_dir.join("fine/smearing-0001"),
        &ep,
    )
    .unwrap();
    let backup = cfg
        .work_dir
        .join("native-fine-validation-before-readback.json");
    if !backup.exists() {
        fs::copy(&report, backup).unwrap();
    }
    publish(&c, &id, &cfg, job, &coarse, &fine, true).unwrap();
}
