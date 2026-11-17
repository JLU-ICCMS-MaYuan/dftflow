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
    let mut workflow = serde_json::to_value(&*cfg).unwrap()["workflow"].clone();
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
        vec![SCF],
        vec![NSCF],
        vec!["3.qe-bands"],
    ] {
        let mut node = &mut *stages;
        for part in &path {
            node = &mut node[*part];
        }
        node["prefix"] = json!(prefix);
        node["ATOMIC_SPECIES"] = json!(species);
        node["ecutwfc"] = json!(cfg.qe.ecutwfc_ry);
        node["ecutrho"] = json!(cfg.qe.ecutrho_ry);
        node["occupations"] = json!(cfg.qe.occupations);
        for (name, val) in [
            ("smearing", json!(cfg.qe.smearing)),
            ("degauss", json!(cfg.qe.degauss_ry)),
        ] {
            if val.is_null() {
                node.as_object_mut().unwrap().remove(name);
            } else {
                node[name] = val;
            }
        }
    }
    stages[&ph]["prefix"] = json!(prefix);
    stages[&ph]["only_init"] = json!(cfg.phonon.mode != PhMode::Nosplit);
    if cfg.phonon.mode == PhMode::Nosplit {
        stages.as_object_mut().unwrap().remove(&format!("{ph}-q"));
    } else {
        let mut q = stages[&ph].clone();
        q["only_init"] = json!(false);
        if cfg.phonon.mode == PhMode::Coords {
            q["ldisp"] = json!(false);
            for k in ["nq1", "nq2", "nq3"] {
                q.as_object_mut().unwrap().remove(k);
            }
        }
        stages[format!("{ph}-q")] = q;
    }
    for name in ["3.qe-bands-post", "3.qe-dos", "3.qe-pdos"] {
        stages[name]["prefix"] = json!(prefix);
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
        vec![WANNIER],
        vec![COARSE],
        vec!["6.epw-comparison"],
        vec![FINE],
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
            stages[COARSE][format!("{base}{}", i + 1)] = json!(n);
        }
    }
    stages["6.qe-matdyn"]["amass"] = json!(masses);
    for stage in [WANNIER, COARSE] {
        workflow[stage]["points_per_edge"] =
            json!(cfg.comparison.clone().unwrap_or_default().points_per_edge);
    }
    stages[WANNIER]["num_iter"] = json!(cfg.wannier.localization.max_iterations);
    let owner = if cfg.superconductivity.mode == epwflow::foundation::config::Mode::Aniso {
        "aniso"
    } else {
        "iso"
    };
    let solver = stages[format!("8.epw-{owner}")].clone();
    for method in ["iso", "aniso"] {
        stages[format!("8.epw-{method}")] = solver.clone();
        let node = &mut stages[format!("8.epw-{method}")];
        node["prefix"] = json!(prefix);
        // 谱读取的测试请求与夹具文件行数一致；生产参数仍由各阶段独立设置。
        node["nqstep"] = json!(cfg.interpolation.spectral_points);
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
        if cfg.superconductivity.task == epwflow::foundation::config::SuperconductivityTask::Gap {
            stages[format!("8.epw-{method}")]["tc_linear"] = json!(false);
        } else {
            for key in ["gap_edge", "tc_linear"] { stages[format!("8.epw-{method}")].as_object_mut().unwrap().remove(key); }
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
            let mut native = stages[FINE].clone();
            native["degaussq"] = json!(smearing);
            stages[format!("{FINE}-{name}")] = native;
        }
    }
    v["workflow"] = workflow;
    *cfg = serde_json::from_value::<Config>(v).unwrap();
}
pub fn phonon(cfg: &mut Config, mode: PhMode) {
    cfg.phonon.mode = mode;
    matrix(cfg);
}
