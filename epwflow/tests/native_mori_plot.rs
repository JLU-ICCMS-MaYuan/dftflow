//! Mori 风格分布的数值契约；合成状态只验证归箱与绘图，不代表 MgB2 物理结果。
use epwflow::{
    delivery::{
        native::{Data, GapState, ModeSummary},
        native_plot::gap_distributions,
    },
    foundation::config::Tc,
    superconductivity::search::{Search, Side},
};
use std::collections::BTreeMap;

fn fixture() -> Data {
    let mut search = Search::new(1000, 2000, 50).unwrap();
    search.observe(1000, Side::Superconducting).unwrap();
    search.observe(1500, Side::Uncertain).unwrap();
    search.observe(2000, Side::Normal).unwrap();
    let mode = ModeSummary {
        method: "合成 IO 验证".into(),
        thresholds: Tc {
            min_k: 10.,
            max_k: 20.,
            precision_k: 0.5,
            max_solves: 64,
            eigen_margin: 0.001,
            normal_gap_mev: 0.0001,
            super_gap_mev: 0.001,
        },
        search,
        complete: false,
        reason: None,
        attempt_count: 6,
        failed_temperatures_centikelvin: vec![],
    };
    let mut data = Data {
        schema_version: 1,
        run_id: "synthetic-plot-contract".into(),
        complete: false,
        modes: BTreeMap::from([("aniso".into(), mode)]),
        spectra: vec![],
        gaps: vec![],
        states: vec![],
        solve_parameters: BTreeMap::new(),
    };
    for t in [1000, 1500, 2000, 2100] {
        for seed in [1., 10.] {
            for (k, delta, weight) in [(1, 2.01, 0.25), (2, -8.01, 0.75)] {
                data.states.push(GapState {
                    mode: "aniso".into(),
                    temperature_centikelvin: t,
                    seed_mev: seed,
                    kpoint: k,
                    band: 1,
                    k_crystal: None,
                    weight,
                    energy_ev: 0.,
                    delta_mev: if t == 2000 { 0. } else { delta },
                    z: 1.5,
                });
            }
        }
    }
    data
}

#[test]
fn distribution_uses_fs_weights_once_and_never_interpolates_temperatures() {
    let mut data = fixture();
    let rows = gap_distributions(&data).unwrap();
    assert_eq!(rows.keys().copied().collect::<Vec<_>>(), [1000, 2000]);
    assert_eq!(rows[&1000].len(), 2);
    for (actual, expected) in rows[&1000].iter().zip([(2.025, 0.25), (8.025, 0.75)]) {
        assert!((actual.0 - expected.0).abs() < 1e-12);
        assert!((actual.1 - expected.1).abs() < 1e-12);
    }
    assert_eq!(rows[&2000], [(0.025, 1.)]);
    for bins in rows.values() {
        assert!((bins.iter().map(|x| x.1).sum::<f64>() - 1.).abs() < 1e-12);
    }
    for s in &mut data.states {
        s.delta_mev = -s.delta_mev;
        if s.seed_mev == 10. {
            s.weight = 100.;
            s.delta_mev = 100.;
        }
    }
    assert_eq!(gap_distributions(&data).unwrap(), rows);
    data.modes
        .get_mut("aniso")
        .unwrap()
        .failed_temperatures_centikelvin
        .push(1000);
    assert_eq!(
        gap_distributions(&data)
            .unwrap()
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        [2000]
    );
}

#[test]
fn distribution_rejects_duplicate_or_invalid_physical_states() {
    let mut data = fixture();
    data.states.push(data.states[0].clone());
    assert!(gap_distributions(&data).is_err());
    for weight in [-1., f64::NAN, f64::INFINITY] {
        let mut data = fixture();
        data.states[0].weight = weight;
        assert!(gap_distributions(&data).is_err());
    }
    let mut data = fixture();
    data.states.iter_mut().for_each(|s| s.weight = 0.);
    assert!(gap_distributions(&data).is_err());
    let mut data = fixture();
    data.states[0].delta_mev = f64::NAN;
    assert!(gap_distributions(&data).is_err());
}

#[test]
fn partial_redraw_retains_formats_and_requires_full_redraw_after_source_change() {
    use epwflow::{
        delivery::{
            comparison::Artifact,
            native::{self, Manifest},
            native_plot,
        },
        foundation::store,
        interpolation::manifest::Spectrum,
    };
    use std::fs;
    let root = std::env::temp_dir().join(format!("native-format-{}", store::token()));
    fs::create_dir(&root).unwrap();
    let mut data = fixture();
    data.spectra.push(Spectrum {
        smearing_id: "synthetic".into(),
        energy_mev: vec![0., 10., 20.],
        a2f: vec![0., 0.2, 0.1],
        lambda_integral: 0.35,
        lambda_sum: 0.35,
        cutoff_ev: 0.02,
    });
    let names = [
        "summary.json",
        "a2f.csv",
        "coupling.csv",
        "gap_summary.csv",
        "gap_states.csv",
        "tc.csv",
        "report.md",
    ];
    for name in &names[1..] {
        fs::write(root.join(name), "synthetic IO fixture\n").unwrap();
    }
    let write_bundle = |data: &Data| {
        store::write(&root.join("summary.json"), data).unwrap();
        let manifest = Manifest {
            schema_version: 1,
            kind: "epwflow-native-results".into(),
            run_id: data.run_id.clone(),
            complete: data.complete,
            files: names
                .iter()
                .map(|name| Artifact::capture(&root, name).unwrap())
                .collect(),
            sources: vec![],
        };
        store::write(&root.join("manifest.json"), &manifest).unwrap();
        native::load(&root).unwrap();
    };
    write_bundle(&data);
    let formats = ["png".into(), "svg".into(), "pdf".into()];
    native_plot::plot(&root, None, &formats).unwrap();
    let png = store::Identity::capture(&root.join("figures/a2f_coupling.png")).unwrap();
    let pdf = store::Identity::capture(&root.join("figures/a2f_coupling.pdf")).unwrap();
    native_plot::plot(&root, None, &["svg".into()]).unwrap();
    png.verify().unwrap();
    pdf.verify().unwrap();
    assert!(native_plot::check(&root).unwrap());
    native_plot::plot(&root, None, &formats).unwrap();
    assert!(native_plot::check(&root).unwrap());

    data.spectra[0].a2f[1] = 0.3;
    write_bundle(&data);
    let figures = store::Identity::capture(&root.join("figures/native-figures.json")).unwrap();
    let err = native_plot::plot(&root, None, &["svg".into()]).unwrap_err();
    assert!(err.to_string().contains("数据来源改变"));
    figures.verify().unwrap();
    native_plot::plot(&root, None, &formats).unwrap();
    assert!(native_plot::check(&root).unwrap());
    fs::write(root.join("figures/a2f_coupling.png"), "tampered").unwrap();
    assert!(native_plot::check(&root).is_err());
    assert!(native_plot::plot(&root, None, &formats).is_err());
    fs::remove_dir_all(root).unwrap();
}
