//! 隔离进程测试的明确参数矩阵；仅在构造测试输入时逐项填写重复值。
//! 产品没有参数继承和旧配置转换。阶段独立性由 native_stage_config 单独检查。
#![allow(dead_code)]
use epwflow::foundation::{
    config::{Config, PhMode},
    native_config::*,
};
use serde_json::json;
pub fn matrix(cfg: &mut Config) {
    let mut v = serde_json::to_value(&*cfg).unwrap();
    let stages = &mut v["stages"];
    let ph = cfg.phonon_stage();
    let prefix = cfg.qe.prefix.clone();
    let species = cfg
        .pseudos
        .iter()
        .map(|(e, p)| {
            format!(
                "{e} {} {}",
                qf_input::species::get_atomic_mass(e, cfg.atomic_masses.get(e).copied()),
                p.file_name().unwrap().to_string_lossy()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    for path in [
        vec![SCF, "pw"],
        vec![NSCF, "pw"],
        vec![ELECTRONIC, "qe_eband", "pw"],
    ] {
        let mut node = &mut *stages;
        for part in &path {
            node = &mut node[*part];
        }
        node["CONTROL"]["prefix"] = json!(prefix);
        node["ATOMIC_SPECIES"] = json!(species);
        node["SYSTEM"]["ecutwfc"] = json!(cfg.qe.ecutwfc_ry);
        node["SYSTEM"]["ecutrho"] = json!(cfg.qe.ecutrho_ry);
        node["SYSTEM"]["occupations"] = json!(cfg.qe.occupations);
        for (name, val) in [
            ("smearing", json!(cfg.qe.smearing)),
            ("degauss", json!(cfg.qe.degauss_ry)),
        ] {
            if val.is_null() {
                node["SYSTEM"].as_object_mut().unwrap().remove(name);
            } else {
                node["SYSTEM"][name] = val;
            }
        }
    }
    stages[&ph]["ph"]["INPUTPH"]["prefix"] = json!(prefix);
    stages[&ph]["ph"]["INPUTPH"]["only_init"] = json!(cfg.phonon.mode != PhMode::Nosplit);
    if cfg.phonon.mode == PhMode::Nosplit {
        stages[&ph].as_object_mut().unwrap().remove("q");
    } else {
        let mut q = stages[&ph]["ph"]["INPUTPH"].clone();
        q["only_init"] = json!(false);
        if cfg.phonon.mode == PhMode::Coords {
            q["ldisp"] = json!(false);
            for k in ["nq1", "nq2", "nq3"] {
                q.as_object_mut().unwrap().remove(k);
            }
        }
        stages[&ph]["q"] = json!({"ph":{"INPUTPH":q}});
        let derived = &mut stages[&ph]["workflow"]["derived"];
        for k in ["start_q", "last_q"] {
            let name = format!("q.ph.INPUTPH.{k}");
            if cfg.phonon.mode == PhMode::Index {
                derived[&name] = json!("测试 q 点序号");
            } else {
                derived.as_object_mut().unwrap().remove(&name);
            }
        }
    }
    for task in [
        ["qe_eband", "bands", "BANDS"],
        ["qe_eledos", "dos", "DOS"],
        ["qe_eledos", "projwfc", "PROJWFC"],
    ] {
        stages[ELECTRONIC][task[0]][task[1]][task[2]]["prefix"] = json!(prefix);
    }
    let order = if cfg.pseudos.contains_key("Mg") && cfg.pseudos.contains_key("B") {
        vec!["Mg".to_owned(), "B".to_owned()]
    } else {
        cfg.pseudos.keys().cloned().collect()
    };
    let masses = order
        .iter()
        .map(|e| qf_input::species::get_atomic_mass(e, cfg.atomic_masses.get(e).copied()))
        .collect::<Vec<_>>();
    for path in [
        vec![WANNIER, "epw", "INPUTEPW"],
        vec![COARSE, "epw", "INPUTEPW"],
        vec![COARSE, "comparison", "epw", "INPUTEPW"],
        vec![FINE, "epw", "INPUTEPW"],
    ] {
        let mut node = &mut *stages;
        for p in path {
            node = &mut node[p];
        }
        node["prefix"] = json!(prefix);
        node["amass"] = json!(masses);
        node["nbndsub"] = json!(cfg.wannier.num_wann);
        for (base, grid) in [("nk", cfg.qe.nscf.k_grid), ("nq", cfg.phonon.q_grid)] {
            for (i, n) in grid.into_iter().enumerate() {
                node[format!("{base}{}", i + 1)] = json!(n);
            }
        }
        node["fsthick"] = json!(cfg.interpolation.fsthick_ev);
        node["efermi_read"] =
            json!(cfg.interpolation.fermi_source == epwflow::foundation::config::FermiSource::F2);
    }
    for (base, grid) in [("nkf", cfg.qe.nscf.k_grid), ("nqf", cfg.phonon.q_grid)] {
        for (i, n) in grid.into_iter().enumerate() {
            stages[COARSE]["epw"]["INPUTEPW"][format!("{base}{}", i + 1)] = json!(n);
        }
    }
    stages[COARSE]["comparison"]["matdyn"]["INPUT"]["amass"] = json!(masses);
    for stage in [WANNIER, COARSE] {
        stages[stage]["workflow"]["points_per_edge"] =
            json!(cfg.comparison.clone().unwrap_or_default().points_per_edge);
    }
    stages[WANNIER]["epw"]["INPUTEPW"]["num_iter"] = json!(cfg.wannier.localization.max_iterations);
    let owner = if cfg.superconductivity.mode == epwflow::foundation::config::Mode::Aniso {
        "aniso"
    } else {
        "iso"
    };
    let solver = stages[SUPER][owner]["epw"]["INPUTEPW"].clone();
    for method in ["iso", "aniso"] {
        stages[SUPER][method]["epw"]["INPUTEPW"] = solver.clone();
        let node = &mut stages[SUPER][method]["epw"]["INPUTEPW"];
        node["prefix"] = json!(prefix);
        node["liso"] = json!(method == "iso");
        node["laniso"] = json!(method == "aniso");
        node["nstemp"] = json!(cfg.superconductivity.gap_temperatures_k.len());
        for (base, grid) in [
            ("nkf", cfg.interpolation.k_grid),
            ("nqf", cfg.interpolation.q_grid),
        ] {
            for (i, n) in grid.into_iter().enumerate() {
                node[format!("{base}{}", i + 1)] = json!(n);
            }
        }
        let derived = &mut stages[SUPER]["workflow"]["derived"];
        for key in ["gap_edge", "tc_linear"] {
            let name = format!("{method}.epw.INPUTEPW.{key}");
            if cfg.superconductivity.task == epwflow::foundation::config::SuperconductivityTask::Gap
            {
                derived.as_object_mut().unwrap().remove(&name);
            } else {
                derived[&name] = json!("当前测试搜索请求");
            }
        }
        if cfg.superconductivity.task == epwflow::foundation::config::SuperconductivityTask::Gap {
            stages[SUPER][method]["epw"]["INPUTEPW"]["tc_linear"] = json!(false);
        }
    }
    if cfg.interpolation.phonon_smearings_mev.len() > 1 {
        for (index, smearing) in cfg
            .interpolation
            .phonon_smearings_mev
            .iter()
            .enumerate()
            .skip(1)
        {
            let name = format!("smearing-{:04}", index + 1);
            let mut native = stages[FINE]["epw"]["INPUTEPW"].clone();
            native["degaussq"] = json!(smearing);
            stages[FINE]["spectra"][&name] = json!({"epw":{"INPUTEPW":native}});
            let derived = stages[FINE]["workflow"]["derived"]
                .as_object()
                .unwrap()
                .clone();
            for (key, rule) in derived {
                if let Some(suffix) = key.strip_prefix("epw.") {
                    stages[FINE]["workflow"]["derived"][format!("spectra.{name}.epw.{suffix}")] =
                        rule;
                }
            }
        }
    }
    *cfg = serde_json::from_value::<Config>(v).unwrap();
}
pub fn phonon(cfg: &mut Config, mode: PhMode) {
    cfg.phonon.mode = mode;
    matrix(cfg);
}
