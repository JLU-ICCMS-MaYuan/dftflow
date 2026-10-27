use epwflow::{
    foundation::{
        config::Config,
        native_config::{self, COARSE, FINE, NSCF, SCF, SUPER, WANNIER},
        stages::{self, Step},
        store,
    },
    qe_data::qe_config::QeSettings,
};
use serde_json::json;
use std::{fs, path::PathBuf};

struct Case {
    root: PathBuf,
    cfg: Config,
}
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("native-stage-config-{}", store::token()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("POSCAR"),"MgB2\n1\n3 0 0\n-1.5 2.598 0\n0 0 3.5\nMg B\n1 2\nDirect\n0 0 0\n0.3333 0.6667 0.5\n0.6667 0.3333 0.5\n").unwrap();
        for name in ["Mg.upf", "B.upf", "pw.x", "ph.x", "epw.x"] {
            fs::write(root.join(name), "test source").unwrap();
        }
        let mut cfg: Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
        cfg.work_dir = root.join("run");
        cfg.outdir = root.join("scratch");
        cfg.structure = root.join("POSCAR");
        cfg.pseudos = [
            ("Mg".into(), root.join("Mg.upf")),
            ("B".into(), root.join("B.upf")),
        ]
        .into();
        cfg.programs.pw.path = root.join("pw.x");
        cfg.programs.ph.path = root.join("ph.x");
        cfg.programs.epw.path = root.join("epw.x");
        Self { root, cfg }
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn version_two_roundtrip_has_only_native_scientific_names() {
    let c = Case::new();
    c.cfg.validate().unwrap();
    let text = toml::to_string_pretty(&c.cfg).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    for key in [
        "qe",
        "phonon",
        "wannier",
        "interpolation",
        "superconductivity",
        "comparison",
        "atomic_masses",
    ] {
        assert!(parsed.get(key).is_none(), "{key}");
    }
    for key in [
        "ecutwfc_ry",
        "mu_star",
        "max_iterations",
        "frequency_cutoff_ev",
        "gap_temperatures_k",
    ] {
        assert!(!text.contains(&format!("{key} =")), "{key}");
    }
    let again: Config = toml::from_str(&text).unwrap();
    again.validate().unwrap();
    assert_eq!(
        c.cfg.stage_document().unwrap(),
        again.stage_document().unwrap()
    );
}

#[test]
fn nscf_system_parameters_are_independent_and_render_native_names_and_units() {
    let mut c = Case::new();
    let before = QeSettings::from_config(&c.cfg)
        .unwrap()
        .scf_input("./out")
        .unwrap();
    c.cfg
        .set_native(&[NSCF, "pw", "SYSTEM", "ecutwfc"], json!(48.))
        .unwrap();
    c.cfg
        .set_native(&[NSCF, "pw", "SYSTEM", "degauss"], json!(0.03))
        .unwrap();
    c.cfg.validate().unwrap();
    let settings = QeSettings::from_config(&c.cfg).unwrap();
    assert_eq!(before, settings.scf_input("./out").unwrap());
    let input = settings.nscf_input("./out").unwrap();
    assert!(input.contains("ecutwfc = 48"), "{input}");
    assert!(input.contains("degauss = 0.03"), "{input}");
    assert!(input.starts_with("&CONTROL\n"));
    assert!(!input.contains("参数来源"));
    assert!(!input.contains("ecutwfc_ry"));
}

#[test]
fn conflicting_coarse_mesh_identifies_both_configuration_locations() {
    let mut c = Case::new();
    c.cfg
        .set_native(&[COARSE, "epw", "INPUTEPW", "nk1"], json!(99))
        .unwrap();
    let error = format!("{:#}", c.cfg.validate().unwrap_err());
    assert!(
        error.contains("6.epw-coarse.nk1") && error.contains("2.qe-nscf.K_POINTS"),
        "{error}"
    );
}

#[test]
fn seventh_stage_temperature_is_not_owned_by_eighth_stage() {
    let mut c = Case::new();
    let before = stages::scientific_key(&c.cfg, Step::Fine).unwrap();
    c.cfg
        .set_native(
            &[SUPER, "iso", "epw", "INPUTEPW", "temps"],
            json!([6., 12., 18.]),
        )
        .unwrap();
    assert_eq!(before, stages::scientific_key(&c.cfg, Step::Fine).unwrap());
    assert_eq!(
        c.cfg
            .native_value::<Vec<f64>>(&[FINE, "epw", "INPUTEPW", "temps"])
            .unwrap(),
        vec![5.]
    );
}

#[test]
fn methods_keep_separate_solver_parameters() {
    let mut c = Case::new();
    c.cfg
        .set_native(&[SUPER, "aniso", "epw", "INPUTEPW", "muc"], json!(0.2))
        .unwrap();
    assert_eq!(
        c.cfg
            .solver_for(epwflow::foundation::config::Mode::Iso)
            .unwrap()
            .mu_star,
        0.1
    );
    assert_eq!(
        c.cfg
            .solver_for(epwflow::foundation::config::Mode::Aniso)
            .unwrap()
            .mu_star,
        0.2
    );
}

#[test]
fn old_versions_unknown_aliases_and_undefined_generated_parameters_are_rejected() {
    let c = Case::new();
    let text = toml::to_string(&c.cfg).unwrap();
    for version in [0, 1, 3] {
        assert!(toml::from_str::<Config>(
            &text.replace("schema_version = 2", &format!("schema_version = {version}"))
        )
        .is_err());
    }
    let mut invalid = c.cfg.clone();
    invalid
        .set_native(&[NSCF, "pw", "SYSTEM", "ecutwfc_ry"], json!(40.))
        .unwrap();
    assert!(format!("{:#}", invalid.validate().unwrap_err()).contains("ecutwfc_ry"));
    let input = "&system\n unconfigured_science=12\n/\n";
    assert!(native_config::apply(&c.cfg.stage_document().unwrap(), &[SCF, "pw"], input).is_err());
}

#[test]
fn declared_native_array_is_written_with_fortran_indices() {
    let c = Case::new();
    let input = "&inputepw\n amass(1)=1.0\n amass(2)=2.0\n/\n";
    let text = c.cfg.apply_native(&[WANNIER, "epw"], input).unwrap();
    assert!(
        text.contains("amass(1) = 24.305") && text.contains("amass(2) = 10.811"),
        "{text}"
    );
    let input = format!(
        "&inputepw\n{}/\n",
        (1..=12)
            .map(|n| format!("wdata({n})='row {n}'\n"))
            .collect::<String>()
    );
    let text = c.cfg.apply_native(&[WANNIER, "epw"], &input).unwrap();
    assert!(text.find(" wdata(9) =").unwrap() < text.find(" wdata(10) =").unwrap());
}

#[test]
fn manual_and_derived_definitions_are_rejected_before_execution() {
    let mut c = Case::new();
    c.cfg
        .set_native(&[SCF, "pw", "CONTROL", "outdir"], json!("./other"))
        .unwrap();
    let error = format!("{:#}", c.cfg.validate().unwrap_err());
    assert!(
        error.contains("1.qe-scf.outdir") && error.contains("双重定义"),
        "{error}"
    );
}

#[test]
fn native_boolean_and_array_types_and_temperature_lengths_are_checked() {
    for (path, value) in [
        (vec![NSCF, "pw", "SYSTEM", "nosym"], json!("false")),
        (
            vec![WANNIER, "epw", "INPUTEPW", "amass"],
            json!([24.305, "B"]),
        ),
        (vec![FINE, "epw", "INPUTEPW", "nstemp"], json!(2)),
    ] {
        let mut c = Case::new();
        c.cfg.set_native(&path, value).unwrap();
        let error = format!("{:#}", c.cfg.validate().unwrap_err());
        assert!(error.contains(&format!("{}.{}", path[0], path.last().unwrap())), "{error}");
    }
}

#[test]
fn crystal_card_is_preserved_and_incomplete_mesh_is_rejected() {
    let mut c = Case::new();
    let card: String = c.cfg.native_value(&[NSCF, "pw", "K_POINTS"]).unwrap();
    let mut reordered = card.lines().collect::<Vec<_>>();
    reordered.swap(2, 3);
    let card = reordered.join("\n");
    c.cfg
        .set_native(&[NSCF, "pw", "K_POINTS"], json!(card))
        .unwrap();
    let actual = QeSettings::from_config(&c.cfg)
        .unwrap()
        .nscf_input("./out")
        .unwrap();
    assert!(actual.contains(&format!("K_POINTS {card}")), "{actual}");
    let mut lines = card.lines().collect::<Vec<_>>();
    lines.pop();
    let error = c
        .cfg
        .set_native(&[NSCF, "pw", "K_POINTS"], json!(lines.join("\n")))
        .unwrap_err();
    assert!(format!("{error:#}").contains("2.qe-nscf.K_POINTS"));
}

#[test]
fn each_method_keeps_its_own_temperature_plan() {
    let mut c = Case::new();
    c.cfg
        .set_native(
            &[SUPER, "aniso", "epw", "INPUTEPW", "temps"],
            json!([12., 24.]),
        )
        .unwrap();
    c.cfg
        .set_native(&[SUPER, "aniso", "epw", "INPUTEPW", "nstemp"], json!(2))
        .unwrap();
    c.cfg.validate().unwrap();
    let iso = c
        .cfg
        .solver_for(epwflow::foundation::config::Mode::Iso)
        .unwrap();
    let aniso = c
        .cfg
        .solver_for(epwflow::foundation::config::Mode::Aniso)
        .unwrap();
    assert_eq!(iso.gap_temperatures_k, vec![5., 10., 15.]);
    assert_eq!(aniso.gap_temperatures_k, vec![12., 24.]);
}

#[test]
fn selecting_aniso_does_not_replace_the_iso_section() {
    let mut c=Case::new();
    c.cfg.set_native(&[SUPER,"aniso","epw","INPUTEPW","temps"],json!([12.,24.])).unwrap();
    c.cfg.set_native(&[SUPER,"aniso","epw","INPUTEPW","nstemp"],json!(2)).unwrap();
    c.cfg.set_native(&[SUPER,"workflow","mode"],json!("aniso")).unwrap();
    c.cfg.validate().unwrap();
    assert_eq!(c.cfg.solver_for(epwflow::foundation::config::Mode::Iso).unwrap().gap_temperatures_k,vec![5.,10.,15.]);
    assert_eq!(c.cfg.solver_for(epwflow::foundation::config::Mode::Aniso).unwrap().gap_temperatures_k,vec![12.,24.]);
}
