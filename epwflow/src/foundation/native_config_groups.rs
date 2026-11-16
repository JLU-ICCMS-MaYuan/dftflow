//! 八个主阶段的展示适配；参数仍由既有科学调用白名单校验。
use super::{
    calls, phonon_name, put, Call, Document, Value, COARSE, ELECTRONIC, FINE, NSCF, SCF, SUPER,
    WANNIER,
};
use anyhow::{ensure, Context, Result};

/// 同一映射服务于读取、输出和错误位置；不改变内部调用路径。
fn path(call: &Call) -> Vec<String> {
    let owner = call.path[0].as_str();
    let names: Vec<&str> = match call.name.as_str() {
        "3.qe-bands" => vec![ELECTRONIC],
        "3.qe-bands-post" => vec![ELECTRONIC, "bands"],
        "3.qe-dos" => vec![ELECTRONIC, "dos"],
        "3.qe-pdos" => vec![ELECTRONIC, "projwfc"],
        "4.qe-q2r" => vec![owner, "q2r"],
        "4.qe-dispersion" => vec![owner, "dispersion"],
        "4.qe-phdos" => vec![owner, "dos"],
        "5.wannier90" => vec![WANNIER, "wannier90"],
        "5.postw90-dos" => vec![WANNIER, "postw90"],
        "6.qe-q2r" => vec![COARSE, "q2r"],
        "6.qe-matdyn" => vec![COARSE, "matdyn"],
        "6.epw-comparison" => vec![COARSE, "comparison"],
        "8.epw-iso" => vec![SUPER, "iso"],
        "8.epw-aniso" => vec![SUPER, "aniso"],
        name if owner == FINE && name.starts_with(&format!("{FINE}-")) => {
            vec![FINE, "spectra", &name[FINE.len() + 1..]]
        }
        name if owner.starts_with("4.qe-phonon-") && name.ends_with("-q") => vec![owner, "q"],
        _ => vec![owner],
    };
    names.into_iter().map(str::to_owned).collect()
}

fn spectra(d: &Document, grouped: bool) -> Result<Vec<String>> {
    if !grouped {
        return Ok(d
            .stages
            .keys()
            .filter_map(|n| n.strip_prefix(&format!("{FINE}-")).map(str::to_owned))
            .collect());
    }
    match d.stages.get(FINE).and_then(|v| v.get("spectra")) {
        None => Ok(vec![]),
        Some(v) => Ok(v
            .as_object()
            .context("configuration: stages.7.epw-fine.spectra: 必须是具名子任务表")?
            .keys()
            .cloned()
            .collect()),
    }
}

pub(super) fn ungroup(mut d: Document) -> Result<Document> {
    let ph = phonon_name(&d, false)?;
    let expected = [
        SCF,
        NSCF,
        ELECTRONIC,
        ph.as_str(),
        WANNIER,
        COARSE,
        FINE,
        SUPER,
    ];
    ensure!(
        d.stages.len() == expected.len() && expected.iter().all(|n| d.stages.contains_key(*n)),
        "configuration: stages: 八主阶段配置只能包含 {:?}，不接受旧平层调用段或未知阶段",
        expected
    );
    for (name, value) in &d.stages {
        ensure!(value.is_object(), "configuration: stages.{name}: 必须是表");
    }
    ensure!(!d.workflow.contains_key("3.qe-bands"),
        "configuration: workflow.3.qe-bands: 八主阶段配置应使用 workflow.3.qe-electronic，不能混写旧名称");
    if let Some(value) = d.workflow.remove(ELECTRONIC) {
        d.workflow.insert("3.qe-bands".into(), value);
    }
    let calls = calls(&ph, &spectra(&d, true)?);
    let mut source = serde_json::to_value(&d.stages)?;
    let mut flat = std::collections::BTreeMap::new();
    for call in calls {
        let path = path(&call);
        let mut node = Some(&mut source);
        for part in &path[..path.len() - 1] {
            node = node.and_then(|v| v.get_mut(part));
        }
        let Some(node) = node else {
            continue;
        };
        let key = path.last().unwrap();
        let value = if path.len() == 1 {
            let table = node
                .get_mut(key)
                .and_then(Value::as_object_mut)
                .context("configuration: 主阶段必须是表")?;
            let keys = table
                .iter()
                .filter(|(_, v)| !v.is_object())
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>();
            let mut values = serde_json::Map::new();
            for key in keys {
                values.insert(key.clone(), table.remove(&key).unwrap());
            }
            Some(Value::Object(values))
        } else {
            node.as_object_mut().and_then(|o| o.remove(key))
        };
        if let Some(value) = value {
            ensure!(
                value.is_object(),
                "configuration: stages.{}: 用途子段必须是表",
                path.join(".")
            );
            flat.insert(call.name, value);
        }
    }
    if source[FINE]
        .get("spectra")
        .and_then(Value::as_object)
        .is_some_and(|o| o.is_empty())
    {
        source[FINE].as_object_mut().unwrap().remove("spectra");
    }
    for name in expected {
        let remaining = source[name].as_object().unwrap();
        ensure!(
            remaining.is_empty(),
            "configuration: stages.{name}: 未知用途子段 {:?}；不接受程序或 namelist 的额外嵌套",
            remaining.keys().collect::<Vec<_>>()
        );
    }
    d.stages = flat;
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{
        config::{Config, PhMode},
        stages::{self, Step},
        store,
    };

    fn legacy() -> Config {
        toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap()
    }
    fn grouped(cfg: &Config) -> Config {
        toml::from_str(&cfg.to_stage_toml().unwrap()).unwrap()
    }

    #[test]
    fn eight_stage_roundtrip_preserves_all_calls_and_legacy_serialization() {
        let old = legacy();
        let original = toml::to_string_pretty(&old).unwrap();
        let new = grouped(&old);
        new.validate().unwrap();
        assert_eq!(old.stage_document().unwrap(), new.stage_document().unwrap());
        assert_eq!(toml::to_string_pretty(&old).unwrap(), original);
        let value = serde_json::to_value(&new).unwrap();
        assert_eq!(value["stages"].as_object().unwrap().len(), 8);
        for name in [
            SCF,
            NSCF,
            ELECTRONIC,
            "4.qe-phonon-index",
            WANNIER,
            COARSE,
            FINE,
            SUPER,
        ] {
            assert!(value["stages"].get(name).is_some(), "{name}");
        }
        assert!(value["stages"][SCF].get("pw").is_none());
        assert!(value["stages"][WANNIER]["wannier90"]
            .get("num_wann")
            .is_some());
        assert!(value["stages"][WANNIER]["postw90"].get("dos").is_some());
        assert!(value["workflow"].get(ELECTRONIC).is_some());
        assert!(value["workflow"].get("3.qe-bands").is_none());
        let again = grouped(&new);
        assert_eq!(
            new.stage_document().unwrap(),
            again.stage_document().unwrap()
        );
        assert_eq!(
            toml::to_string_pretty(&new).unwrap(),
            toml::to_string_pretty(&again).unwrap()
        );
    }

    #[test]
    fn conflicting_names_remain_independent_and_render_to_their_owners() {
        let cfg = grouped(&legacy());
        let mut value = serde_json::to_value(&cfg).unwrap();
        value["stages"][ELECTRONIC]["dos"]["degauss"] = serde_json::json!(0.0123);
        value["stages"][ELECTRONIC]["projwfc"]["degauss"] = serde_json::json!(0.0456);
        value["stages"][SUPER]["iso"]["nsiter"] = serde_json::json!(120);
        value["stages"][SUPER]["aniso"]["nsiter"] = serde_json::json!(240);
        let edited: Config = serde_json::from_value(value).unwrap();
        edited.validate().unwrap();
        for (task, title, expected) in [
            (
                vec![ELECTRONIC, "qe_eledos", "dos"],
                "DOS",
                "degauss = 0.0123",
            ),
            (
                vec![ELECTRONIC, "qe_eledos", "projwfc"],
                "PROJWFC",
                "degauss = 0.0456",
            ),
            (vec![SUPER, "iso", "epw"], "INPUTEPW", "nsiter = 120"),
            (vec![SUPER, "aniso", "epw"], "INPUTEPW", "nsiter = 240"),
        ] {
            let input = edited
                .apply_native(&task, &format!("&{title}\n/\n"))
                .unwrap();
            assert!(input.contains(expected), "{task:?}: {input}");
        }
        for (path, expected) in [
            (vec![WANNIER, "wannier90", "bands_plot"], true),
            (
                vec![WANNIER, "electron_dos", "postw90", "bands_plot"],
                false,
            ),
            (vec![COARSE, "epw", "INPUTEPW", "epwread"], false),
            (
                vec![COARSE, "comparison", "epw", "INPUTEPW", "epwread"],
                true,
            ),
            (vec![SUPER, "iso", "epw", "INPUTEPW", "liso"], true),
            (vec![SUPER, "aniso", "epw", "INPUTEPW", "liso"], false),
        ] {
            assert_eq!(
                edited.native_value::<bool>(&path).unwrap(),
                expected,
                "{path:?}"
            );
        }
    }

    #[test]
    fn grouped_unknown_mixed_and_deep_tables_are_rejected_with_real_paths() {
        for case in ["mixed", "unknown", "deep", "workflow", "scalar", "stage"] {
            let mut value = serde_json::to_value(grouped(&legacy())).unwrap();
            match case {
                "mixed" => value["stages"]["5.wannier90"] = serde_json::json!({"num_iter":500}),
                "unknown" => value["stages"][WANNIER]["misspelled"] = serde_json::json!({}),
                "deep" => {
                    value["stages"][WANNIER]["wannier90"]["SYSTEM"] =
                        serde_json::json!({"num_iter":500})
                }
                "workflow" => value["workflow"]["3.qe-bands"] = serde_json::json!({}),
                "scalar" => value["stages"][WANNIER]["wannier90"] = serde_json::json!(false),
                "stage" => value["stages"]
                    .as_object_mut()
                    .unwrap()
                    .remove(NSCF)
                    .map(|_| ())
                    .unwrap(),
                _ => unreachable!(),
            }
            let error = serde_json::from_value::<Config>(value)
                .unwrap_err()
                .to_string();
            assert!(error.contains("configuration:"), "{case}: {error}");
            if case == "deep" {
                assert!(
                    error.contains("stages.5.epw-wannier.wannier90.SYSTEM"),
                    "{error}"
                );
            }
        }
        let mut value = serde_json::to_value(grouped(&legacy())).unwrap();
        value["stages"][WANNIER]["wannier90"]["num_iter"] = serde_json::json!(750);
        let cfg: Config = serde_json::from_value(value).unwrap();
        let error = format!("{:#}", cfg.validate().unwrap_err());
        assert!(
            error.contains("stages.5.epw-wannier.wannier90.num_iter"),
            "{error}"
        );
        assert!(!error.contains("stages.5.wannier90"), "{error}");
    }

    #[test]
    fn phonon_modes_and_named_fine_spectra_keep_their_native_views() {
        for mode in [PhMode::Nosplit, PhMode::Index, PhMode::Coords] {
            let mut cfg = legacy();
            cfg.phonon.mode = mode;
            // 先生成对应模式的外部旧配置再读取，避免保留原 index 的非活动逐 q 节点。
            let cfg: Config = toml::from_str(&toml::to_string_pretty(&cfg).unwrap()).unwrap();
            let new = grouped(&cfg);
            assert_eq!(cfg.stage_document().unwrap(), new.stage_document().unwrap());
            let mut external = serde_json::to_value(&new).unwrap();
            assert!(external["stages"].get(cfg.phonon_stage()).is_some());
            assert_eq!(
                external["stages"][cfg.phonon_stage()].get("q").is_some(),
                mode != PhMode::Nosplit
            );
            external["stages"][cfg.phonon_stage()]["q2r"]["unsupported"] = serde_json::json!(false);
            let error = serde_json::from_value::<Config>(external)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(&format!("stages.{}.q2r.unsupported", cfg.phonon_stage())),
                "{error}"
            );
        }
        let mut value = serde_json::to_value(legacy()).unwrap();
        let mut extra = value["stages"][FINE].clone();
        extra["degaussq"] = serde_json::json!(2.0);
        value["stages"]["7.epw-fine-smearing-0002"] = extra;
        let old: Config = serde_json::from_value(value).unwrap();
        let new = grouped(&old);
        new.validate().unwrap();
        assert_eq!(old.stage_document().unwrap(), new.stage_document().unwrap());
        assert_eq!(new.interpolation.phonon_smearings_mev, vec![0.5, 2.0]);
        assert!(
            serde_json::to_value(&new).unwrap()["stages"][FINE]["spectra"]
                .get("smearing-0002")
                .is_some()
        );
    }

    #[test]
    fn equivalent_layouts_preserve_native_inputs_and_every_scientific_key() {
        let root = std::env::temp_dir().join(format!("eight-stage-keys-{}", store::token()));
        std::fs::create_dir_all(&root).unwrap();
        let mut old = legacy();
        old.work_dir = root.join("run");
        old.structure = root.join("POSCAR");
        old.kpath = None;
        std::fs::write(&old.structure, "source").unwrap();
        old.programs.pw.path = root.join("native.x");
        old.programs.ph.path = root.join("native.x");
        old.programs.epw.path = root.join("native.x");
        std::fs::write(root.join("native.x"), "scientific program").unwrap();
        for path in old.pseudos.values_mut() {
            *path = root.join("pseudo.upf");
        }
        std::fs::write(root.join("pseudo.upf"), "pseudopotential").unwrap();
        let new = grouped(&old);
        for step in Step::ALL {
            assert_eq!(
                stages::parameters(&old, step).unwrap(),
                stages::parameters(&new, step).unwrap(),
                "{step:?}"
            );
            assert_eq!(
                stages::scientific_key(&old, step).unwrap(),
                stages::scientific_key(&new, step).unwrap(),
                "{step:?}"
            );
        }
        let original = old.stage_document().unwrap();
        for call in calls(&old.phonon_stage(), &[]) {
            for section in call
                .sections
                .iter()
                .filter(|s| !["cards", "wannier90", "postw90"].contains(s))
            {
                let input = format!("&{section}\n/\n");
                let refs = call.refs();
                assert_eq!(
                    old.apply_native(&refs, &input).unwrap(),
                    new.apply_native(&refs, &input).unwrap(),
                    "{}",
                    call.name
                );
            }
        }
        assert_eq!(new.stage_document().unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "显式提供 SiP2 原配置副本及当前配置；只读比较科学设置，不转换状态或提交作业"]
    fn real_sip2_grouped_configuration_is_read_only_and_scientifically_identical() {
        let current = std::path::PathBuf::from(
            std::env::var_os("DFTFLOW_SIP2_GROUPED_CONFIG").expect("current config"),
        );
        let before = std::path::PathBuf::from(
            std::env::var_os("DFTFLOW_SIP2_PREVIOUS_CONFIG").expect("previous config"),
        );
        let mut old: Config = toml::from_str(&std::fs::read_to_string(before).unwrap()).unwrap();
        old.validate().unwrap();
        old.resolve(current.parent().unwrap()).unwrap();
        let new = Config::load(&current).unwrap();
        assert_eq!(old.stage_document().unwrap(), new.stage_document().unwrap());
        for step in Step::ALL {
            assert_eq!(
                stages::scientific_key(&old, step).unwrap(),
                stages::scientific_key(&new, step).unwrap(),
                "{step:?}"
            );
        }
        for cfg in [&old, &new] {
            assert_eq!(
                serde_json::to_value(grouped(cfg)).unwrap()["stages"]
                    .as_object()
                    .unwrap()
                    .len(),
                8
            );
        }
        let qe = crate::qe_data::qe_config::QeSettings::from_config(&new).unwrap();
        let old_qe = crate::qe_data::qe_config::QeSettings::from_config(&old).unwrap();
        assert_eq!(
            qe.scf_input("./out").unwrap(),
            old_qe.scf_input("./out").unwrap()
        );
        assert_eq!(
            qe.nscf_input("./out").unwrap(),
            old_qe.nscf_input("./out").unwrap()
        );
        eprintln!("SiP2: eight stages; all scientific keys and SCF/NSCF inputs consistent; no state migration or submissions");
    }
}

pub(super) fn group(mut d: Document) -> Result<Document> {
    let ph = phonon_name(&d, false)?;
    let calls = calls(&ph, &spectra(&d, false)?);
    let mut result = serde_json::json!({});
    for call in calls {
        if let Some(value) = d.stages.remove(&call.name) {
            let path = path(&call);
            let refs = path.iter().map(String::as_str).collect::<Vec<_>>();
            put(&mut result, &refs, value);
        }
    }
    ensure!(
        d.stages.is_empty(),
        "configuration: 尚有未知调用无法归入八主阶段"
    );
    if let Some(value) = d.workflow.remove("3.qe-bands") {
        d.workflow.insert(ELECTRONIC.into(), value);
    }
    d.stages = serde_json::from_value(result)?;
    Ok(d)
}

pub(super) fn message(text: &str, ph: &str) -> String {
    let re = regex::Regex::new(r"(?:stages|workflow)\.[A-Za-z0-9_.-]+").expect("固定位置表达式");
    re.replace_all(text, |capture: &regex::Captures<'_>| {
        let input = &capture[0];
        if let Some(rest) = input.strip_prefix("workflow.3.qe-bands") {
            return format!("workflow.{ELECTRONIC}{rest}");
        }
        let extra = input
            .strip_prefix(&format!("stages.{FINE}-"))
            .and_then(|s| s.split('.').next())
            .map(|s| vec![s.to_owned()])
            .unwrap_or_default();
        let mut calls = calls(ph, &extra);
        calls.sort_by_key(|c| std::cmp::Reverse(c.name.len()));
        for call in calls {
            let prefix = format!("stages.{}", call.name);
            if input == prefix || input.starts_with(&(prefix.clone() + ".")) {
                return format!("stages.{}{}", path(&call).join("."), &input[prefix.len()..]);
            }
        }
        input.to_owned()
    })
    .into_owned()
}
