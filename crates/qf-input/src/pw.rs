//! `pw.x` 输入写出（复刻 `qeflow/input_deck/pw.py`）。

use anyhow::Result;
use qf_config::{Config, SectionMap};
use qf_crystal::kmesh::{get_kpoints_from_spacing, KmeshSpec};
use qf_crystal::lattice::{
    write_atomic_positions_with_precision, write_cell_parameters_with_precision, write_system_lattice,
};
use qf_crystal::Structure;
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::species::write_atomic_species;
use crate::to_qe_value;

const SCF_CONTROL_KEYS: &[&str] = &[
    "calculation",
    "restart_mode",
    "prefix",
    "outdir",
    "pseudo_dir",
    "verbosity",
    "tprnfor",
    "tstress",
    "forc_conv_thr",
    "etot_conv_thr",
];
const SCF_SYSTEM_KEYS: &[&str] = &["ecutwfc", "ecutrho", "occupations", "smearing", "degauss"];
const SCF_ELECTRON_KEYS: &[&str] = &["conv_thr", "mixing_beta", "electron_maxstep"];

/// pw.x 输入的三个 namelist 段。
#[derive(Debug, Clone, Default)]
pub struct PwParams {
    pub control: SectionMap,
    pub system: SectionMap,
    pub electrons: SectionMap,
}

/// 将 `Value` 当作 `SectionMap` 取出（非对象返回空表）。
pub fn as_section(v: &Value) -> SectionMap {
    serde_json::from_value(v.clone()).unwrap_or_default()
}

/// 由 `config.qe_phonon.scf` 构造 SCF 的 [`PwParams`]。
pub fn build_scf_pw_params(config: &Config) -> PwParams {
    let scf = &config.qe_phonon.scf;
    let mut control = SectionMap::new();
    let mut system = SectionMap::new();
    let mut electrons = SectionMap::new();
    for k in SCF_CONTROL_KEYS {
        if let Some(v) = scf.get(*k) {
            control.insert((*k).to_string(), v.clone());
        }
    }
    for k in SCF_SYSTEM_KEYS {
        if let Some(v) = scf.get(*k) {
            system.insert((*k).to_string(), v.clone());
        }
    }
    for k in SCF_ELECTRON_KEYS {
        if let Some(v) = scf.get(*k) {
            electrons.insert((*k).to_string(), v.clone());
        }
    }
    PwParams {
        control,
        system,
        electrons,
    }
}

/// 写出完整 pw.x 输入字符串（CONTROL/SYSTEM/ELECTRONS + 原子 + K_POINTS）。
pub fn write_pw_input(
    config: &Config,
    structure: &Structure,
    params: &PwParams,
    prefix: &str,
    kpoints: &str,
) -> Result<String> {
    write_pw_input_with_precision(config, structure, params, prefix, kpoints, 17)
}

pub fn write_pw_input_with_precision(config: &Config, structure: &Structure, params: &PwParams, prefix: &str, kpoints: &str, precision: usize) -> Result<String> {
    let species = write_atomic_species(config, &structure.elements)?;
    write_pw_input_with_species(config, structure, params, prefix, kpoints, precision, &species)
}

/// 使用调用方已绑定的赝势条目；与 qeflow 共用全部 PW namelist/结构写出逻辑。
/// species 必须是完整 ATOMIC_SPECIES 段，由受控适配器构造。
pub fn write_pw_input_with_species(
    config: &Config,
    structure: &Structure,
    params: &PwParams,
    prefix: &str,
    kpoints: &str,
    precision: usize,
    species: &str,
) -> Result<String> {
    let mut s = String::new();
    s.push_str("&CONTROL\n");
    s.push_str(&format!("  prefix = {}\n", to_qe_value(&json!(prefix))));
    if let Some(pd) = config.pp_dir.as_ref() {
        s.push_str(&format!("  pseudo_dir = {}\n", to_qe_value(&json!(pd))));
    }
    for (k, v) in &params.control {
        if k == "prefix" || k == "pseudo_dir" {
            continue;
        }
        s.push_str(&format!("  {k} = {}\n", to_qe_value(v)));
    }
    s.push_str("/\n\n");

    s.push_str("&SYSTEM\n");
    s.push_str(&format!("  nat = {}\n", structure.positions.len()));
    s.push_str(&format!("  ntyp = {}\n", structure.elements.len()));
    s.push_str(&write_system_lattice(config, structure)?);
    for (k, v) in &params.system {
        s.push_str(&format!("  {k} = {}\n", to_qe_value(v)));
    }
    s.push_str("/\n\n");

    s.push_str("&ELECTRONS\n");
    for (k, v) in &params.electrons {
        s.push_str(&format!("  {k} = {}\n", to_qe_value(v)));
    }
    s.push_str("/\n\n");

    s.push_str(species);
    s.push_str(&write_atomic_positions_with_precision(structure, precision));
    s.push_str(&write_cell_parameters_with_precision(config, structure, precision)?);
    s.push_str(kpoints);
    Ok(s)
}

/// 写出 phonon 工作流的 scf.in（复刻 `write_phonon_scf_input`）。
pub fn write_phonon_scf_input(
    workspace: &str,
    config: &Config,
    structure: &Structure,
    prefix: &str,
) -> Result<String> {
    let params = build_scf_pw_params(config);
    let kpoints = scf_kpoints_text(structure, &config.qe_phonon.scf);
    let content = write_pw_input(config, structure, &params, prefix, &kpoints)?;
    std::fs::create_dir_all(workspace)?;
    let path = format!("{workspace}/scf.in");
    std::fs::write(&path, content)?;
    if let Some(od) = config.qe_phonon.scf.get("outdir").and_then(|v| v.as_str()) {
        let od = od.trim_matches('\'').trim_matches('"');
        if !od.is_empty() {
            let _ = std::fs::create_dir_all(format!("{workspace}/{od}"));
        }
    }
    Ok(path)
}

/// 由 `scf` 配置段的 kpoints / kmesh 推导自动 k 点网格文本。
pub fn scf_kpoints_text(structure: &Structure, scf: &SectionMap) -> String {
    let grid = match scf.get("kpoints") {
        Some(Value::Array(a)) if a.len() == 3 => [
            a[0].as_i64().unwrap_or(1),
            a[1].as_i64().unwrap_or(1),
            a[2].as_i64().unwrap_or(1),
        ],
        Some(Value::Number(n)) => get_kpoints_from_spacing(
            structure.lattice,
            &KmeshSpec::Spacing(n.as_f64().unwrap_or(0.04)),
            0.04,
        ),
        _ => get_kpoints_from_spacing(structure.lattice, &KmeshSpec::Spacing(0.04), 0.04),
    };
    let shift: [i64; 3] = std::array::from_fn(|i| {
        scf.get("k_shift").and_then(Value::as_array).and_then(|v| v.get(i)).and_then(Value::as_i64).unwrap_or(0)
    });
    format!(
        "K_POINTS automatic\n  {grid0} {grid1} {grid2} {s0} {s1} {s2}\n",
        grid0 = grid[0],
        grid1 = grid[1],
        grid2 = grid[2], s0 = shift[0], s1 = shift[1], s2 = shift[2]
    )
}

/// 由嵌套 CONTROL/SYSTEM/ELECTRONS 的配置段构造 [`PwParams`]（复刻 `setup/calculation_setup.py`
/// 中 `qe_params = {section: cli_config.get(section)}` 的取段逻辑）。
pub fn build_nested_pw_params(section: &SectionMap) -> PwParams {
    PwParams {
        control: as_section(section.get("CONTROL").unwrap_or(&Value::Null)),
        system: as_section(section.get("SYSTEM").unwrap_or(&Value::Null)),
        electrons: as_section(section.get("ELECTRONS").unwrap_or(&Value::Null)),
    }
}

/// 由 `config.qe_phonon.scf` 构造 nscf 的 [`PwParams`]，覆盖 `calculation=nscf` 并补充 elph 所需字段。
pub fn build_nscf_pw_params(config: &Config) -> PwParams {
    let mut params = build_scf_pw_params(config);
    params
        .control
        .insert("calculation".to_string(), json!("nscf"));
    params.system.insert("nosym".to_string(), json!(true));
    params.system.insert("noinv".to_string(), json!(true));
    params
        .control
        .insert("verbosity".to_string(), json!("high"));
    if !params.system.contains_key("degauss") {
        params.system.insert("degauss".to_string(), json!(0.02));
    }
    if !params.system.contains_key("occupations") {
        params
            .system
            .insert("occupations".to_string(), json!("smearing"));
    }
    params
}

/// 写出 phonon工作流的 nscf.in（复刻 `write_phonon_scf_input` 的 nscf 变体）。
pub fn write_phonon_nscf_input(
    workspace: &str,
    config: &Config,
    structure: &Structure,
    prefix: &str,
) -> Result<String> {
    let params = build_nscf_pw_params(config);
    let kpoints = scf_kpoints_text(structure, &config.qe_phonon.scf);
    let content = write_pw_input(config, structure, &params, prefix, &kpoints)?;
    std::fs::create_dir_all(workspace)?;
    let path = format!("{workspace}/nscf.in");
    std::fs::write(&path, content)?;
    Ok(path)
}

/// 独立 qe_scf/qe_nscf 的输入，不能复用 qe_phonon.scf 配置。
pub fn write_calculation_input(
    workspace: &str,
    config: &Config,
    structure: &Structure,
    kind: &str,
) -> Result<String> {
    let nscf = kind == "nscf";
    let mut template = crate::electronic::eband_template();
    template["CONTROL"]["calculation"] = json!(kind);
    template["SYSTEM"] = if nscf {
        json!({"ecutwfc":50.0,"ecutrho":400.0,"occupations":"tetrahedra"})
    } else {
        json!({"ecutwfc":50.0,"ecutrho":400.0,"occupations":"smearing","smearing":"gaussian","degauss":0.01})
    };
    template["k_points"] = json!({"kmesh":if nscf {0.02} else {0.04},"kpoints":null,"wan":false});
    let section = crate::electronic::merge_section(
        &template,
        if nscf {
            &config.qe_nscf
        } else {
            &config.qe_scf
        },
    );
    let params = build_nested_pw_params(&section);
    let kp = &section["k_points"];
    let grid = if let Some(a) = kp["kpoints"].as_array() {
        anyhow::ensure!(a.len() == 3, "kpoints 必须包含三个整数");
        [
            a[0].as_i64().unwrap_or(0),
            a[1].as_i64().unwrap_or(0),
            a[2].as_i64().unwrap_or(0),
        ]
    } else {
        get_kpoints_from_spacing(
            structure.lattice,
            &KmeshSpec::Spacing(
                kp["kmesh"]
                    .as_f64()
                    .unwrap_or(if nscf { 0.02 } else { 0.04 }),
            ),
            0.04,
        )
    };
    anyhow::ensure!(grid.iter().all(|v| *v > 0), "kpoints 必须为正整数");
    let [nx, ny, nz] = grid;
    let points = if nscf {
        uniform_kpoints_text(grid, !kp["wan"].as_bool().unwrap_or(false))?
    } else {
        format!("K_POINTS automatic\n  {nx} {ny} {nz} 0 0 0\n")
    };
    let content = write_pw_input_with_precision(config, structure, &params, &structure.formula, &points, if nscf {8} else {17})?;
    std::fs::write(PathBuf::from(workspace).join(format!("{kind}.in")), content)?;
    let outdir = params
        .control
        .get("outdir")
        .and_then(Value::as_str)
        .unwrap_or("./tmp")
        .trim_matches('\'')
        .trim_matches('"')
        .to_string();
    std::fs::create_dir_all(PathBuf::from(workspace).join(&outdir))?;
    Ok(outdir)
}

/// Γ 中心完整均匀网格；不作对称约化，供 QE NSCF 与 EPW 共用。
pub fn uniform_kpoints_text(grid: [i64; 3], weights: bool) -> Result<String> {
    anyhow::ensure!(grid.iter().all(|n| *n > 0), "kpoints 必须为正整数");
    let total = grid.iter().try_fold(1i64, |n, v| n.checked_mul(*v))
        .ok_or_else(|| anyhow::anyhow!("kpoints 数量溢出"))?;
    let [nx, ny, nz] = grid;
    let mut points = format!("K_POINTS crystal\n{total}\n");
    for x in 0..nx { for y in 0..ny { for z in 0..nz {
        points.push_str(&format!(" {:21.17}{:21.17}{:21.17}",
            x as f64 / nx as f64, y as f64 / ny as f64, z as f64 / nz as f64));
        if weights { points.push_str(&format!(" {:21.17}", 1.0 / total as f64)); }
        points.push('\n');
    } } }
    Ok(points)
}
