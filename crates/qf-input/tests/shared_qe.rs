use qf_config::Config;
use serde_json::json;

#[test]
fn nscf_sections_and_complete_grid() {
    let mut cfg = Config::default();
    cfg.qe_phonon.scf.insert("degauss".into(), json!(0.01));
    cfg.qe_phonon
        .scf
        .insert("occupations".into(), json!("smearing"));
    let p = qf_input::pw::build_nscf_pw_params(&cfg);
    assert_eq!(p.system["nosym"], true);
    assert_eq!(p.system["noinv"], true);
    assert_eq!(p.system["degauss"], 0.01);
    for key in ["nosym", "noinv", "degauss", "occupations"] {
        assert!(!p.control.contains_key(key));
        assert!(!p.electrons.contains_key(key));
    }
    let grid = qf_input::pw::uniform_kpoints_text([2, 3, 1], true).unwrap();
    let lines: Vec<_> = grid.lines().collect();
    assert_eq!(lines[1], "6");
    let points: Vec<Vec<f64>> = lines[2..]
        .iter()
        .map(|l| l.split_whitespace().map(|x| x.parse().unwrap()).collect())
        .collect();
    assert_eq!(points.len(), 6);
    assert!(points.iter().all(|p| p.len() == 4));
    assert!((points.iter().map(|p| p[3]).sum::<f64>() - 1.).abs() < 1e-14);
    assert_eq!(&points[5][..3], &[0.5, 2.0 / 3.0, 0.0]);
    assert!(qf_input::pw::uniform_kpoints_text([0, 1, 1], true).is_err());
}

#[test]
fn shared_ph_renderer_preserves_bound_prefix_and_outdir() {
    let cfg = serde_json::from_value(
        json!({"prefix":"wrong", "outdir":"wrong", "ldisp":true,"nq1":2,"nq2":2,"nq3":1}),
    )
    .unwrap();
    let input = qf_input::ph::render_inputph("MgB2", "./tmp", &cfg);
    assert_eq!(input.matches("prefix =").count(), 1);
    assert_eq!(input.matches("outdir =").count(), 1);
    assert!(input.contains("prefix = 'MgB2'"));
    assert!(input.contains("outdir = './tmp'"));
    assert!(input.ends_with("/\n"));
}

#[test]
fn shared_scf_preserves_explicit_mass_and_mesh_shift() {
    let root = std::env::temp_dir().join(format!("qf-shared-mass-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Si.UPF"), "fixture").unwrap();
    let mut cfg = Config::default();
    cfg.pp_dir = Some(root.to_string_lossy().into_owned());
    cfg.atomic_masses.insert("Si".into(), 29.1234);
    let entries = qf_input::species::resolve_atomic_species_entries(&cfg, &["Si".into()]).unwrap();
    assert_eq!(entries[0].1, 29.1234);
    cfg.qe_phonon.scf.insert("kpoints".into(), json!([2, 3, 4]));
    cfg.qe_phonon.scf.insert("k_shift".into(), json!([1, 0, 1]));
    let cell = qf_crystal::Structure { lattice: [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]], elements: vec!["Si".into()], counts: vec![1], positions: vec![[0.;3]], coord_type: "Direct".into(), formula: "Si".into() };
    assert!(qf_input::pw::scf_kpoints_text(&cell, &cfg.qe_phonon.scf).contains("2 3 4 1 0 1"));
    std::fs::remove_dir_all(root).unwrap();
}
