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
