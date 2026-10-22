//! 已有真实 QE 数据的只读绘图检查；显式运行，不启动科学程序或 Slurm。
use epwflow::foundation::{config::Config, stages::Step, store, workspace_qe_plot};
use std::{fs, path::PathBuf};

#[test]
#[ignore = "需要显式指定已有真实工作区和新的图像输出目录"]
fn existing_real_qe_data_plot_without_scientific_execution() {
    let source =
        PathBuf::from(std::env::var_os("DFTFLOW_QE_PLOT_SOURCE").expect("DFTFLOW_QE_PLOT_SOURCE"));
    let output =
        PathBuf::from(std::env::var_os("DFTFLOW_QE_PLOT_OUTPUT").expect("DFTFLOW_QE_PLOT_OUTPUT"));
    assert!(
        !output.exists(),
        "使用新的输出目录，不能覆盖真实来源或历史结果"
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
    workspace_qe_plot::refresh(&cfg, Step::Electronic).unwrap();
    workspace_qe_plot::refresh(&cfg, Step::Phonon).unwrap();
    for p in [
        "3.qe-electronic/qe_eband/plots/electronic-bands.png",
        "3.qe-electronic/qe_eledos/plots/electronic-dos.png",
        "4.qe-phonon-nosplit/plots/phband/phonon_band.png",
        "4.qe-phonon-nosplit/plots/phdos/phonon_dos.png",
    ] {
        assert!(fs::read(output.join(p)).unwrap().starts_with(b"\x89PNG"));
    }
    let elements =
        fs::read_to_string(output.join("4.qe-phonon-nosplit/plots/phdos/phonon_dos_elements.csv"))
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
    println!("真实 QE 图像：{}", output.display());
}
