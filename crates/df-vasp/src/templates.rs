//! VASP INCAR 默认参数模板（逐字移植 `vaspflow/templates.py` 的各 `get_vasp_*_parameters_template`）。
//! 这些默认值在 `build_params` 中与用户 `[vasp_*]` 段递归合并（用户值优先）。

use qf_config::SectionMap;
use serde_json::{json, Value};

fn section(name: &str, obj: Value) -> SectionMap {
    let mut m = SectionMap::new();
    if let Value::Object(o) = obj {
        m.insert(name.to_string(), Value::Object(o));
    }
    m
}

pub fn vasp_scf() -> SectionMap {
    section(
        "vasp_scf",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_Job",
            "ALGO": "Normal",
            "ISYM": 2,
            "ISTART": 0,
            "ICHARG": 2,
            "PREC": "Accurate",
            "IBRION": -1,
            "LWAVE": ".TRUE.",
            "LCHARG": ".TRUE.",
            "NCORE": 4
        }),
    )
}

pub fn vasp_scf_converge() -> SectionMap {
    vasp_scf()
}

pub fn vasp_eband() -> SectionMap {
    section(
        "vasp_eband",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_Band",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "ISMEAR": 0,
            "SIGMA": 0.05,
            "LORBIT": 11,
            "LWAVE": ".TRUE.",
            "LCHARG": ".FALSE.",
            "NCORE": 4
        }),
    )
}

pub fn vasp_eledos() -> SectionMap {
    section(
        "vasp_eledos",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_DOS",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "ISMEAR": -5,
            "LORBIT": 11,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
            "NCORE": 4,
            "NEDOS": 2000
        }),
    )
}

pub fn vasp_fermi() -> SectionMap {
    section(
        "vasp_fermi",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_FermiSurface",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "NSW": 0,
            "ISMEAR": 0,
            "SIGMA": 0.05,
            "LORBIT": 11,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE."
        }),
    )
}

pub fn vasp_locpot() -> SectionMap {
    section(
        "vasp_locpot",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_LOCPOT",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "NSW": 0,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
            "NCORE": 4,
            "LVHAR": ".TRUE."
        }),
    )
}

pub fn vasp_charge() -> SectionMap {
    section(
        "vasp_charge",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_CHARGE",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "NSW": 0,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
            "NCORE": 4,
            "LORBIT": 11,
            "LPARD": ".TRUE."
        }),
    )
}

pub fn vasp_elf() -> SectionMap {
    section(
        "vasp_elf",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_Job",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "LELF": ".TRUE.",
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE."
        }),
    )
}

pub fn vasp_cohp_uniform() -> SectionMap {
    section(
        "vasp_cohp_uniform",
        json!({
            "kmesh": 0.025,
            "SYSTEM": "VASP_COHP",
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "SYMPREC": 1e-5,
            "EDIFF": 1e-8,
            "NELM": 200,
            "NELMIN": 2,
            "ISMEAR": -5,
            "PREC": "Accurate",
            "IBRION": -1,
            "LORBIT": 12,
            "LWAVE": ".TRUE.",
            "LCHARG": ".TRUE.",
            "ISYM": -1,
            "NCORE": 4,
            "NEDOS": 2000
        }),
    )
}

pub fn vasp_cohp_fatband() -> SectionMap {
    section(
        "vasp_cohp_fatband",
        json!({
            "SYSTEM": "VASP_COHP",
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "SYMPREC": 1e-5,
            "EDIFF": 1e-8,
            "NELM": 200,
            "ISMEAR": 0,
            "PREC": "Accurate",
            "IBRION": -1,
            "LORBIT": 12,
            "LWAVE": ".TRUE.",
            "LCHARG": ".TRUE.",
            "ISYM": 2,
            "NCORE": 4,
            "scf_kmesh": 0.03,
            "kpath_kmesh": 0.03,
            "kmesh_scheme": "Monkhorst-Pack"
        }),
    )
}

pub fn vasp_opt() -> SectionMap {
    section(
        "vasp_opt",
        vasp_opt_inner(),
    )
}

/// `vasp_opt` 内部参数表（不含包裹的 `vasp_opt` 键），供合并逻辑复用。
pub fn vasp_opt_inner() -> Value {
    json!({
        "kmesh": 0.025,
        "ISTART": 0,
        "ICHARG": 2,
        "ISYM": 2,
        "ENCUT": 800,
        "PREC": "Accurate",
        "SYMPREC": 1e-5,
        "NCORE": 1,
        "ISMEAR": 0,
        "SIGMA": 0.05,
        "NELM": 200,
        "NELMIN": 6,
        "EDIFF": 1e-7,
        "EDIFFG": -0.01,
        "NSW": 500,
        "IBRION": 2,
        "ISIF": 3,
        "POTIM": 0.01,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE.",
        "PSTRESS": 0.0,
        "KSPACING": Value::Null,
        "NPAR": Value::Null,
        "KPAR": Value::Null
    })
}

pub fn vasp_opt_stage1() -> Value {
    json!({
        "ISTART": 0,
        "ICHARG": 2,
        "ISYM": 2,
        "ENCUT": 400,
        "PREC": "Normal",
        "NCORE": 4,
        "KSPACING": 0.5,
        "ISMEAR": 0,
        "SIGMA": 0.2,
        "NELM": 200,
        "NELMIN": 6,
        "EDIFF": 1e-4,
        "EDIFFG": -0.1,
        "NSW": 100,
        "IBRION": 2,
        "ISIF": 2,
        "POTIM": 0.01,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE."
    })
}

pub fn vasp_opt_stage2() -> Value {
    json!({
        "ISTART": 0,
        "ICHARG": 2,
        "ISYM": 2,
        "ENCUT": 500,
        "PREC": "Normal",
        "NCORE": 4,
        "KSPACING": 0.4,
        "ISMEAR": 0,
        "SIGMA": 0.1,
        "NELM": 200,
        "NELMIN": 6,
        "EDIFF": 1e-5,
        "EDIFFG": -0.1,
        "NSW": 200,
        "IBRION": 2,
        "ISIF": 4,
        "POTIM": 0.1,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE."
    })
}

pub fn vasp_opt_stage3() -> Value {
    json!({
        "ISTART": 0,
        "ICHARG": 2,
        "ISYM": 2,
        "ENCUT": 600,
        "PREC": "Accurate",
        "NCORE": 1,
        "KSPACING": 0.3,
        "ISMEAR": 0,
        "SIGMA": 0.05,
        "NELM": 200,
        "NELMIN": 6,
        "EDIFF": 1e-6,
        "EDIFFG": -0.05,
        "NSW": 300,
        "IBRION": 2,
        "ISIF": 3,
        "POTIM": 0.05,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE."
    })
}

/// 合并 `vasp_opt` 默认参数与用户段，并提取三段 stage 模板，返回
/// `(vasp_opt, stage1, stage2, stage3)` 四个内部 `SectionMap`。
pub fn vasp_opt_merged(
    user_vasp_opt: &SectionMap,
    user_stages: &SectionMap,
) -> (SectionMap, SectionMap, SectionMap, SectionMap) {
    let merge = |default: Value, user: &SectionMap| -> SectionMap {
        let mut v = default;
        let user_val = Value::Object(
            user.iter()
                .map(|(k, val)| (k.clone(), val.clone()))
                .collect(),
        );
        qf_config::merge_recursive(&mut v, &user_val);
        match v {
            Value::Object(m) => m.into_iter().collect(),
            _ => SectionMap::new(),
        }
    };
    let stage_of = |stages: &SectionMap, key: &str| -> SectionMap {
        match stages.get(key) {
            Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => SectionMap::new(),
        }
    };
    let vopt = merge(vasp_opt_inner(), user_vasp_opt);
    let s1 = merge(vasp_opt_stage1(), &stage_of(user_stages, "stage1"));
    let s2 = merge(vasp_opt_stage2(), &stage_of(user_stages, "stage2"));
    let s3 = merge(vasp_opt_stage3(), &stage_of(user_stages, "stage3"));
    (vopt, s1, s2, s3)
}

/// 取出对应命令名的默认参数段（内部 `vasp_*` 子表）。
pub fn default_params(name: &str) -> SectionMap {
    match name {
        "vasp_scf" => vasp_scf(),
        "vasp_scf_converge" => vasp_scf_converge(),
        "vasp_eband" => vasp_eband(),
        "vasp_eledos" => vasp_eledos(),
        "vasp_fermi" => vasp_fermi(),
        "vasp_locpot" => vasp_locpot(),
        "vasp_charge" => vasp_charge(),
        "vasp_elf" => vasp_elf(),
        "vasp_cohp_uniform" => vasp_cohp_uniform(),
        "vasp_cohp_fatband" => vasp_cohp_fatband(),
        "vasp_opt" => vasp_opt(),
        _ => SectionMap::new(),
    }
}
