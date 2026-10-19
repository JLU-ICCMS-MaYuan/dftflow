use epwflow::{
    foundation::config::Config,
    interpolation::epw_config::{EpwConfig, NativeStage},
};

fn cfg() -> Config {
    let mut c: Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
    c.wannier.num_wann = 3;
    c.wannier.outer_window_ev = None;
    c.wannier.frozen_window_ev = None;
    c.wannier.frozen_max_ev = None;
    c.qe.nscf.nbnd = 4;
    c
}

#[test]
fn comparison_defaults_preserve_legacy_binding_and_invalid_sampling_is_rejected() {
    let mut c: Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
    assert!(c.comparison.is_none());
    assert!(serde_json::to_value(&c).unwrap().get("comparison").is_none());
    c.comparison = Some(Default::default());
    c.validate().unwrap();
    for points in [0,1001] {
        c.comparison.as_mut().unwrap().points_per_edge=points;
        assert!(format!("{:#}",c.validate().unwrap_err()).contains("comparison.points_per_edge"));
    }
    c.comparison.as_mut().unwrap().points_per_edge=20;
    c.comparison.as_mut().unwrap().electronic_sigma_ev=f64::NAN;
    assert!(c.validate().is_err());
    c.comparison.as_mut().unwrap().electronic_sigma_ev=0.1;
    c.comparison.as_mut().unwrap().phonon_sigma_cm1=0.;
    assert!(c.validate().is_err());
}

#[test]
fn both_native_inputs_consume_same_upstream_selection() {
    let c = cfg();
    let ec = EpwConfig::from_config(&c)
        .unwrap()
        .with_fermi(0.)
        .unwrap()
        .with_bands(&[vec![-0.0002, 0.0002, 4., 6.]])
        .unwrap();
    let win = ec.wannier90_input().unwrap();
    let coarse = ec
        .native_input(NativeStage::CoarseQuality, "./out", "./save", 0.5)
        .unwrap();
    assert!(win.contains("dis_proj_min = 0.01") && win.contains("dis_proj_max = 0.95"));
    assert!(win.contains("dis_froz_proj = true"));
    assert!(coarse.contains("'dis_froz_proj = true'"));
    assert!(coarse.contains("'dis_proj_min = 0.01'") && coarse.contains("'dis_proj_max = 0.95'"));
    assert_eq!(
        ec.window_selection().unwrap().parameters["dis_froz_max"],
        2.
    );
    assert!(!win.contains("dis_win_min") && !coarse.contains("dis_win_min"));
    assert!(!win.contains("dis_froz_min") && !coarse.contains("dis_froz_min"));
}

#[test]
fn missing_bound_bands_cannot_silently_use_previous_fixed_window() {
    let c = cfg();
    let ec = EpwConfig::from_config(&c).unwrap().with_fermi(0.).unwrap();
    assert!(ec.wannier90_input().is_err());
    assert!(ec.epw_coarse_input("out").is_err());
    assert!(
        ec.epw_fine_input("out", 0.5).is_ok(),
        "重读细网格不重新选窗"
    );
}

#[test]
fn explicit_absolute_override_is_capped_instead_of_preserving_old_rule() {
    let mut c = cfg();
    c.wannier.energy_reference = epwflow::foundation::config::EnergyReference::QeZero;
    c.wannier.frozen_max_ev = Some(8.8);
    let ec = EpwConfig::from_config(&c)
        .unwrap()
        .with_fermi(0.)
        .unwrap()
        .with_bands(&[vec![-2., -1., 2., 3.], vec![-2., -0.5, 2.5, 6.]])
        .unwrap();
    let s = ec.window_selection().unwrap();
    assert_eq!(s.requested_frozen_max_ev, 8.8);
    assert_eq!(s.parameters["dis_froz_max"], 2.5);
    assert_eq!(s.parameters["dis_proj_max"], 0.95);
}

#[test]
fn energy_only_uses_four_bounds_and_existing_epw_without_projectability() {
    let mut c = cfg();
    let program = std::env::temp_dir().join(format!("epw-window-{}", epwflow::foundation::store::token()));
    std::fs::write(&program, "#!/bin/sh\nexit 0\n").unwrap();
    c.programs.epw.path = program.clone();
    c.wannier.selection_mode = qf_wannier::SelectionMode::EnergyOnly;
    let ec = EpwConfig::from_config(&c)
        .unwrap()
        .with_fermi(0.)
        .unwrap()
        .with_bands(&[vec![-2., -1., 3., 6.]])
        .unwrap();
    let selection = ec.window_selection().unwrap();
    assert_eq!(selection.mode, "energy_only");
    assert_eq!(
        selection.parameters["dis_froz_max"], 2.,
        "稀疏间隙不能将上界移到 LUMO+2"
    );
    assert_eq!(selection.parameters.len(), 4);
    for input in [
        ec.wannier90_input().unwrap(),
        ec.epw_coarse_input("out").unwrap(),
    ] {
        assert!(!input.contains("dis_proj_") && !input.contains("dis_froz_proj"));
        for bound in ["dis_win_min", "dis_win_max", "dis_froz_min", "dis_froz_max"] {
            assert!(input.contains(bound));
        }
        assert!(input.contains("conv_tol"));
    }
    // 同一程序没有组合参数标记；纯能量可用，组合模式仍拒绝。
    ec.require_window_support().unwrap();
    c.wannier.selection_mode = qf_wannier::SelectionMode::FixedPlusProjectability;
    assert!(EpwConfig::from_config(&c)
        .unwrap()
        .require_window_support()
        .is_err());
    std::fs::remove_file(program).unwrap();
}

#[test]
fn energy_only_rejects_target_loss_and_preserves_legacy_config_encoding() {
    let mut c = cfg();
    let old = toml::to_string(&c).unwrap();
    assert!(!old.contains("selection_mode"));
    assert!(toml::from_str::<Config>(
        &old.replace("[wannier]", "[wannier]\nselection_mode = 'unknown'")
    )
    .is_err());
    c.wannier.selection_mode = qf_wannier::SelectionMode::EnergyOnly;
    c.wannier.frozen_window_ev = Some([-0.1, 0.1]);
    c.validate().unwrap();
    let mut no_convergence = c.clone();
    no_convergence.wannier.disentanglement = None;
    assert!(no_convergence.validate().is_err());
    let ec = EpwConfig::from_config(&c)
        .unwrap()
        .with_fermi(0.)
        .unwrap()
        .with_bands(&[vec![-2., -1., 3., 6.]])
        .unwrap();
    assert!(ec
        .window_selection()
        .unwrap_err()
        .to_string()
        .contains("目标窗"));
    c.wannier.frozen_window_ev = Some([-0.5, 0.5]);
    c.interpolation.fsthick_ev = 0.6;
    let ec = EpwConfig::from_config(&c)
        .unwrap()
        .with_fermi(0.)
        .unwrap()
        .with_bands(&[vec![-2., -1., 3., 6.]])
        .unwrap();
    assert!(ec.window_selection().is_err(), "EPC 窗不能超出冻结窗");
    assert!(toml::to_string(&c).unwrap().contains("energy_only"));
}

#[test]
fn energy_only_lower_bound_survives_epw59_namelist_override() {
    let mut c = cfg();
    c.wannier.selection_mode = qf_wannier::SelectionMode::EnergyOnly;
    c.wannier.outer_window_ev = Some([-1.5, 6.1]);
    c.wannier.frozen_window_ev = Some([-1.5, 2.]);
    let ec = EpwConfig::from_config(&c).unwrap().with_fermi(0.).unwrap()
        .with_bands(&[vec![-2., -1., 3., 6.]]).unwrap();
    let expected = ec.window_selection().unwrap().parameters["dis_win_min"];
    let input = ec.epw_coarse_input("out").unwrap();
    // readin 将直接下界重设为 -9999；write_winfil 的 wdata 必须单独保留下界。
    assert!(!regex::Regex::new(r"(?m)^\s*dis_win_min\s*=").unwrap().is_match(&input));
    let extra = regex::Regex::new(r"(?m)^\s*wdata\(\d+\) = 'dis_win_min = ([^']+)'$").unwrap();
    let bounds = extra.captures_iter(&input).map(|v| v[1].parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(bounds, vec![expected]);
    assert_eq!(input.matches("dis_win_min").count(), 1);
    let standalone = ec.wannier90_input().unwrap();
    assert!(standalone.contains(&format!("dis_win_min = {expected:.16e}")));
}
