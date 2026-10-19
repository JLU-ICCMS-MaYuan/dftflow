//! 赝势文件匹配与 ATOMIC_SPECIES 解析（复刻 `qeflow/common.py`）。

use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use regex::Regex;
use serde_json::Value;

use qf_config::Config;

/// 元素标准原子质量表（来源 `qeflow/common.py: ATOMIC_MASSES`）。
pub const ATOMIC_MASSES: &[(&str, f64)] = &[
    ("H", 1.008), ("He", 4.0026), ("Li", 6.94), ("Be", 9.0122), ("B", 10.81),
    ("C", 12.011), ("N", 14.007), ("O", 15.999), ("F", 18.998), ("Ne", 20.180),
    ("Na", 22.990), ("Mg", 24.305), ("Al", 26.982), ("Si", 28.085), ("P", 30.974),
    ("S", 32.06), ("Cl", 35.45), ("Ar", 39.948), ("K", 39.098), ("Ca", 40.078),
    ("Sc", 44.956), ("Ti", 47.867), ("V", 50.942), ("Cr", 51.996), ("Mn", 54.938),
    ("Fe", 55.845), ("Co", 58.933), ("Ni", 58.693), ("Cu", 63.546), ("Zn", 65.38),
    ("Ga", 69.723), ("Ge", 72.63), ("As", 74.922), ("Se", 78.971), ("Br", 79.904),
    ("Kr", 83.798), ("Rb", 85.468), ("Sr", 87.62), ("Y", 88.906), ("Zr", 91.224),
    ("Nb", 92.906), ("Mo", 95.95), ("Tc", 98.0), ("Ru", 101.07), ("Rh", 102.91),
    ("Pd", 106.42), ("Ag", 107.87), ("Cd", 112.41), ("In", 114.82), ("Sn", 118.71),
    ("Sb", 121.76), ("Te", 127.60), ("I", 126.90), ("Xe", 131.29), ("Cs", 132.91),
    ("Ba", 137.33), ("La", 138.91), ("Ce", 140.12), ("Pr", 140.91), ("Nd", 144.24),
    ("Pm", 145.0), ("Sm", 150.36), ("Eu", 151.96), ("Gd", 157.25), ("Tb", 158.93),
    ("Dy", 162.50), ("Ho", 164.93), ("Er", 167.26), ("Tm", 168.93), ("Yb", 173.05),
    ("Lu", 174.97), ("Hf", 178.49), ("Ta", 180.95), ("W", 183.84), ("Re", 186.21),
    ("Os", 190.23), ("Ir", 192.22), ("Pt", 195.08), ("Au", 196.97), ("Hg", 200.59),
    ("Tl", 204.38), ("Pb", 207.2), ("Bi", 208.98), ("Th", 232.04), ("Pa", 231.04),
    ("U", 238.03),
];

/// 返回元素的原子质量；显式质量优先。
pub fn get_atomic_mass(element: &str, explicit_mass: Option<f64>) -> f64 {
    explicit_mass.unwrap_or_else(|| {
        ATOMIC_MASSES
            .iter()
            .find(|(e, _)| *e == element)
            .map(|(_, m)| *m)
            .unwrap_or(1.0)
    })
}

/// 由赝势文件名提取元素符号 token（文件名开头连续字母串）。
pub fn extract_pseudo_element_token(filename: &str) -> String {
    let basename = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);
    Regex::new(r"^([A-Za-z]+)")
        .ok()
        .and_then(|re| re.find(basename))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

/// 在 `pp_dir` 中查找元素 `element` 对应的唯一赝势文件。
pub fn find_pseudo_file(pp_dir: &str, element: &str) -> Result<String> {
    let dir = Path::new(pp_dir);
    if !dir.is_dir() {
        bail!("找不到赝势目录: {pp_dir}");
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let token = extract_pseudo_element_token(&name);
        if !token.is_empty() && token.eq_ignore_ascii_case(element) {
            candidates.push(name);
        }
    }
    match candidates.len() {
        0 => bail!("在 {pp_dir} 中找不到元素 {element} 对应的赝势文件"),
        1 => Ok(candidates.into_iter().next().unwrap()),
        _ => {
            candidates.sort();
            bail!(
                "元素 {element} 在 {pp_dir} 中匹配到多个赝势文件: {}",
                candidates.join(", ")
            )
        }
    }
}

/// 解析 ATOMIC_SPECIES 三元组列表：(元素, 原子质量, 赝势文件名)。
pub fn resolve_atomic_species_entries(
    config: &Config,
    elements: &[String],
) -> Result<Vec<(String, f64, String)>> {
    let pp_dir = config
        .pp_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少 pp_dir 配置，无法自动匹配赝势文件"))?;
    let mut resolved: std::collections::BTreeMap<String, String> = Default::default();
    for element in elements {
        if !resolved.contains_key(element) {
            let pseudo = find_pseudo_file(pp_dir, element)?;
            resolved.insert(element.clone(), pseudo);
        }
    }
    Ok(elements
        .iter()
        .map(|element| {
            (
                element.clone(),
                get_atomic_mass(element, config.atomic_masses.get(element).copied()),
                resolved[element].clone(),
            )
        })
        .collect())
}

/// 写出 ATOMIC_SPECIES 段。
pub fn write_atomic_species(config: &Config, elements: &[String]) -> Result<String> {
    let entries = resolve_atomic_species_entries(config, elements)?;
    let mut s = String::from("ATOMIC_SPECIES\n");
    for (el, mass, pseudo) in entries {
        s.push_str(&format!("  {el:3} {mass:8.4} {pseudo}\n"));
    }
    s.push('\n');
    Ok(s)
}

/// 供测试/调试：将 QE 值序列化（避免未使用警告）。
#[allow(dead_code)]
fn _value_dbg(_v: &Value) {}
