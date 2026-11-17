use epwflow::foundation::{
    config::Config,
    native_config::*,
    stages::{self, Step},
};
use serde_json::{json, Value};

fn original() -> Config {
    toml::from_str(include_str!("fixtures/input.toml")).unwrap()
}
fn compact() -> Value {
    let mut value = serde_json::to_value(original()).unwrap();
    let base = json!({"ATOMIC_SPECIES":[["Mg",24.305,"Mg.upf"],["B",10.811,"B.upf"]],
        "prefix":"MgB2","ecutwfc":40.0,"ecutrho":320.0,"Emin":-100.0,"Emax":100.0,"DeltaE":0.01});
    for (_, stage) in value["stages"].as_object_mut().unwrap() {
        let stage = stage.as_object_mut().unwrap();
        for (key, common) in base.as_object().unwrap() {
            if key == "ATOMIC_SPECIES" || stage.get(key) == Some(common) {
                stage.remove(key);
            }
        }
        for grid in ["nk", "nq", "nkf", "nqf"] {
            let keys = (1..=3).map(|i| format!("{grid}{i}")).collect::<Vec<_>>();
            if let Some(values) = keys
                .iter()
                .map(|k| stage.get(k).cloned())
                .collect::<Option<Vec<_>>>()
            {
                for key in keys {
                    stage.remove(&key);
                }
                stage.insert(grid.into(), Value::Array(values));
            }
        }
    }
    value["baseparams"] = base;
    value
}

#[test]
fn common_fields_and_arrays_preserve_native_values_inputs_and_scientific_parameters() {
    let old = original();
    let new: Config = serde_json::from_value(compact()).unwrap();
    new.validate().unwrap();
    assert_eq!(old.stage_document().unwrap(), new.stage_document().unwrap());
    for step in Step::ALL {
        assert_eq!(
            stages::parameters(&old, step).unwrap(),
            stages::parameters(&new, step).unwrap(),
            "{step:?}"
        );
        assert_eq!(
            stages::scientific_key(&old, step).unwrap(),
            stages::scientific_key(&new, step).unwrap()
        );
    }
    for (path, title) in [
        (vec![SCF, "pw"], "CONTROL"),
        (vec![NSCF, "pw"], "SYSTEM"),
        (vec![ELECTRONIC, "qe_eledos", "dos"], "DOS"),
        (vec![ELECTRONIC, "qe_eledos", "projwfc"], "PROJWFC"),
        (vec!["4.qe-phonon-index", "ph"], "INPUTPH"),
        (vec!["4.qe-phonon-index", "dos", "matdyn"], "INPUT"),
        (vec![FINE, "epw"], "INPUTEPW"),
        (vec![SUPER, "iso", "epw"], "INPUTEPW"),
    ] {
        let input = format!("&{title}\n/\n");
        assert_eq!(
            old.apply_native(&path, &input).unwrap(),
            new.apply_native(&path, &input).unwrap()
        );
    }
    let roundtrip = toml::to_string_pretty(&new).unwrap();
    let again: Config = toml::from_str(&roundtrip).unwrap();
    assert_eq!(
        new.stage_document().unwrap(),
        again.stage_document().unwrap()
    );
    let grouped = new.to_stage_toml().unwrap();
    assert_eq!(
        new.stage_document().unwrap(),
        toml::from_str::<Config>(&grouped)
            .unwrap()
            .stage_document()
            .unwrap()
    );
    assert!(grouped.contains("[baseparams]") && grouped.contains("nkf = ["));
    let external = serde_json::to_value(&again).unwrap();
    assert!(external["stages"][SCF].get("prefix").is_none());
    assert!(external["stages"][FINE].get("nkf1").is_none());
    assert!(external["stages"]["5.wannier90"].get("Emin").is_none());
    assert!(external["stages"]["4.qe-q2r"].get("prefix").is_none());
}

#[test]
fn stage_overrides_remain_independent_and_invalid_values_never_fall_back() {
    let mut value = compact();
    value["stages"]["3.qe-dos"]["DeltaE"] = json!(0.02);
    let cfg: Config = serde_json::from_value(value.clone()).unwrap();
    cfg.validate().unwrap();
    assert_eq!(
        cfg.native_value::<f64>(&[ELECTRONIC, "qe_eledos", "dos", "DOS", "DeltaE"])
            .unwrap(),
        0.02
    );
    assert_eq!(
        cfg.native_value::<f64>(&[ELECTRONIC, "qe_eledos", "projwfc", "PROJWFC", "DeltaE"])
            .unwrap(),
        0.01
    );
    let serialized = serde_json::to_value(&cfg).unwrap();
    assert_eq!(serialized["stages"]["3.qe-dos"]["DeltaE"], 0.02);
    value["stages"][SCF]["ecutwfc"] = json!("invalid");
    let error = serde_json::from_value::<Config>(value)
        .unwrap_err()
        .to_string();
    assert!(error.contains("stages.1.qe-scf.ecutwfc"), "{error}");
    let mut value = compact();
    value["stages"][NSCF]["prefix"] = json!("different");
    assert!(serde_json::from_value::<Config>(value)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn invalid_common_fields_species_and_grid_aliases_report_the_actual_location() {
    for (field, bad) in [
        ("nk", json!([1, 1, 1])),
        ("nk1", json!(1)),
        ("K_POINTS", json!([1, 1, 1])),
        ("dos_kmesh", json!([1, 1, 1])),
        ("outdir", json!("other")),
        ("ecutwfc", json!(-1)),
        ("DeltaE", json!(0)),
        ("prefix", json!("")),
        ("ATOMIC_SPECIES", json!([["Mg", 0, "Mg.upf"]])),
        ("ATOMIC_SPECIES", json!([["Mg", 24.3]])),
        ("ATOMIC_SPECIES", json!([["Mg", "24.3", "Mg.upf"]])),
        ("ATOMIC_SPECIES", json!([["Mg", 24.3, ""]])),
        (
            "ATOMIC_SPECIES",
            json!([["Mg", 24.3, "Mg.upf"], ["Mg", 24.3, "Mg.upf"]]),
        ),
    ] {
        let mut value = compact();
        value["baseparams"][field] = bad;
        let err = serde_json::from_value::<Config>(value)
            .unwrap_err()
            .to_string();
        assert!(err.contains(&format!("baseparams.{field}")), "{err}");
    }
    for bad in [
        json!([1, 2]),
        json!([1, 0, 2]),
        json!([1, -1, 2]),
        json!([1, 2.5, 2]),
        json!([1, 4294967296u64, 2]),
    ] {
        let mut value = compact();
        value["stages"][FINE]["nkf"] = bad;
        let err = serde_json::from_value::<Config>(value)
            .unwrap_err()
            .to_string();
        assert!(err.contains("stages.7.epw-fine.nkf"), "{err}");
    }
    let mut value = compact();
    value["stages"][FINE]["nkf1"] = json!(4);
    assert!(serde_json::from_value::<Config>(value)
        .unwrap_err()
        .to_string()
        .contains("cannot mix"));
    let mut value = compact();
    value["stages"]["3.qe-dos"]["nk"] = json!([1, 1, 1]);
    assert!(serde_json::from_value::<Config>(value)
        .unwrap_err()
        .to_string()
        .contains("stages.3.qe-dos.nk"));
}

#[test]
fn explicit_species_array_roundtrips_without_changing_cards() {
    let mut value = compact();
    let species = value["baseparams"]["ATOMIC_SPECIES"].take();
    value["baseparams"]
        .as_object_mut()
        .unwrap()
        .remove("ATOMIC_SPECIES");
    for stage in [SCF, NSCF, "3.qe-bands"] {
        value["stages"][stage]["ATOMIC_SPECIES"] = species.clone();
    }
    let cfg: Config = serde_json::from_value(value).unwrap();
    cfg.validate().unwrap();
    assert_eq!(
        original().stage_document().unwrap(),
        cfg.stage_document().unwrap()
    );
    assert!(serde_json::to_value(&cfg).unwrap()["stages"][NSCF]["ATOMIC_SPECIES"].is_array());
}

#[test]
#[ignore = "requires DFTFLOW_MGB2_BEFORE_INPUT and DFTFLOW_MGB2_CURRENT_INPUT; configuration and native inputs only"]
fn actual_mgb2_public_defaults_preserve_science_without_reading_calculation_data() {
    let before =
        std::fs::read_to_string(std::env::var("DFTFLOW_MGB2_BEFORE_INPUT").unwrap()).unwrap();
    let current =
        std::fs::read_to_string(std::env::var("DFTFLOW_MGB2_CURRENT_INPUT").unwrap()).unwrap();
    let old: Config = toml::from_str(&before).unwrap();
    let new: Config = toml::from_str(&current).unwrap();
    old.validate().unwrap();
    new.validate().unwrap();
    assert_eq!(old.stage_document().unwrap(), new.stage_document().unwrap());
    for step in Step::ALL {
        assert_eq!(
            stages::parameters(&old, step).unwrap(),
            stages::parameters(&new, step).unwrap()
        );
    }
    let external = serde_json::to_value(&new).unwrap();
    assert_eq!(external["stages"].as_object().unwrap().len(), 8);
    assert!(external["baseparams"]["ATOMIC_SPECIES"].is_array());
}
