//! 显式最小真实验收：只补 projwfc，复用已完成 QE／EPW 采样，不重算模型。
use epwflow::foundation::{
    config::Config,
    stage_results,
    stages::Step,
    store::{self, Identity},
    workspace_pdos, workspace_plot, workspace_qe_plot,
};
use epwflow::qe_data::save;
use std::{fs, path::PathBuf};
#[test]
#[ignore = "需要指定真实 PBE 工作区、新目录及 Slurm 分配；只执行必要 projwfc 后处理"]
fn real_electronic_dual_windows_and_pdos() {
    assert!(
        std::env::var_os("SLURM_JOB_ID").is_some(),
        "真实科学程序仅在 Slurm 分配内执行"
    );
    let source = PathBuf::from(std::env::var_os("DFTFLOW_0055_SOURCE").unwrap());
    let output = PathBuf::from(std::env::var_os("DFTFLOW_0055_OUTPUT").unwrap());
    let resume = std::env::var_os("DFTFLOW_0055_RESUME").is_some();
    assert!(
        !output.exists() || (resume && !output.join("verification.json").exists()),
        "拒绝覆盖已有完整验收目录"
    );
    let mut cfg = Config::load(&source.join("epwinput.toml")).unwrap();
    let old = cfg.work_dir.clone();
    cfg.work_dir = output.join("run");
    epwflow::foundation::layout::initialize(&cfg).unwrap();
    fs::write(
        cfg.work_dir.join("epw-state/current-config.toml"),
        toml::to_string_pretty(&cfg).unwrap(),
    )
    .unwrap();
    let before = save::files(&old.join("2.qe-nscf/out")).unwrap();
    if !cfg.work_dir.join("2.qe-nscf/out").exists() {
        save::copy_tree(
            &old.join("2.qe-nscf/out"),
            &cfg.work_dir.join("2.qe-nscf/out"),
        )
        .unwrap();
    }
    assert_eq!(
        before,
        save::files(&cfg.work_dir.join("2.qe-nscf/out")).unwrap()
    );
    let prefix = &cfg.qe.prefix;
    let files = [
        format!("3.qe-electronic/qe_eband/out/{prefix}.save/data-file-schema.xml"),
        "3.qe-electronic/qe_eband/path-points.json".into(),
        format!("3.qe-electronic/qe_eledos/{prefix}.tdos"),
    ];
    let mut identities = vec![];
    for rel in &files {
        identities.push(Identity::capture(&old.join(rel)).unwrap());
        let target = cfg.work_dir.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(old.join(rel), target).unwrap();
    }
    let fermi = epwflow::qe_data::nscf::parse_xml(
        &cfg.work_dir
            .join(format!("2.qe-nscf/out/{prefix}.save/data-file-schema.xml")),
    )
    .unwrap()
    .fermi;
    let comparison = cfg
        .work_dir
        .join("5.epw-wannier/comparison-native/electron");
    fs::create_dir_all(&comparison).unwrap();
    for name in [
        "band.eig".into(),
        format!("{prefix}.wout"),
        format!("{prefix}.wpout"),
        format!("{prefix}-dos.dat"),
    ] {
        let from = old
            .join("5.epw-wannier/comparison-native/electron")
            .join(&name);
        identities.push(Identity::capture(&from).unwrap());
        fs::copy(from, comparison.join(name)).unwrap();
    }
    workspace_qe_plot::electronic(&cfg, "qe_eband").unwrap();
    workspace_qe_plot::electronic(&cfg, "qe_eledos").unwrap();
    let receipt = fs::read(
        cfg.work_dir
            .join("3.qe-electronic/qe_eledos/pdos/completion.json"),
    )
    .unwrap();
    workspace_pdos::run(&cfg).unwrap();
    assert_eq!(
        receipt,
        fs::read(
            cfg.work_dir
                .join("3.qe-electronic/qe_eledos/pdos/completion.json")
        )
        .unwrap()
    );
    workspace_plot::bands(&cfg, &comparison, &comparison.join("band.eig"), fermi).unwrap();
    workspace_plot::dos(
        &cfg,
        &comparison,
        &comparison.join(format!("{}-dos.dat", cfg.qe.prefix)),
        fermi,
    )
    .unwrap();
    let names = fs::read_dir(&comparison)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("electronic-") || n == "metrics.json")
        .collect::<Vec<_>>();
    stage_results::publish(
        &cfg,
        Step::Wannier,
        "",
        &comparison,
        &names,
        serde_json::json!({"kind":"real source redraw; no model recalculation"}),
        identities.clone(),
    )
    .unwrap();
    stage_results::check(&cfg.work_dir.join("results")).unwrap();
    assert_eq!(before, save::files(&old.join("2.qe-nscf/out")).unwrap());
    for id in &identities {
        id.verify().unwrap();
    }
    store::write(&output.join("verification.json"),&serde_json::json!({"source":source,"sources":identities,"nscf_source_files":before,"scientific_calls":{"projwfc":1,"pw":0,"ph":0,"wannier":0,"epw":0,"postw90":0},"native_pdos_repeat_reused":true,"fermi_ev":fermi})).unwrap();
}
