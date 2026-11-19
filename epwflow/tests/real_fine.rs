//! 本机真实生产函数验收；显式选择、运行中的 Slurm 分配、固定来源三者缺一不可。
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
        std::env::var_os("EPWFLOW_REAL_FINE_CASE").context("Specifies the true test manifest for the freeze")?,
    );
    let case_id = Identity::capture(&case_path)?;
    let c: Case = store::read(&case_path)?;
    ensure!(
        c.schema_version == 1 && c.source_identities.len() >= 8,
        "Real sources are incomplete"
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
            && cfg.interpolation.phonon_smearings_mev == [0.5],
        "There's a discrepancy between the acceptance parameters"
    );
    let job: u64 = std::env::var("SLURM_JOB_ID")?.parse()?;
    ensure!(
        slurm::running_allocation(job)?.state == "RUNNING",
        "It has to be operational Slurm Inward"
    );
    ensure!(
        std::env::var("SLURM_NTASKS")?.parse::<u32>()? == epwflow::foundation::parallelism::mpi_count(&cfg).unwrap(),
        "MPI Numbers do not match"
    );
    ensure!(!cfg.work_dir.exists(), "Target exists. Covering or retrying is prohibited");
    let states = epwflow::qe_data::nscf::parse_xml(&c.nscf_xml.path)?;
    ensure!(
        states.nbnd == cfg.qe.nscf.nbnd as usize && states.kpoints.len() == 216,
        "Original NSCF Inconsistencies"
    );
    let structure = epwflow::qe_data::qe_config::QeSettings::from_config(&cfg)?;
    let ep = EpwConfig::from_config(&cfg)?
        .with_fermi(states.fermi)?
        .with_bands(&states.eigenvalues)?
        .with_species(&structure.cell.elements)?;
    fs::create_dir_all(&cfg.work_dir)?;
    let coarse_dir=cfg.work_dir.join("coarse");
    interpolation::restart::stage(&c.source,&c.source_out,&cfg.qe.prefix,
        cfg.wannier.num_wann as usize,structure.cell.positions.len()*3,&coarse_dir)?;
    for name in [format!("{}.wout",cfg.qe.prefix),"epw_coarse.in".into(),"epw.out".into()] {
        fs::copy(c.source.join(&name),coarse_dir.join(&name))?;
    }
    let coarse=interpolation::coarse::collect_coarse(&cfg.work_dir,&ep)?;
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
    let spectrum = interpolation::native::NativeSpectrum::read(
        &cfg.work_dir.join("fine/smearing-0001/mgb2.a2f"),
        500,
    )?;
    case_id.verify()?;
    c.config.verify()?;
    for id in &c.source_identities {
        id.verify()?;
    }
    let fine_data = interpolation::fine::into_fine_data(&fine);
    store::write(
        &report,
        &serde_json::json!({"schema_version":1,"stage":"fine_matrices_verified","job_id":job,"case":case_id,"readback_only":readback_only,
        "coarse":interpolation::coarse::into_coarse_data(&coarse),"fine":fine_data,
        "spectrum":{"energy_mev":spectrum.energy_mev,"a2f":spectrum.a2f,"lambda_integral":spectrum.lambda_integral,"lambda_sum":spectrum.lambda_sum},
        "historical_sources_unchanged":true,"automatic_workflow_validated":false,"tc_computed":false}),
    )?;
    Ok(())
}

#[test]
#[ignore = "Real 40³/20³ EPC ;Requires visible configuration, execution within a Slurm allocation"]
fn actual_lda_fine_matrices() {
    actual().unwrap();
}

#[test]
#[ignore = "Read only the truth of the termination EPC ;Do not start scientific program"]
fn actual_lda_fine_readback() {
    let path = PathBuf::from(std::env::var_os("EPWFLOW_REAL_FINE_CASE").expect("Specifies the original freeze manifest"));
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
