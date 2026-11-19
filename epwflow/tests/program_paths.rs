//! 程序路径／argv 的公开配置契约；不调用真实科学程序。
use epwflow::foundation::{config::Config, parallelism, stages::Step};
use serde_json::json;

fn config() -> Config {
    toml::from_str(include_str!("fixtures/input.toml")).unwrap()
}

#[test]
fn flat_paths_and_every_argument_list_roundtrip_without_program_subtables() {
    let mut cfg = config();
    cfg.programs.bands.as_mut().unwrap().path = "/separate/bands.x".into();
    cfg.programs.bands.as_mut().unwrap().args = vec!["-pd".into(), ".true.".into()];
    let text = toml::to_string_pretty(&cfg).unwrap();
    let value: toml::Value = toml::from_str(&text).unwrap();
    for name in ["pw", "ph", "epw", "bands", "dos", "projwfc", "q2r", "matdyn", "postw90"] {
        assert!(value["programs"].get(name).is_none());
        assert!(value["programs"][format!("{name}_x")].is_str());
        assert!(value["programs"][format!("{name}_args")].is_array());
    }
    let restored: Config = toml::from_str(&text).unwrap();
    assert_eq!(restored.programs.bands.as_ref().unwrap().args, ["-pd", ".true."]);
    assert!(restored.programs.dos.as_ref().unwrap().args.is_empty());
}

#[test]
fn omitted_arguments_mean_empty_and_missing_paths_are_never_inferred() {
    let mut value = serde_json::to_value(config()).unwrap();
    value["programs"].as_object_mut().unwrap().remove("dos_args");
    value["programs"].as_object_mut().unwrap().remove("matdyn_x");
    let cfg: Config = serde_json::from_value(value.clone()).unwrap();
    assert!(cfg.programs.dos.as_ref().unwrap().args.is_empty());
    assert!(cfg.programs.matdyn.is_none());
    value["programs"]["matdyn_args"] = json!(["-nk", "1"]);
    let error = serde_json::from_value::<Config>(value).unwrap_err().to_string();
    assert!(error.contains("matdyn_args") && error.contains("matdyn_x"), "{error}");
}

#[test]
fn old_program_tables_unknown_fields_and_wrong_argument_types_are_rejected() {
    for (name, val) in [("pw", json!({"path":"pw.x","args":[]})), ("unrecognized_x", json!("x")), ("q2r_args", json!("-nk 1"))] {
        let mut value = serde_json::to_value(config()).unwrap();
        value["programs"][name] = val;
        assert!(serde_json::from_value::<Config>(value).is_err(), "{name}");
    }
}

#[test]
fn epw_images_are_rejected_for_all_stages_but_ph_images_and_pool_aliases_work() {
    for flag in ["-ni", "-nimage", "-nimages", "-npot", "--nimage", "--nimages"] {
        let mut cfg = config();
        cfg.programs.launcher = vec!["mpirun".into(), "-np".into(), "2".into()];
        cfg.programs.epw.args = vec![flag.into(), "2".into()];
        assert!(cfg.validate().unwrap_err().to_string().contains("EPW multi-image"));
        for step in [Step::Wannier, Step::Coarse, Step::Fine, Step::Superconductivity] {
            assert!(parallelism::preflight(&cfg, step, false).unwrap_err().to_string().contains("EPW multi-image"));
        }
        cfg.programs.epw.args = vec![flag.into(), "1".into(), "-npool".into(), "2".into()];
        cfg.validate().unwrap();
        cfg.programs.ph.args = vec!["-nimage".into(), "2".into()];
        parallelism::preflight(&cfg, Step::Phonon, false).unwrap();
    }
    for pool in ["-nk", "-npool", "-npools"] {
        let mut cfg = config();
        cfg.programs.epw.args = vec![pool.into(), "1".into()];
        parallelism::preflight(&cfg, Step::Fine, false).unwrap();
    }
}
