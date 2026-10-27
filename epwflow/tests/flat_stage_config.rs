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
    assert!(total.contains("stages.3.qe-dos.degauss"));
    assert!(!projected.contains("degauss = 0.0123"));
    assert!(projected.contains("stages.3.qe-pdos.degauss"));
}

#[test]
fn deeply_nested_configuration_and_user_binding_registry_are_rejected() {
    let mut document = serde_json::to_value(fixture()).unwrap();
    document["stages"][SCF]["pw"] = json!({"SYSTEM":{"ecutwfc":40}});
    let error = serde_json::from_value::<Config>(document)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("stages.1.qe-scf.pw") && error.contains("不再支持多层"),
        "{error}"
    );
    let mut document = serde_json::to_value(fixture()).unwrap();
    document["workflow"][SCF] = json!({"derived":{"nat":"arbitrary"}});
    assert!(serde_json::from_value::<Config>(document)
        .unwrap_err()
        .to_string()
        .contains("自动绑定由程序内部登记"));
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
            error.contains(&format!("stages.{call}.{key}")) && error.contains("双重定义"),
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
fn internal_rules_are_traced_in_generated_input_and_parameter_receipts() {
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
    assert!(
        text.contains("内部推导规则 stages.1.qe-scf.outdir"),
        "{text}"
    );
    assert!(text.contains("stages.1.qe-scf.ecutwfc"));
    assert!(!text.contains("workflow.derived") && !text.contains("pw.SYSTEM"));
    let receipt: Value = store::read(&input.with_extension("parameters.json")).unwrap();
    assert_eq!(receipt["configuration"], "stages.1.qe-scf");
    assert!(receipt["derived"].get("stages.1.qe-scf.nat").is_some());
    cfg.write_parameter_map(&root.join("parameter-map.json"))
        .unwrap();
    let map: Value = store::read(&root.join("parameter-map.json")).unwrap();
    assert!(map["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["configuration"] == "stages.1.qe-scf.ecutwfc"));
    assert!(map["derived"].get("stages.1.qe-scf.outdir").is_some());
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
    assert!(input.contains("stages.7.epw-fine-smearing-0002.degaussq"));
    let text = toml::to_string(&cfg).unwrap();
    assert!(!text.contains(".spectra."));
}
