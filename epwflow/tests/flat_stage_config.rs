use epwflow::foundation::{
    config::Config,
    native_config::{SCF, WANNIER},
    store,
};
use serde_json::{json, Value};

fn fixture() -> Config {
    toml::from_str(include_str!("fixtures/input.toml")).unwrap()
}
fn edited(name: &str, key: &str, value: Value) -> Config {
    let mut document = serde_json::to_value(fixture()).unwrap();
    document["stages"][name][key] = value;
    serde_json::from_value(document).unwrap()
}

#[test]
fn every_scientific_call_is_flat_and_roundtrips_without_binding_tables() {
    let cfg = fixture();
    cfg.validate().unwrap();
    let text = toml::to_string_pretty(&cfg).unwrap();
    let value: toml::Value = toml::from_str(&text).unwrap();
    for (name, task) in value["stages"].as_table().unwrap() {
        assert!(
            task.as_table().unwrap().values().all(|v| !v.is_table()),
            "{name}"
        );
    }
    for name in [
        "3.qe-bands",
        "3.qe-bands-post",
        "3.qe-dos",
        "3.qe-pdos",
        "5.wannier90",
        "5.postw90-dos",
        "8.epw-iso",
        "8.epw-aniso",
    ] {
        assert!(value["stages"].get(name).is_some(), "{name}");
    }
    assert!(!text.contains("derived"));
    assert!(!text.contains("INPUTEPW"));
    assert!(!text
        .lines()
        .any(|line| line.starts_with("[stages.") && line.contains(".pw")));
    assert_eq!(
        cfg.stage_document().unwrap(),
        toml::from_str::<Config>(&text)
            .unwrap()
            .stage_document()
            .unwrap()
    );
}

#[test]
fn total_and_projected_dos_keep_independent_native_smearing_values() {
    let cfg = edited("3.qe-dos", "degauss", json!(0.0123));
    cfg.validate().unwrap();
    let total = cfg
        .apply_native(
            &["3.qe-electronic", "qe_eledos", "dos"],
            "&DOS\n degauss=1\n/\n",
        )
        .unwrap();
    let projected = cfg
        .apply_native(
            &["3.qe-electronic", "qe_eledos", "projwfc"],
            "&PROJWFC\n degauss=1\n/\n",
        )
        .unwrap();
    assert!(total.contains("degauss = 0.0123"), "{total}");
    assert!(total.starts_with("&DOS\n"));
    assert!(!projected.contains("degauss = 0.0123"));
    assert!(projected.starts_with("&PROJWFC\n"));
}

#[test]
fn deeply_nested_configuration_and_user_binding_registry_are_rejected() {
    let mut document = serde_json::to_value(fixture()).unwrap();
    document["stages"][SCF]["pw"] = json!({"SYSTEM":{"ecutwfc":40}});
    let error = serde_json::from_value::<Config>(document)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("stages.1.qe-scf.pw") && error.contains("Do not support multilayers anymore"),
        "{error}"
    );
    let mut document = serde_json::to_value(fixture()).unwrap();
    document["workflow"][SCF] = json!({"derived":{"nat":"arbitrary"}});
    assert!(serde_json::from_value::<Config>(document)
        .unwrap_err()
        .to_string()
        .contains("Automatic binding by internal registration"));
}

#[test]
fn manual_internal_bindings_and_aliases_are_rejected_with_real_toml_locations() {
    for (call, key, value) in [
        (SCF, "outdir", json!("./other")),
        (SCF, "nat", json!(3)),
        (WANNIER, "fermi_energy", json!(1.)),
    ] {
        let error = format!("{:#}", edited(call, key, value).validate().unwrap_err());
        assert!(
            error.contains(&format!("stages.{call}.{key}")) && error.contains("Dual definition"),
            "{error}"
        );
        assert!(
            !error.contains("pw.CONTROL") && !error.contains("INPUTEPW"),
            "{error}"
        );
    }
    let mut document = serde_json::to_value(fixture()).unwrap();
    document["stages"][SCF]["ecutwfc_ry"] = json!(40.);
    assert!(serde_json::from_value::<Config>(document)
        .unwrap_err()
        .to_string()
        .contains("stages.1.qe-scf.ecutwfc_ry"));
}

#[test]
fn clean_input_keeps_internal_rules_and_identity_in_parameter_receipts() {
    let root = std::env::temp_dir().join(format!("epw-flat-source-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let mut cfg = fixture();
    cfg.structure = root.join("POSCAR");
    std::fs::write(&cfg.structure, "source identity").unwrap();
    let input = root.join("scf.in");
    std::fs::write(
        &input,
        "&control\n outdir='./bound'\n/\n&system\n nat=3\n/\n",
    )
    .unwrap();
    cfg.configure_file(&[SCF, "pw"], &input).unwrap();
    let text = std::fs::read_to_string(&input).unwrap();
    assert!(text.starts_with("&control\n"), "{text}");
    assert!(!text.contains("Parameter Source") && !text.contains("Internal derivation rule"));
    assert!(!input.with_extension("parameters.json").exists());
    assert!(!root.join("parameter-map.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fine_spectrum_subtasks_are_also_independent_flat_calls() {
    let mut document = serde_json::to_value(fixture()).unwrap();
    let mut extra = document["stages"]["7.epw-fine"].clone();
    extra["degaussq"] = json!(2.);
    document["stages"]["7.epw-fine-smearing-0002"] = extra;
    let cfg: Config = serde_json::from_value(document).unwrap();
    cfg.validate().unwrap();
    assert_eq!(cfg.interpolation.phonon_smearings_mev, vec![0.5, 2.]);
    let path = cfg.fine_task(2.).unwrap();
    let path = path.iter().map(String::as_str).collect::<Vec<_>>();
    let input = cfg
        .apply_native(&path, "&inputepw\n degaussq=1\n/\n")
        .unwrap();
    assert!(input.contains("degaussq = 2"));
    assert!(input.starts_with("&inputepw\n") && !input.contains("Parameter Source"));
    let text = toml::to_string(&cfg).unwrap();
    assert!(!text.contains(".spectra."));
}

#[test]
fn all_native_namelists_keep_titles_and_cards_without_source_headers() {
    let cfg = fixture();
    let cases: &[(&[&str], &str)] = &[
        (&[SCF, "pw"], "CONTROL"),
        (&["2.qe-nscf", "pw"], "CONTROL"),
        (&["3.qe-electronic", "qe_eband", "bands"], "BANDS"),
        (&["3.qe-electronic", "qe_eledos", "dos"], "DOS"),
        (&["3.qe-electronic", "qe_eledos", "projwfc"], "PROJWFC"),
        (&["4.qe-phonon-index", "ph"], "INPUTPH"),
        (&["4.qe-phonon-index", "q2r"], "INPUT"),
        (&["4.qe-phonon-index", "dispersion", "matdyn"], "INPUT"),
        (&["4.qe-phonon-index", "dos", "matdyn"], "INPUT"),
        (&[WANNIER, "epw"], "INPUTEPW"),
        (&["6.epw-coarse", "epw"], "INPUTEPW"),
        (&["7.epw-fine", "epw"], "INPUTEPW"),
        (&["8.epw-superconductivity", "iso", "epw"], "INPUTEPW"),
        (&["8.epw-superconductivity", "aniso", "epw"], "INPUTEPW"),
    ];
    for (task, section) in cases {
        let original = format!("Native title\n! user note\n&{section}\n/\n0.0 0.0 0.0\n");
        let clean = cfg.apply_native(task, &original).unwrap();
        assert!(
            clean.starts_with("Native title\n! user note\n&"),
            "{task:?}"
        );
        assert!(clean.ends_with("0.0 0.0 0.0\n"), "{task:?}");
        assert!(!clean.contains("Parameter Source"), "{task:?}");
        let legacy = original.replace(
            &format!("&{section}"),
            &format!("! Parameter Source stages.old.parameter\n ! Parameter Source Internal extrapolation rules old\n&{section}"),
        );
        assert_eq!(cfg.apply_native(task, &legacy).unwrap(), clean, "{task:?}");
    }
}
