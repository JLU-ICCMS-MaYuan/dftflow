//! 输出工具（复刻 `scflow/utils.py`：stround/floatround/format_e/parse_formula 打印、
//! save_dict / save_result / save_structure / update_summary）。

use anyhow::Result;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::epc::DynElph;
use crate::structure_out::StructureOut;
use crate::system::OrderedDict;

/// Python `round(a, d)`（half-even，作用于二进制值的精确十进制）。
pub fn py_round(x: f64, d: i32) -> f64 {
    let formatted = format!("{:.*}", d as usize, x);
    formatted.parse().unwrap_or(x)
}

/// Python `round(a, d)` 后按 `%.df` 输出（`stround`）。
pub fn stround(x: f64, d: usize) -> String {
    format!("{:.*}", d, py_round(x, d as i32))
}

/// Python `int(x)`：向零截断。
pub fn py_int(x: f64) -> i64 {
    x.trunc() as i64
}

/// `format_e`：Python `'%E'`（6 位小数）后去尾零（如 `1.234500E-18` → `1.2345E-18`）。
pub fn format_e(n: f64) -> String {
    let s = format!("{:.6e}", n); // "1.2345e-18"
    let (mantissa, exponent) = s.split_once('e').unwrap_or((s.as_str(), "0"));
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    let exp: i32 = exponent.parse().unwrap_or(0);
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mantissa}E{sign}{:02}", exp.abs())
}

fn format_smoothing(v: f64) -> String {
    // main.py：整数则取整，再 '.'→','
    format_smoothing_raw(v)
}

/// Python `str(v).replace('.', ',')`（整数先取整）。
pub fn format_smoothing_raw(v: f64) -> String {
    let v = if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") };
    v.replace('.', ",")
}

/// `save_dict`：direct/a2f 有序字典 → CSV（pandas `to_csv(float_format='%.8f')`）。
pub fn save_dict(dict: &OrderedDict, path: &str, prefix: &str, b: f64) -> Result<String> {
    let name = if b > 0.0 {
        format!("{path}/{prefix}_b{}.csv", format_smoothing(b))
    } else {
        format!("{path}/{prefix}.csv")
    };
    let n_rows = dict.first().map(|(_, v)| v.len()).unwrap_or(0);
    let mut text = String::new();
    // pandas to_csv 默认 QUOTE_MINIMAL：含逗号的表头加引号
    let headers: Vec<String> = dict
        .iter()
        .map(|(k, _)| {
            if k.contains(',') || k.contains('"') || k.contains('\n') {
                format!("\"{}\"", k.replace('"', "\"\""))
            } else {
                k.clone()
            }
        })
        .collect();
    text.push_str(&headers.join(","));
    text.push('\n');
    for i in 0..n_rows {
        let row: Vec<String> = dict
            .iter()
            .map(|(_, v)| format!("{:.8}", v[i]))
            .collect();
        text.push_str(&row.join(","));
        text.push('\n');
    }
    std::fs::write(&name, text)?;
    Ok(name)
}

/// `save_result`：`json.dump(..., indent=4)`。
pub fn save_result(result: &Value, path: &str) -> Result<()> {
    let out = format!("{path}/result.json");
    std::fs::write(&out, serde_json::to_string_pretty(result)?)?;
    Ok(())
}

/// `save_structure`：spglib 对称化 CIF + POSCAR（VASP）。
pub fn save_structure(
    structure: &StructureOut,
    tolerance: f64,
    output_dir: &str,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    let (formula, fu) = structure.formula(true);
    let mut sym = "Unknown".to_string();
    match qf_crystal::analyze_symmetry(&structure.to_structure(), 0.2) {
        Ok(info) => {
            sym = info.symbol.clone();
            let cif = qf_crystal::write_cif_symmetrized(&info, &formula);
            let cif_path = format!("{output_dir}/{formula}_{}.cif", info.number);
            std::fs::write(&cif_path, cif)?;
        }
        Err(_) => println!("Unable to symmetrize the structure"),
    }
    let vasp_name = if fu == 1 {
        format!("{output_dir}/{formula}.vasp")
    } else {
        format!("{output_dir}/{fu}x{formula}.vasp")
    };
    println!("\nThe structure under consideration is {sym}-{formula}.");
    structure.write_vasp(Path::new(&vasp_name))?;
    Ok(())
}

fn get_last(dict: &OrderedDict, key: &str) -> f64 {
    crate::system::get(dict, key).and_then(|v| v.last()).copied().unwrap_or(0.0)
}

/// `print_direct`。
pub fn print_direct(direct: &OrderedDict, smoothing: f64) {
    let lambda = stround(get_last(direct, "lambda (gamma)"), 3);
    let wlog = py_round(get_last(direct, "wlog (gamma), K"), 0);
    let w2 = py_round(get_last(direct, "w2 (gamma), K"), 0);
    println!(
        "With the smoothing of {smoothing} THz, direct lambda = {lambda}, direct wlog = {} K, direct w2 = {} K.",
        format_int_for_print(wlog),
        format_int_for_print(w2)
    );
}

/// `print_a2f`。
pub fn print_a2f(a2f: &OrderedDict, resolution: usize, sigma: f64) {
    let lambda = stround(get_last(a2f, "lambda (gamma)"), 3);
    let wlog = py_round(get_last(a2f, "wlog (gamma), K"), 0);
    let w2 = py_round(get_last(a2f, "w2 (gamma), K"), 0);
    println!(
        "After calculating alpha2f on resolution r = {resolution} and applying gaussian filter with sigma = {sigma}, interp lambda = {lambda}, interp wlog = {} K, interp w2 = {} K.",
        format_int_for_print(wlog),
        format_int_for_print(w2)
    );
}

/// `print_tc`。
pub fn print_tc(direct: &OrderedDict, a2f: &OrderedDict, mu: f64) {
    let tc_mcm_direct = stround(get_last(direct, "Tc_McM (gamma), K"), 3);
    let tc_ad_direct = stround(get_last(direct, "Tc_AD (gamma), K"), 3);
    let tc_mcm_a2f = stround(get_last(a2f, "Tc_McM (gamma), K"), 3);
    let tc_ad_a2f = stround(get_last(a2f, "Tc_AD (gamma), K"), 3);
    println!(
        "Mu = {mu}. Calculated McMillan Tc = {tc_mcm_direct} ({tc_mcm_a2f}) K, calculated Allen-Dynes Tc = {tc_ad_direct} ({tc_ad_a2f}) K"
    );
}

fn format_int_for_print(v: f64) -> String {
    // Python round(x) 输出为 int（如 352.0 → 352）
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// 零频模式记录（result.json 的 `zero_frequency_handling.modes`）。
pub fn zero_modes(dyn_elphs: &[DynElph], paths: &[std::path::PathBuf]) -> Vec<Value> {
    let mut modes = Vec::new();
    for (path, dyn_elph) in paths.iter().zip(dyn_elphs) {
        for (mode, _) in dyn_elph
            .sqr_freqs
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == 0.0)
        {
            modes.push(json!({
                "file": path.to_string_lossy(),
                "q_point": dyn_elph.q_point.to_vec(),
                "mode_1based": mode + 1,
                "lambda": dyn_elph.lambdas[0][mode],
                "gamma_GHz": dyn_elph.gammas[0][mode],
            }));
        }
    }
    modes
}

/// `Superconducting.get_all`（复刻 `scflow/sc_e.py: get_all`）构建 result.json。
#[allow(clippy::too_many_arguments)]
pub fn build_result(
    direct: &OrderedDict,
    a2f: &OrderedDict,
    tc_e: f64,
    nef: f64,
    structure: Option<&StructureOut>,
    smoothing: f64,
    resolution: usize,
    mu: f64,
    broadening: f64,
    zero_modes: Vec<Value>,
) -> Value {
    let get_last = |dict: &OrderedDict, key: &str| -> f64 {
        crate::system::get(dict, key).and_then(|v| v.last()).copied().unwrap_or(0.0)
    };
    let mut result = Map::new();
    let (formula, fu) = match structure {
        Some(s) => {
            let (f, fu) = s.formula(true);
            result.insert("volume, A^3".into(), json!(py_round(s.volume(), 3)));
            result.insert("density, g/cm^3".into(), json!(py_round(s.density().unwrap_or(f64::NAN), 3)));
            result.insert("volume of chemical formula unit, A^3".into(), json!(py_round(s.volume() / fu as f64, 3)));
            (f, fu)
        }
        None => ("Unknown".to_string(), 1usize),
    };
    result.insert("formula".into(), json!(formula));
    result.insert("# of chemical formula units (f.u.)".into(), json!(fu));
    result.insert("Smoothing, THz".into(), json!(smoothing));
    result.insert("Resolution, pts".into(), json!(resolution));
    result.insert("mu".into(), json!(mu));
    result.insert(
        "lambda".into(),
        json!({
            "direct": {
                "lambda": py_round(get_last(direct, "lambda (lambda)"), 3),
                "gamma": py_round(get_last(direct, "lambda (gamma)"), 3),
            },
            "a2f": {
                "lambda": py_round(get_last(a2f, "lambda (lambda)"), 3),
                "gamma": py_round(get_last(a2f, "lambda (gamma)"), 3),
            },
        }),
    );
    let wlog_entry = |dict: &OrderedDict| {
        json!({
            "lambda": {
                "K": py_int(get_last(dict, "wlog (lambda), K")),
                "THz": py_int(get_last(dict, "wlog (lambda), K") * K_T_THZ),
            },
            "gamma": {
                "K": py_int(get_last(dict, "wlog (gamma), K")),
                "THz": py_int(get_last(dict, "wlog (gamma), K") * K_T_THZ),
            },
        })
    };
    result.insert(
        "wlog, K".into(),
        json!({"direct": wlog_entry(direct), "a2f": wlog_entry(a2f)}),
    );
    let w2_entry = |dict: &OrderedDict| {
        json!({
            "lambda": {
                "K": py_int(get_last(dict, "w2 (lambda), K")),
                "THz": py_int(get_last(dict, "w2 (lambda), K") * K_T_THZ),
            },
            "gamma": {
                "K": py_int(get_last(dict, "w2 (gamma), K")),
                "THz": py_int(get_last(dict, "w2 (gamma), K") * K_T_THZ),
            },
        })
    };
    result.insert(
        "w2, K".into(),
        json!({"direct": w2_entry(direct), "a2f": w2_entry(a2f)}),
    );
    result.insert(
        "Tc, K".into(),
        json!({
            "McMillan": {
                "direct": {
                    "lambda": py_round(get_last(direct, "Tc_McM (lambda), K"), 3),
                    "gamma": py_round(get_last(direct, "Tc_McM (gamma), K"), 3),
                },
                "a2f": {
                    "lambda": py_round(get_last(a2f, "Tc_McM (lambda), K"), 3),
                    "gamma": py_round(get_last(a2f, "Tc_McM (gamma), K"), 3),
                },
            },
            "Allen-Dynes": {
                "direct": {
                    "lambda": py_round(get_last(direct, "Tc_AD (lambda), K"), 3),
                    "gamma": py_round(get_last(direct, "Tc_AD (gamma), K"), 3),
                },
                "a2f": {
                    "lambda": py_round(get_last(a2f, "Tc_AD (lambda), K"), 3),
                    "gamma": py_round(get_last(a2f, "Tc_AD (gamma), K"), 3),
                },
            },
            "Eliashberg": py_round(tc_e, 3),
        }),
    );
    // Sommerfeld gamma / DeltaC/Tc / Delta / Hc2 / beta / DOS
    let lambda_direct_gamma = get_last(direct, "lambda (gamma)");
    let wlog_direct = get_last(direct, "wlog (gamma), K");
    let gamma = 2.0 / 3.0 * (std::f64::consts::PI * K_B).powi(2) * nef * (1.0 + lambda_direct_gamma)
        / K_RY_J;
    match structure {
        None => {
            result.insert(
                "Sommerfeld gamma".into(),
                json!({"mJ/mol/K^2": py_round(1000.0 * gamma * N_A / fu as f64, 3)}),
            );
            let dctc_v = dctc(tc_e, wlog_direct, gamma);
            result.insert("DeltaC/Tc".into(), json!({"mJ/mol/K^2": py_round(dctc_v, 0)}));
        }
        Some(s) => {
            let volume = s.volume();
            let density = s.density().unwrap_or(f64::NAN);
            result.insert(
                "Sommerfeld gamma".into(),
                json!({
                    "mJ/mol/K^2": py_round(1000.0 * gamma * N_A / fu as f64, 3),
                    "mJ/g/K^2": py_round(1000.0 * gamma / volume / density / K_A_CM.powi(3), 5),
                    "mJ/cm^3/K^2": py_round(1000.0 * gamma / volume / K_A_CM.powi(3), 4),
                }),
            );
            let dctc_v = dctc(tc_e, wlog_direct, gamma);
            result.insert(
                "DeltaC/Tc".into(),
                json!({
                    "mJ/mol/K^2": py_round(1000.0 * dctc_v * N_A / fu as f64, 3),
                    "mJ/g/K^2": py_round(1000.0 * dctc_v / volume / density / K_A_CM.powi(3), 3),
                    "mJ/cm^3/K^2": py_round(1000.0 * dctc_v / volume / K_A_CM.powi(3), 4),
                }),
            );
        }
    }
    let delta_v = delta(tc_e, wlog_direct);
    result.insert(
        "Delta".into(),
        json!({
            "meV": py_round(delta_v * K_J_MEV, 3),
            "2Delta/kBTc": py_round(2.0 * delta_v / (K_B * tc_e), 3),
            "J": format_e(delta_v),
        }),
    );
    match structure {
        None => {
            result.insert("Hc2, T".into(), Value::Null);
        }
        Some(s) => {
            let volume = s.volume();
            let hc2 = hc(tc_e, wlog_direct, 2.0 * 1000.0 * gamma / volume / K_A_CM.powi(3));
            // Python: round(hc, 3), 带尾逗号成为单元素元组 → JSON 数组
            result.insert("Hc2, T".into(), json!([py_round(hc2, 3)]));
        }
    }
    let beta_v = beta(lambda_direct_gamma, mu);
    result.insert("beta (McMillan isotope coefficient)".into(), json!(py_round(beta_v, 5)));
    let mut dos = Map::new();
    dos.insert(
        "per Unit cell".into(),
        json!({
            "states/spin/Ry": py_round(nef, 4),
            "states/spin/eV": py_round(nef / K_RY_EV, 4),
        }),
    );
    dos.insert(
        format!("per chemical formula unit ({formula})"),
        json!({
            "states/spin/Ry": py_round(nef / fu as f64, 4),
            "states/spin/eV": py_round(nef / K_RY_EV / fu as f64, 4),
        }),
    );
    match structure {
        None => {
            dos.insert(
                format!("per mole of {formula}"),
                json!({
                    "states/spin/Ry": format_e(nef / fu as f64 * N_A),
                    "states/spin/eV": format_e(nef / K_RY_EV / fu as f64 * N_A),
                }),
            );
        }
        Some(s) => {
            let volume = s.volume();
            let density = s.density().unwrap_or(f64::NAN);
            dos.insert(
                "per A^3".into(),
                json!({
                    "states/spin/Ry": py_round(nef / volume, 4),
                    "states/spin/eV": py_round(nef / K_RY_EV / volume, 4),
                }),
            );
            dos.insert(
                format!("per mole of {formula}"),
                json!({
                    "states/spin/Ry": format_e(nef / fu as f64 * N_A),
                    "states/spin/eV": format_e(nef / K_RY_EV / fu as f64 * N_A),
                }),
            );
            dos.insert(
                "per g".into(),
                json!({
                    "states/spin/Ry": format_e(nef / volume / density / K_A_CM.powi(3)),
                    "states/spin/eV": format_e(nef / K_RY_EV / volume / density / K_A_CM.powi(3)),
                }),
            );
        }
    }
    result.insert("DOS".into(), Value::Object(dos));
    result.insert(
        "eliashberg_solver".into(),
        json!({
            "status": if tc_e <= 0.001 { "resolution_limit" } else { "resolved" },
            "resolution_K": 0.001,
            "minimum_reported_Tc_K": 0.001,
            "matrix_size": 25,
        }),
    );
    result.insert(
        "zero_frequency_handling".into(),
        json!({
            "policy": "exclude_exact_zero_modes_only_if_lambda_is_zero",
            "modes": zero_modes,
        }),
    );
    if broadening > 0.0 {
        result.insert("Broadening, Ry".into(), json!(broadening));
    }
    Value::Object(result)
}

use crate::constants::{K_A_CM, K_B, K_J_MEV, K_RY_EV, K_RY_J, K_T_THZ, N_A};

/// `tc.py: hc`。
pub fn hc(tc: f64, wlog: f64, gamma: f64) -> f64 {
    0.01 * tc
        * (gamma
            / (0.168 * (1.0 - 12.2 * (tc / wlog).powi(2) * (wlog / (3.0 * tc)).ln())))
        .sqrt()
}

/// `tc.py: dctc`。
pub fn dctc(tc: f64, wlog: f64, gamma: f64) -> f64 {
    gamma * 1.43 * (1.0 + (wlog / (3.0 * tc)).ln() * 53.0 * (tc / wlog).powi(2))
}

/// `tc.py: delta`。
pub fn delta(tc: f64, wlog: f64) -> f64 {
    0.5 * K_B * tc * 3.53
        * (1.0 + 12.5 * (tc / wlog).powi(2) * (wlog / (2.0 * tc)).ln())
}

/// `tc.py: beta`。
pub fn beta(_lambda: f64, mu: f64) -> f64 {
    0.5 * (1.0
        - mu * mu * (1.04 * (1.0 + _lambda)) * (1.0 + 0.62 * _lambda)
            / (_lambda - mu * (1.0 + 0.62 * _lambda)).powi(2))
}

/// `update_summary` + `summary_{formula}_s{s}_mu{mu}.csv`。
pub fn update_summary(
    summary: &mut std::collections::BTreeMap<String, Vec<f64>>,
    result: &Value,
    broadening: f64,
    formula: &str,
    smoothing: f64,
    mu: f64,
    path: &str,
) -> Result<()> {
    let keys = [
        "Broadening, Ry",
        "lambda (direct)",
        "lambda (interp)",
        "wlog, K (direct)",
        "wlog, K (interp)",
        "w2, K (direct)",
        "w2, K (interp)",
        "Tc McM, K (direct)",
        "Tc McM, K (interp)",
        "Tc AD, K (direct)",
        "Tc AD, K (interp)",
        "Tc E, K",
        "Sommerfeld gamma, mJ/cm^3/K^2",
        "DOS, states/spin/Ry/A^3",
        "DeltaC/Tc, mJ/cm^3/K^2",
        "Delta, meV",
        "Hc2, T",
        "beta",
    ];
    let get = |r: &Value, path: &[&str]| -> f64 {
        let mut cur = r;
        for p in path {
            match cur.get(*p) {
                Some(v) => cur = v,
                None => return f64::NAN,
            }
        }
        cur.as_f64().unwrap_or(f64::NAN)
    };
    let mut append = |m: &mut std::collections::BTreeMap<String, Vec<f64>>, key: &str, value: f64| {
        m.entry(key.to_string()).or_default().push(value);
    };
    let arr_first = |v: Option<&Value>| -> f64 {
        v.and_then(|x| x.as_array())
            .and_then(|a| a.first())
            .and_then(|x| x.as_f64())
            .unwrap_or(f64::NAN)
    };
    append(summary, "Broadening, Ry", broadening);
    append(summary, "lambda (direct)", get(result, &["lambda", "direct", "gamma"]));
    append(summary, "lambda (interp)", get(result, &["lambda", "a2f", "gamma"]));
    append(summary, "wlog, K (direct)", get(result, &["wlog, K", "direct", "gamma", "K"]));
    append(summary, "wlog, K (interp)", get(result, &["wlog, K", "a2f", "gamma", "K"]));
    append(summary, "w2, K (direct)", get(result, &["w2, K", "direct", "gamma", "K"]));
    append(summary, "w2, K (interp)", get(result, &["w2, K", "a2f", "gamma", "K"]));
    append(summary, "Tc McM, K (direct)", get(result, &["Tc, K", "McMillan", "direct", "gamma"]));
    append(summary, "Tc McM, K (interp)", get(result, &["Tc, K", "McMillan", "a2f", "gamma"]));
    append(summary, "Tc AD, K (direct)", get(result, &["Tc, K", "Allen-Dynes", "direct", "gamma"]));
    append(summary, "Tc AD, K (interp)", get(result, &["Tc, K", "Allen-Dynes", "a2f", "gamma"]));
    append(summary, "Tc E, K", get(result, &["Tc, K", "Eliashberg"]));
    append(summary, "Sommerfeld gamma, mJ/cm^3/K^2", get(result, &["Sommerfeld gamma", "mJ/cm^3/K^2"]));
    append(summary, "DOS, states/spin/Ry/A^3", get(result, &["DOS", "per A^3", "states/spin/Ry"]));
    append(summary, "DeltaC/Tc, mJ/cm^3/K^2", get(result, &["DeltaC/Tc", "mJ/cm^3/K^2"]));
    append(summary, "Delta, meV", get(result, &["Delta", "meV"]));
    append(summary, "Hc2, T", arr_first(result.get("Hc2, T")));
    append(summary, "beta", get(result, &["beta (McMillan isotope coefficient)"]));
    // 写 CSV（pandas to_csv：NaN 写空字段；数值走最短表示；含逗号的表头加引号）
    let n_rows = summary.get(keys[0]).map(|v| v.len()).unwrap_or(0);
    let header_fields: Vec<String> = keys
        .iter()
        .map(|h| {
            if h.contains(',') {
                format!("\"{h}\"")
            } else {
                h.to_string()
            }
        })
        .collect();
    let mut text = header_fields.join(",");
    text.push('\n');
    for i in 0..n_rows {
        let row: Vec<String> = keys
            .iter()
            .map(|h| {
                match summary.get(*h).and_then(|v| v.get(i)).copied() {
                    Some(f) if f.is_nan() => String::new(),
                    Some(f) => format!("{f}"),
                    None => String::new(),
                }
            })
            .collect();
        text.push_str(&row.join(","));
        text.push('\n');
    }
    let s = format_smoothing(smoothing);
    let mu_str = format_smoothing(mu);
    let csv_path = format!("{path}/summary_{formula}_s{s}_mu{mu_str}.csv");
    std::fs::write(&csv_path, text)?;
    Ok(())
}
