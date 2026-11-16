//! 已有真实 QE 数据的只读绘图检查；显式运行，不启动科学程序或 Slurm。
use epwflow::foundation::{config::Config, stages::Step, store, workspace_qe_plot};
use std::{fs, path::PathBuf};

#[test]
#[ignore = "Need visible specification of existing real workspaces and new image output directories"]
fn existing_real_qe_data_plot_without_scientific_execution() {
    let source =
        PathBuf::from(std::env::var_os("DFTFLOW_QE_PLOT_SOURCE").expect("DFTFLOW_QE_PLOT_SOURCE"));
    let output =
        PathBuf::from(std::env::var_os("DFTFLOW_QE_PLOT_OUTPUT").expect("DFTFLOW_QE_PLOT_OUTPUT"));
    assert!(
        !output.exists(),
        "Use new output directories to overwrite real sources or historical results"
    );
    let mut cfg = Config::load(&source.join("epwinput.toml")).unwrap();
    let prefix = cfg.qe.prefix.clone();
    let inputs = vec![
        format!("2.qe-nscf/out/{prefix}.save/data-file-schema.xml"),
        format!("3.qe-electronic/qe_eband/out/{prefix}.save/data-file-schema.xml"),
        "3.qe-electronic/qe_eband/path-points.json".into(),
        format!("3.qe-electronic/qe_eledos/{prefix}.tdos"),
        format!("4.qe-phonon-nosplit/{prefix}.phband.freq"),
        format!("4.qe-phonon-nosplit/{prefix}_band.labelinfo.dat"),
        format!("4.qe-phonon-nosplit/{prefix}.phdos.dat"),
    ];
    let mut identities = inputs
        .iter()
        .map(|p| store::Identity::capture(&source.join(p)).unwrap())
        .collect::<Vec<_>>();
    identities.push(store::Identity::capture(&cfg.structure).unwrap());
    identities.push(store::Identity::capture(cfg.kpath.as_ref().unwrap()).unwrap());
    for relative in &inputs {
        let target = output.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(relative), target).unwrap();
    }
    cfg.work_dir = output.clone();
    epwflow::foundation::layout::initialize(&cfg).unwrap();
    // 旧只读探针缺少波函数／投影，只重绘已存在的 TDOS，明确不完整；完整元素验收见 0055。
    let fermi=epwflow::qe_data::nscf::parse_xml(&output.join(format!("2.qe-nscf/out/{prefix}.save/data-file-schema.xml"))).unwrap().fermi;
    let plot=output.join("3.qe-electronic/plot");
    fs::create_dir_all(&plot).unwrap();
    epwflow::foundation::workspace_plot::qe_bands(&cfg,&plot,fermi).unwrap();
    epwflow::foundation::workspace_plot::qe_dos(&cfg,&plot,fermi).unwrap();
    epwflow::foundation::combined_plot::electronic(&cfg,&plot,&plot).unwrap();
    workspace_qe_plot::refresh(&cfg, Step::Phonon).unwrap();
    for p in [
        "3.qe-electronic/plot/electronic.png",
        "3.qe-electronic/plot/electronic-from-lowest-to-highest.png",
        "4.qe-phonon-nosplit/plot/phonon.png",
    ] {
        assert!(fs::read(output.join(p)).unwrap().starts_with(b"\x89PNG"));
    }
    let elements =
        fs::read_to_string(output.join("4.qe-phonon-nosplit/plot/phdos/phonon_dos_elements.csv"))
            .unwrap();
    assert!(elements.lines().next().unwrap().contains(",Mg,B"));
    // QE 原始逐原子投影是较低精度文本；用对应舍入精度核对可加性。
    for row in elements.lines().skip(1) {
        let values: Vec<f64> = row.split(',').map(|v| v.parse().unwrap()).collect();
        assert!((values[1] - values[2] - values[3]).abs() < 2e-5_f64.max(values[1].abs() * 2e-4));
    }
    for id in &identities {
        id.verify().unwrap();
    }
    store::write(&output.join("source-verification.json"), &serde_json::json!({
        "source":source, "inputs":identities, "input_hashes_unchanged":true,
        "scientific_program_calls":0, "slurm_submissions":0, "kind":"existing real QE data, plotting only"
    })).unwrap();
    println!("Real QE Image:{}", output.display());
}
