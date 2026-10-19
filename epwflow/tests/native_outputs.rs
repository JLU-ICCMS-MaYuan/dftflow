//! 0015：原生 EPW 输出的格式、来源与完整覆盖检查。
use epwflow::interpolation::native::{self, NativeBands, NativeSpectrum};
use std::{fs, process::Command};

const TABLE: &str =
    "1 1 2\n2 2 4\n3 3 6\n Integrated el-ph coupling\n # 6 12\n Summed el-ph coupling 4.2\n";

#[test]
fn native_spectrum_uses_its_own_axis_and_preserves_both_couplings() {
    let s = NativeSpectrum::parse(TABLE, 3).unwrap();
    assert_eq!(s.energy_mev, vec![1., 2., 3.]);
    assert_eq!(s.a2f[1], vec![2., 4., 6.]);
    assert_eq!(s.lambda_integral, vec![6., 12.]);
    assert_eq!(s.lambda_sum, 4.2);
    for t in [
        TABLE.replace("3 3 6\n", ""),
        TABLE.replace("2 2 4", "2 NaN 4"),
        TABLE.replace("3 3 6", "2 3 6"),
        TABLE.replace("# 6 12", "# 6.1 12"),
        TABLE.replace("2 2 4", "2 -2 4"),
        TABLE.replace("# 6 12", "# 6"),
        TABLE.replace("Summed el-ph coupling 4.2", ""),
    ] {
        assert!(
            NativeSpectrum::parse(&t, 3).is_err(),
            "应拒绝损坏的原生谱: {t}"
        );
    }
}

#[test]
fn eliashberg_spectrum_separates_a2f_from_cumulative_columns() {
    let text = " w[meV] a2f and integrated 2*a2f/w for 2 smearing values\n1 1 2 2 4\n2 2 4 4 8\n3 3 6 6 12\nIntegrated el-ph coupling\n # 6 12\nSummed el-ph coupling 4.2\n";
    let s = NativeSpectrum::parse(text, 3).unwrap();
    assert_eq!(s.a2f.len(), 2);
    assert_eq!(s.a2f[1], vec![2., 4., 6.]);
    assert!(NativeSpectrum::parse(&text.replace("2 2 4 4 8", "2 2 4 4 9"), 3).is_err());
    assert!(NativeSpectrum::parse(&text.replace("for 2", "for 3"), 3).is_err());
}

#[test]
fn completion_requires_exit_status_and_epw_timing() {
    let log = "Program EPW v.5.9 starts\nTotal program execution\nEPW : 2s CPU 3s WALL\n";
    native::checked_log(log, 0).unwrap();
    assert!(native::checked_log(log, 1).is_err());
    assert!(native::checked_log(&log.replace("5.9", "6.0"), 0).is_err());
    assert!(native::checked_log(&log.replace("Total program execution", "JOB DONE"), 0).is_err());
    assert!(native::checked_log(&format!("{log} Error in routine broken"), 0).is_err());
}

#[test]
fn native_bands_reject_truncation_and_non_finite_records() {
    let text = " &plot nbnd= 2, nks= 2 /\n 0 0 0\n 1 2\n .1 .2 .3\n 3 4\n";
    let b = native::parse_bands(text).unwrap();
    assert_eq!(b.values, vec![vec![1., 2.], vec![3., 4.]]);
    assert_eq!(b.points_cartesian[1], [0.1, 0.2, 0.3]);
    assert!(native::parse_bands(&text.replace("3 4", "3")).is_err());
    assert!(native::parse_bands(&text.replace("3 4", "3 NaN")).is_err());
}

#[test]
fn matdyn_fourth_coordinate_column_is_weight_not_frequency() {
    let text = " &plot nbnd= 3, nks= 2 /\n0 0 0 0\n-1 2\n3\n.1 .2 .3 .5\n4 5 6\n";
    let data = native::parse_matdyn(text).unwrap();
    assert_eq!(data.values, vec![vec![-1., 2., 3.], vec![4., 5., 6.]]);
    assert_eq!(data.points_cartesian[1], [0.1, 0.2, 0.3]);
    assert!(native::parse_bands(text).is_err());
    assert!(native::parse_matdyn(&text.replace(".3 .5", ".3 -1")).is_err());
    assert!(native::parse_matdyn(&text.replace(".3 .5", ".3 NaN")).is_err());
    assert!(native::parse_matdyn(&text.replace("4 5 6", "4 5")).is_err());
}

fn bands(values: Vec<f64>) -> NativeBands {
    NativeBands {
        points_cartesian: vec![[0.; 3]],
        values: vec![values],
    }
}

#[test]
fn quality_does_not_hide_missing_band_or_reuse_one_state_twice() {
    let r = bands(vec![8.0, 8.01]);
    let good = bands(vec![1., 8.001, 8.011, 20.]);
    let q = native::electronic_quality(&r, &good, [7., 11.], 0.05, 0.02).unwrap();
    assert!(q.passed);
    assert_eq!(q.reference_states, 2);
    let missing = bands(vec![1., 8.001, 20.]);
    let q = native::electronic_quality(&r, &missing, [7., 11.], 0.05, 0.02).unwrap();
    assert!(!q.passed);
    assert!(q.max_error_ev > 5.);
    assert!(native::electronic_quality(&r, &bands(vec![8.]), [7., 11.], 0.05, 0.02).is_err());
}

#[test]
fn quality_checks_coordinates_full_window_and_common_energy_zero() {
    let r = bands(vec![8., 10.65]);
    let mut fit = bands(vec![8., 19.]);
    assert!(
        !native::electronic_quality(&r, &fit, [7., 11.], 0.05, 0.02)
            .unwrap()
            .passed
    );
    fit.points_cartesian[0] = [0.1, 0., 0.];
    assert!(native::electronic_quality(&r, &fit, [7., 11.], 0.05, 0.02).is_err());
    assert!(
        !native::electronic_quality(&r, &bands(vec![9., 11.65]), [7., 11.], 0.05, 0.02)
            .unwrap()
            .passed
    );
    assert!(native::electronic_quality(&r, &bands(vec![f64::NAN]), [7., 11.], 0.05, 0.02).is_err());
}

#[test]
fn native_cli_checks_actual_files_and_reports_unverified_science() {
    let root = std::env::temp_dir().join(format!(
        "native epw {}",
        epwflow::foundation::store::token()
    ));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("epw.in"),
        "&inputepw\n prefix = 'mgb2'\n nqstep = 3\n/\n",
    )
    .unwrap();
    fs::write(
        root.join("epw.out"),
        "Program EPW v.5.9 starts\nTotal program execution\nEPW : 1s CPU 2s WALL\n",
    )
    .unwrap();
    fs::write(root.join("epw.exitcode"), "0\n").unwrap();
    fs::write(root.join("mgb2.a2f"), TABLE).unwrap();
    let wout = "Using band disentanglement : T\n<<< Disentanglement convergence criteria satisfied >>>\n<<< Wannierisation convergence criteria satisfied >>>\n";
    fs::write(root.join("mgb2.wout"), wout).unwrap();
    let before = epwflow::foundation::store::hash_file(&root.join("mgb2.a2f")).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_epwflow"))
            .current_dir("/")
            .env("PATH", "")
            .args([
                "interpolation",
                "check",
                "-w",
                root.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap()
    };
    let out = run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["workflow_status"], "native_epc_only");
    assert_eq!(v["workflow_complete"], false);
    assert_eq!(v["interpolation_quality"], "unverified");
    assert_eq!(v["eliashberg"], "unverified");
    assert_eq!(v["automatic_chain"], "unverified");
    assert_eq!(v["spectrum"], "mgb2.a2f");
    assert_eq!(
        epwflow::foundation::store::hash_file(&root.join("mgb2.a2f")).unwrap(),
        before
    );
    fs::write(
        root.join("mgb2.wout"),
        wout.replace("<<< Disentanglement convergence criteria satisfied >>>", ""),
    )
    .unwrap();
    assert!(
        !run().status.success(),
        "解纠缠没有收敛，不得仅凭局域化放行"
    );
    fs::write(root.join("mgb2.wout"), wout).unwrap();
    fs::write(root.join("epw.exitcode"), "91").unwrap();
    assert!(!run().status.success());
    fs::write(root.join("epw.exitcode"), "0").unwrap();
    fs::write(root.join("mgb2.a2f"), "1 1\n").unwrap();
    assert!(!run().status.success());
    fs::remove_dir_all(root).unwrap();
}

fn mori_config() -> epwflow::foundation::config::Config {
    use epwflow::foundation::config::{EnergyReference, FermiSource};
    let mut cfg: epwflow::foundation::config::Config =
        toml::from_str(include_str!("fixtures/input.toml")).unwrap();
    cfg.qe.prefix = "mgb2".into();
    cfg.atomic_masses.insert("Mg".into(), 24.305);
    cfg.atomic_masses.insert("B".into(), 10.811);
    cfg.qe.nscf.k_grid = [6; 3];
    cfg.qe.nscf.nbnd = 16;
    cfg.qe.scf.k_grid = [8; 3];
    cfg.qe.ecutrho_ry = 160.;
    cfg.phonon.q_grid = [3; 3];
    cfg.phonon.tr2_ph = 1e-16;
    cfg.wannier.num_wann = 5;
    cfg.wannier.energy_reference = EnergyReference::QeZero;
    cfg.wannier.projections = vec![
        "B:pz",
        "f=0.5,1.0,0.5:s",
        "f=0.0,0.5,0.5:s",
        "f=0.5,0.5,0.5:s",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    cfg.wannier.outer_window_ev = None;
    cfg.wannier.frozen_window_ev = None;
    cfg.wannier.frozen_max_ev = Some(8.8);
    cfg.wannier.localization.max_iterations = 500;
    cfg.interpolation.k_grid = [40; 3];
    cfg.interpolation.q_grid = [20; 3];
    cfg.interpolation.fsthick_ev = 0.4;
    cfg.interpolation.fermi_source = FermiSource::FineFixedElectrons;
    cfg.superconductivity.mu_star = 0.05;
    cfg.superconductivity.frequency_cutoff_ev = 0.5;
    cfg
}

fn field(input: &str, name: &str) -> String {
    regex::Regex::new(&format!(r"(?m)^\s*{}\s*=\s*([^\n]+)", regex::escape(name)))
        .unwrap()
        .captures(input)
        .unwrap_or_else(|| panic!("missing {name}: {input}"))[1]
        .trim()
        .into()
}

#[test]
fn mori_fsr_parameters_use_native_coarse_and_fine_grid_contracts() {
    use epwflow::interpolation::epw_config::{EpwConfig, NativeStage};
    let cfg = mori_config();
    cfg.validate().unwrap();
    let ec = EpwConfig::from_config(&cfg)
        .unwrap()
        .with_bands(&[vec![-4., 0., 1., 2., 10., 11., 12., 13., 14., 15., 16., 17., 18., 19., 20., 21.]])
        .unwrap()
        .with_fermi(7.5).unwrap()
        .with_species(&["Mg".into(), "B".into()])
        .unwrap();
    let coarse = ec
        .native_input(NativeStage::CoarseQuality, "./tmp", "../phonon/save", 0.5)
        .unwrap();
    assert_eq!(field(&coarse, "prefix"), "'mgb2'");
    assert_eq!(field(&coarse, "nk1"), "6");
    assert_eq!(field(&coarse, "nq1"), "3");
    assert_eq!(field(&coarse, "nbndsub"), "5");
    assert_eq!(field(&coarse, "amass(1)").parse::<f64>().unwrap(), 24.305);
    assert_eq!(field(&coarse, "amass(2)").parse::<f64>().unwrap(), 10.811);
    assert_eq!(field(&coarse, "dis_froz_max").parse::<f64>().unwrap(), 8.8);
    assert_eq!(field(&coarse, "proj(2)"), "'f=0.5,1.0,0.5:s'");
    assert!(!coarse.contains("dis_froz_min") && !coarse.contains("dis_win_min"));
    assert!(
        !coarse.contains("bands_skipped")
            && !coarse.contains("energy_reference")
            && !coarse.contains("seedname")
    );
    assert_eq!(field(&coarse, "band_plot"), ".true.");
    assert_eq!(field(&coarse, "ephwrite"), ".false.");
    let fine = ec.epw_fine_input("./tmp", 0.5).unwrap();
    for (n, v) in [
        ("nk1", "6"),
        ("nq1", "3"),
        ("nkf1", "40"),
        ("nqf1", "20"),
        ("nqsmear", "1"),
    ] {
        assert_eq!(field(&fine, n), v);
    }
    assert_eq!(field(&fine, "mp_mesh_k"), ".true.");
    assert_eq!(field(&fine, "phonselfen"), ".false.");
    assert_eq!(field(&fine, "eliashberg"), ".true.");
    assert_eq!(field(&fine, "laniso"), ".false.", "准备谱不执行温度求解");
    assert_eq!(
        field(&fine, "a2f"),
        ".false.",
        "必须写出原生 freq 数据供超导重读"
    );
    assert_eq!(
        field(&fine, "degaussq").parse::<f64>().unwrap(),
        0.5,
        "meV 不能误除以 1000"
    );
}

#[test]
fn native_windows_require_bound_fermi_and_reject_fortran_truncation() {
    use epwflow::{
        foundation::config::EnergyReference,
        interpolation::epw_config::{EpwConfig, NativeStage},
    };
    let mut cfg = mori_config();
    cfg.wannier.energy_reference = EnergyReference::F2Fermi;
    let ec = EpwConfig::from_config(&cfg)
        .unwrap()
        .with_bands(&[vec![-4., 0., 1., 2., 10., 11., 12., 13., 14., 15., 16., 17., 18., 19., 20., 21.]])
        .unwrap()
        .with_species(&["Mg".into(), "B".into()])
        .unwrap();
    assert!(ec.epw_coarse_input("./tmp").is_err());
    let ec = ec.with_fermi(7.5).unwrap();
    let input = ec.epw_coarse_input("./tmp").unwrap();
    assert!((field(&input, "dis_froz_max").parse::<f64>().unwrap() - 10.0).abs() < 1e-12,
        "相对覆盖也必须受上游态数上限约束");
    assert!(ec
        .native_input(NativeStage::CoarseQuality, "./tmp", &"a".repeat(100), 0.5)
        .is_err());
    assert!(ec
        .native_input(
            NativeStage::CoarseQuality,
            "./tmp",
            "bad\n prefix = 'other'",
            0.5
        )
        .is_err());
    let input = ec
        .native_input(NativeStage::CoarseQuality, "./tmp", "../a'b", 0.5)
        .unwrap();
    assert_eq!(field(&input, "dvscf_dir"), "'../a''b'");
    cfg.wannier.frozen_window_ev = Some([0., 8.8]);
    assert!(cfg.validate().is_err(), "两种冻结能窗不能同时指定");
}

#[test]
fn diagnostic_points_include_all_fixed_q_without_changing_fermi_window() {
    use epwflow::interpolation::epw_config::EpwConfig;
    let root = std::env::temp_dir().join(format!(
        "native-points-{}",
        epwflow::foundation::store::token()
    ));
    fs::create_dir(&root).unwrap();
    let mut cfg = mori_config();
    cfg.interpolation.quality.q_points_crystal =
        vec![[0.; 3], [1.0 / 12.0, 0., 0.], [1.0 / 6.0, 0., 1.0 / 6.0]];
    let epw = EpwConfig::from_config(&cfg)
        .unwrap()
        .with_bands(&[vec![-4., 0., 1., 2., 10., 11., 12., 13., 14., 15., 16., 17., 18., 19., 20., 21.]])
        .unwrap()
        .with_species(&["Mg".into(), "B".into()])
        .unwrap()
        .with_fermi(7.5)
        .unwrap();
    epw.write_quality_points(&root).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("selecq.fmt")).unwrap(),
        "3\n3\n1 2 3\n"
    );
    let q = fs::read_to_string(root.join("qpoints.dat")).unwrap();
    assert!(q.starts_with("3 crystal\n"));
    let input = epw.epw_coarse_input("./tmp").unwrap();
    assert_eq!(field(&input, "selecqread"), ".true.");
    assert!((field(&input, "fsthick").parse::<f64>().unwrap() - 0.4).abs() < 1e-12);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn phonon_quality_preserves_units_signs_and_all_modes() {
    use epwflow::interpolation::native_quality::phonon_quality;
    let reference = bands(vec![-0.1, 2.0, 3.0]);
    let mut fit = bands(vec![
        3.0 / 8.065543937,
        -0.1 / 8.065543937,
        2.0 / 8.065543937,
    ]);
    assert!(phonon_quality(&reference, &fit, 3).unwrap().passed);
    fit.values[0][0] = 52.0 / 8.065543937;
    let q = phonon_quality(&reference, &fit, 3).unwrap();
    assert!(!q.passed);
    assert!((q.max_error_cm1 - 49.0).abs() < 1e-12);
    assert!(phonon_quality(&reference, &bands(vec![0., 1.]), 3).is_err());
    assert!(
        phonon_quality(
            &bands(vec![-2., 2., 3.]),
            &bands(vec![-2. / 8.065543937, 2. / 8.065543937, 3. / 8.065543937]),
            3
        )
        .unwrap()
        .passed
            == false
    );
    assert!(phonon_quality(&reference, &bands(vec![f64::NAN, 1., 2.]), 3).is_err());
}

#[test]
fn revised_phonon_maximum_keeps_rms_and_stability_gates() {
    use epwflow::interpolation::{native_quality::phonon_quality, quality};
    // 稀疏的大偏差使最大误差与 RMS 两条独立规则都实际参与判断。
    let reference = bands(vec![100.0; 900]);
    let check = |maximum: f64, background: f64| {
        let mut values = vec![(100.0 + background) / 8.065543937; 900];
        values[0] = (100.0 + maximum) / 8.065543937;
        phonon_quality(&reference, &bands(values), 900).unwrap()
    };
    let measured = check(9.042613107, 0.0);
    assert!(measured.passed);
    assert_eq!(measured.max_error_limit_cm1, 50.0);
    assert_eq!(measured.rms_error_limit_cm1, 2.0);
    assert!(check(50.0, 0.0).passed);
    assert!(!check(50.01, 0.0).passed);
    assert!(!check(9.04, 2.1).passed);
    let reference_values = vec![100.0; 900];
    let mut fit = reference_values.clone();
    fit[0] += 9.04;
    let legacy = quality::evaluate(&reference_values, &fit, "QeZero").unwrap();
    assert_eq!(legacy.thresholds["frequency_max_error_cm1"], 50.0);
    fit[0] = 150.01;
    assert!(quality::evaluate(&reference_values, &fit, "QeZero").is_err());
}

#[test]
fn readonly_quality_cli_reports_numeric_failure_and_rejects_changed_sources() {
    use epwflow::{
        foundation::store,
        interpolation::native_quality::{QualitySources, SourceFile},
    };
    let root = std::env::temp_dir().join(format!("native-quality-{}", store::token()));
    fs::create_dir(&root).unwrap();
    let ks = "<ks_energies><k_point weight='1'>0 0 0</k_point><eigenvalues size='1'>0</eigenvalues></ks_energies>";
    fs::write(root.join("reference.xml"),format!("<output><atomic_structure nat='1'/><band_structure><nbnd>1</nbnd><nks>6</nks><fermi_energy>0</fermi_energy>{}</band_structure></output>",ks.repeat(6))).unwrap();
    fs::write(
        root.join("band.eig"),
        format!("&plot nbnd= 1, nks= 6 /\n{}", "0 0 0\n0\n".repeat(6)),
    )
    .unwrap();
    fs::write(
        root.join("reference.freq"),
        format!("&plot nbnd= 3, nks= 4 /\n{}", "0 0 0\n1 2 3\n".repeat(4)),
    )
    .unwrap();
    fs::write(
        root.join("phband.freq"),
        format!(
            "&plot nbnd= 3, nks= 4 /\n{}",
            "0 0 0\n0.123984198 0.247968397 10\n".repeat(4)
        ),
    )
    .unwrap();
    fs::write(root.join("frozen-plan.json"), "test-only-fixed-plan").unwrap();
    let file = |name: &str| SourceFile {
        path: name.into(),
        sha256: store::hash_file(&root.join(name)).unwrap(),
    };
    let sources = QualitySources {
        schema_version: 1,
        asr: "epw_simple_gamma_onsite".into(),
        window_ev: [-0.5, 0.5],
        electron_reference: file("reference.xml"),
        fermi_reference: file("reference.xml"),
        electron_interpolated: file("band.eig"),
        phonon_reference_cm1: file("reference.freq"),
        phonon_interpolated_mev: file("phband.freq"),
        provenance: vec![file("frozen-plan.json")],
    };
    fs::write(
        root.join("quality-reference.json"),
        serde_json::to_vec_pretty(&sources).unwrap(),
    )
    .unwrap();
    let before = store::hash_file(&root.join("reference.xml")).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_epwflow"))
            .env("PATH", "")
            .current_dir("/")
            .args([
                "interpolation",
                "check",
                "-w",
                root.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap()
    };
    let out = run();
    assert!(!out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["workflow_status"], "quality_failed");
    assert_eq!(v["electron"]["passed"], true);
    assert_eq!(v["phonon"]["passed"], false);
    assert_eq!(v["workflow_complete"], false);
    assert_eq!(
        store::hash_file(&root.join("reference.xml")).unwrap(),
        before
    );
    fs::write(root.join("band.eig"), "altered").unwrap();
    let out = run();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("指纹"));
    fs::remove_dir_all(root).unwrap();
}
