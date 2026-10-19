use qf_wannier::{select_default, Bounds, Reference};

#[test]
fn defaults_keep_unspecified_bounds_and_shift_metal_by_fermi() {
    let bands = vec![
        vec![-0.0002, 0.0002, 4., 6.],
        vec![-0.0001, 0.0003, 4.5, 7.],
    ];
    let s = select_default(
        &bands,
        0.,
        3,
        &[],
        &Bounds::default(),
        Reference::UpstreamRelative,
    )
    .unwrap();
    assert_eq!(s.shift_ev, 0.);
    assert_eq!(s.parameters["dis_froz_max"], 2.);
    assert_eq!(s.parameters["dis_proj_min"], 0.01);
    assert_eq!(s.parameters["dis_proj_max"], 0.95);
    assert!(!s.parameters.contains_key("dis_win_min"));
    assert!(!s.parameters.contains_key("dis_win_max"));
    assert!(!s.parameters.contains_key("dis_froz_min"));
    assert_eq!(s.outer_counts, vec![4, 4]);
    assert_eq!(s.energy_frozen_counts, vec![2, 2]);
}

#[test]
fn gap_shift_and_both_cap_terms_follow_upstream() {
    let b = vec![vec![-2., -1., 2., 2.5], vec![-2., -0.8, 3., 6.]];
    let s = select_default(
        &b,
        0.,
        3,
        &[],
        &Bounds::default(),
        Reference::UpstreamRelative,
    )
    .unwrap();
    assert_eq!(s.shift_ev, 2.);
    assert_eq!(s.requested_frozen_max_ev, 4.);
    assert_eq!(s.frozen_cap_ev, 2.4999);
    let s = select_default(
        &b,
        0.,
        2,
        &[],
        &Bounds::default(),
        Reference::UpstreamRelative,
    )
    .unwrap();
    assert_eq!(s.frozen_cap_ev, -0.8, "不能省略第 N 带最大值项");
}

#[test]
fn exclusion_uses_original_one_based_indices_and_deduplicates() {
    let b = vec![vec![-10., -1., 1., 8.], vec![-9., -0.5, 2., 7.]];
    let s = select_default(
        &b,
        0.,
        2,
        &[1, 1],
        &Bounds::default(),
        Reference::UpstreamRelative,
    )
    .unwrap();
    assert_eq!(s.excluded_bands, vec![1]);
    assert_eq!(s.shift_ev, 1.);
    assert_eq!(s.frozen_cap_ev, 2.);
    assert_eq!(s.energy_frozen_counts, vec![2, 2]);
    assert!(select_default(&b, 0., 2, &[0], &Bounds::default(), Reference::Absolute).is_err());
    assert!(select_default(
        &b,
        0.,
        2,
        &[1, 2, 3],
        &Bounds::default(),
        Reference::Absolute
    )
    .is_err());
}

#[test]
fn overrides_are_shifted_once_and_still_capped() {
    let b = vec![vec![4., 6., 9., 11.], vec![4., 7., 10., 12.]];
    let bounds = Bounds {
        dis_froz_max: Some(1.),
        ..Bounds::default()
    };
    let a = select_default(&b, 5., 3, &[], &bounds, Reference::UpstreamRelative).unwrap();
    assert_eq!(a.parameters["dis_froz_max"], 7.);
    let f = select_default(&b, 5., 3, &[], &bounds, Reference::Fermi).unwrap();
    assert_eq!(f.parameters["dis_froz_max"], 6.);
    let too_high = Bounds {
        dis_froz_max: Some(99.),
        ..Bounds::default()
    };
    let c = select_default(&b, 5., 3, &[], &too_high, Reference::Absolute).unwrap();
    assert_eq!(c.parameters["dis_froz_max"], 10.);
    let invalid = Bounds {
        dis_froz_min: Some(10.5),
        ..too_high
    };
    assert!(select_default(&b, 5., 3, &[], &invalid, Reference::Absolute).is_err());
}

#[test]
fn malformed_data_and_impossible_outer_window_are_rejected() {
    for b in [
        vec![],
        vec![vec![]],
        vec![vec![1., 0.]],
        vec![vec![f64::NAN]],
        vec![vec![-1., 1.], vec![1.]],
    ] {
        assert!(select_default(
            &b,
            0.,
            1,
            &[],
            &Bounds::default(),
            Reference::UpstreamRelative
        )
        .is_err());
    }
    let b = vec![vec![-2., -1., 1., 4.]];
    let bounds = Bounds {
        dis_win_min: Some(0.),
        dis_win_max: Some(2.),
        dis_froz_max: Some(1.),
        ..Bounds::default()
    };
    assert!(select_default(&b, 0., 2, &[], &bounds, Reference::Absolute).is_err());
    assert!(select_default(&b, 9., 2, &[], &Bounds::default(), Reference::Absolute).is_err());
    assert!(select_default(&b, -9., 2, &[], &Bounds::default(), Reference::Absolute).is_err());
}

#[test]
fn keyword_gate_is_conservative_and_handles_block_boundaries() {
    let p = std::env::temp_dir().join(format!("qf-wannier-keywords-{}", std::process::id()));
    std::fs::write(&p, "Wannier90 3.1.0").unwrap();
    assert!(qf_wannier::require_projectability_keywords(&p).is_err());
    let mut data = vec![0u8; 65530];
    data.extend_from_slice(b"DIS_PROJ_MIN dis_proj_max dis_froz_proj");
    std::fs::write(&p, data).unwrap();
    qf_wannier::require_projectability_keywords(&p).unwrap();
    std::fs::remove_file(p).unwrap();
}

#[test]
fn qe_xml_requires_complete_nonmagnetic_output() {
    let p = std::env::temp_dir().join(format!("qf-wannier-xml-{}", std::process::id()));
    let xml = "<espresso><output><band_structure><nbnd>2</nbnd><nks>1</nks><fermi_energy>0D0</fermi_energy><lsda>false</lsda><ks_energies><eigenvalues size=\"2\">-1D0 1D0</eigenvalues></ks_energies></band_structure></output></espresso>";
    std::fs::write(&p, xml).unwrap();
    let (b, f) = qf_wannier::read_qe_bands(&p).unwrap();
    assert_eq!(b[0][1], 27.211386245988);
    assert_eq!(f, 0.);
    for invalid in [
        xml.replace("<nks>1", "<nks>2"),
        xml.replace("size=\"2\"", "size=\"3\""),
        xml.replace("<lsda>false", "<lsda>true"),
    ] {
        std::fs::write(&p, invalid).unwrap();
        assert!(qf_wannier::read_qe_bands(&p).is_err());
    }
    std::fs::remove_file(p).unwrap();
}
