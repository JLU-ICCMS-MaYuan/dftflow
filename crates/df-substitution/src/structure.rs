//! 结构替换与 case 目录生成（复刻 `substitutionflow/structure_substitution.py`）。

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::selection::{atomic_number_to_period, selection_to_symbols, symbol_to_atomic_number};
use crate::{load_config_file};

/// 文本级 POSCAR（保留 scale/lattice/selective/坐标/tail 原文）。
#[derive(Debug, Clone)]
pub struct PoscarData {
    pub comment: String,
    pub scale_line: String,
    pub lattice_lines: Vec<String>,
    pub elements: Vec<String>,
    pub counts: Vec<usize>,
    pub selective_line: String,
    pub coordinate_mode: String,
    pub coordinate_lines: Vec<String>,
    pub tail_lines: Vec<String>,
}

pub fn read_poscar(path: &str) -> Result<PoscarData> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    if lines.len() < 8 {
        bail!("POSCAR 内容不完整: {path}");
    }
    let elements: Vec<String> = lines[5].split_whitespace().map(|s| s.to_string()).collect();
    let counts: Vec<usize> = lines[6]
        .split_whitespace()
        .map(|s| s.parse())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("POSCAR 计数解析失败"))?;
    if elements.len() != counts.len() {
        bail!("POSCAR 元素与计数列不匹配。");
    }
    let mut next_index = 7usize;
    let mut selective_line = String::new();
    if lines[next_index].trim().to_lowercase().starts_with("selective") {
        selective_line = lines[next_index].clone();
        next_index += 1;
    }
    let coordinate_mode = lines[next_index].clone();
    next_index += 1;
    let total_atoms: usize = counts.iter().sum();
    if next_index + total_atoms > lines.len() {
        bail!("POSCAR 原子坐标数不足。");
    }
    let coordinate_lines: Vec<String> = lines[next_index..next_index + total_atoms].to_vec();
    let tail_lines: Vec<String> = lines[next_index + total_atoms..].to_vec();
    Ok(PoscarData {
        comment: lines[0].clone(),
        scale_line: lines[1].clone(),
        lattice_lines: lines[2..5].to_vec(),
        elements,
        counts,
        selective_line,
        coordinate_mode,
        coordinate_lines,
        tail_lines,
    })
}

pub fn build_formula(elements: &[String], counts: &[usize]) -> String {
    let mut common_divisor = 0usize;
    for count in counts {
        common_divisor = if common_divisor == 0 { *count } else { gcd(common_divisor, *count) };
    }
    let normalized: Vec<usize> = if common_divisor > 1 {
        counts.iter().map(|c| c / common_divisor).collect()
    } else {
        counts.to_vec()
    };
    elements
        .iter()
        .zip(&normalized)
        .map(|(e, c)| if *c == 1 { e.clone() } else { format!("{e}{c}") })
        .collect()
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

pub fn substitute_poscar(data: &PoscarData, replacements: &BTreeMap<String, String>) -> PoscarData {
    // 按元素分组坐标行，再按替换后的元素合并（保序）。
    let mut new_groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut start = 0usize;
    for (element, count) in data.elements.iter().zip(&data.counts) {
        let coords: Vec<String> = data.coordinate_lines[start..start + count].to_vec();
        start += count;
        let symbol = replacements.get(element).cloned().unwrap_or_else(|| element.clone());
        if let Some(entry) = new_groups.iter_mut().find(|(e, _)| *e == symbol) {
            entry.1.extend(coords);
        } else {
            new_groups.push((symbol, coords));
        }
    }
    let new_elements: Vec<String> = new_groups.iter().map(|(e, _)| e.clone()).collect();
    let new_counts: Vec<usize> = new_groups.iter().map(|(_, v)| v.len()).collect();
    let mut new_coordinate_lines = Vec::new();
    for (_, coords) in &new_groups {
        new_coordinate_lines.extend(coords.clone());
    }
    let formula = build_formula(&new_elements, &new_counts);
    PoscarData {
        comment: formula,
        scale_line: data.scale_line.clone(),
        lattice_lines: data.lattice_lines.clone(),
        elements: new_elements,
        counts: new_counts,
        selective_line: data.selective_line.clone(),
        coordinate_mode: data.coordinate_mode.clone(),
        coordinate_lines: new_coordinate_lines,
        tail_lines: data.tail_lines.clone(),
    }
}

pub fn write_poscar(path: &str, data: &PoscarData) -> Result<()> {
    let mut lines = vec![
        data.comment.clone(),
        data.scale_line.clone(),
    ];
    lines.extend(data.lattice_lines.clone());
    lines.push(format!("  {}", data.elements.join("  ")));
    lines.push(format!("  {}", data.counts.iter().map(|c| c.to_string()).collect::<Vec<_>>().join("  ")));
    if !data.selective_line.is_empty() {
        lines.push(data.selective_line.clone());
    }
    lines.push(data.coordinate_mode.clone());
    lines.extend(data.coordinate_lines.clone());
    lines.extend(data.tail_lines.clone());
    std::fs::write(path, lines.join("\n") + "\n").with_context(|| format!("write {path}"))?;
    Ok(())
}

fn dump_toml_value(value: &Value) -> String {
    match value {
        Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        Value::String(s) => json!(s).to_string(),
        Value::Null => json!("").to_string(),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(dump_toml_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Number(n) => n.to_string(),
        Value::Object(_) => value.to_string(),
    }
}

/// `dump_toml`：标量在前、嵌套表在后（复刻 `structure_substitution.dump_toml`）。
pub fn dump_toml(data: &Map<String, Value>, prefix: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut dict_keys: Vec<(&String, &Value)> = Vec::new();
    for (key, value) in data {
        if value.is_object() {
            dict_keys.push((key, value));
        } else {
            lines.push(format!("{key} = {}", dump_toml_value(value)));
        }
    }
    for (key, value) in dict_keys {
        let section = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("[{section}]"));
        if let Value::Object(obj) = value {
            let nested = dump_toml(obj, &section);
            if !nested.is_empty() {
                lines.push(nested);
            }
        }
    }
    lines.join("\n")
}

pub fn load_substitutionflow_config(config_file: &str) -> Result<Value> {
    let config = load_config_file(config_file)?;
    if config.get("substitutionflow").is_none() {
        bail!("配置缺少 [substitutionflow] 块。");
    }
    Ok(config)
}

pub fn resolve_input_structure_path(config_file: &str) -> Result<String> {
    let config = load_substitutionflow_config(config_file)?;
    let ht = config.get("substitutionflow").cloned().unwrap_or(Value::Null);
    let structure_filename = ht
        .get("structure_filename")
        .and_then(|v| v.as_str())
        .unwrap_or("structure.vasp");
    let config_dir = Path::new(config_file)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let structure_path = config_dir.join(structure_filename);
    if !structure_path.exists() {
        bail!("找不到结构文件: {}", structure_path.display());
    }
    Ok(structure_path.to_string_lossy().into_owned())
}

pub fn validate_placeholder_elements(elements: &[String]) -> Result<()> {
    let missing: Vec<&str> = ["XA", "XB"]
        .iter()
        .filter(|r| !elements.iter().any(|e| e == *r))
        .copied()
        .collect();
    if !missing.is_empty() {
        bail!("POSCAR 必须包含占位元素 XA/XB，缺少: {}", missing.join(", "));
    }
    Ok(())
}

pub fn create_case_input_config(root_config: &Value) -> Value {
    let mut config = root_config.clone();
    if let Value::Object(obj) = &mut config {
        obj.insert("chgcar_path".into(), Value::String("./vasp_scf/CHGCAR".into()));
        obj.insert("wavecar_path".into(), Value::String("./vasp_scf/WAVECAR".into()));
    }
    config
}

pub fn write_case_input_config(target_path: &str, root_config: &Value) -> Result<Value> {
    let case_config = create_case_input_config(root_config);
    if let Some(parent) = Path::new(target_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = if let Value::Object(obj) = &case_config {
        dump_toml(obj, "")
    } else {
        String::new()
    };
    std::fs::write(target_path, format!("{}\n", text.trim()))
        .with_context(|| format!("write {target_path}"))?;
    Ok(case_config)
}

pub fn build_case_relative_path(xa_symbol: &str, xb_symbol: &str, formula: &str) -> Result<String> {
    let xa_z = symbol_to_atomic_number(xa_symbol)?;
    let xb_z = symbol_to_atomic_number(xb_symbol)?;
    let xb_period = atomic_number_to_period(xb_z)?;
    Ok(format!("{xa_z:02}.XA_{xa_symbol}/period_{xb_period}/{xb_z:02}.{formula}"))
}

pub struct GenerateResult {
    pub root_config: Value,
    /// case 相对路径 → (formula, xa, xb, pressures)
    pub cases: Vec<CaseInfo>,
    pub pressures: Vec<Value>,
    pub structure_filename: String,
}

#[derive(Debug, Clone)]
pub struct CaseInfo {
    pub case_rel: String,
    pub formula: String,
    pub xa_symbol: String,
    pub xb_symbol: String,
    pub pressures: Vec<Value>,
}

/// `generate_cases`：XA×XB 枚举替换、写结构/配置/manifest。
pub fn generate_cases(input_path: &str, config_file: &str, output_dir: &str) -> Result<GenerateResult> {
    let root_config = load_substitutionflow_config(config_file)?;
    let ht = root_config.get("substitutionflow").cloned().unwrap_or(json!({}));
    let xa_symbols = selection_to_symbols(
        ht.get("XA").and_then(|v| v.as_str()).context("[substitutionflow].XA 缺失")?,
    )?;
    let xb_symbols = selection_to_symbols(
        ht.get("XB").and_then(|v| v.as_str()).context("[substitutionflow].XB 缺失")?,
    )?;
    let pressures: Vec<Value> = match ht.get("pressures") {
        Some(Value::Array(items)) => items.clone(),
        _ => vec![json!(20), json!(50), json!(100)],
    };
    let structure_filename = ht
        .get("structure_filename")
        .and_then(|v| v.as_str())
        .unwrap_or("structure.vasp")
        .to_string();

    let poscar = read_poscar(input_path)?;
    validate_placeholder_elements(&poscar.elements)?;
    std::fs::create_dir_all(output_dir)?;
    let input_abs = std::fs::canonicalize(input_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| input_path.to_string());

    let mut cases: Vec<CaseInfo> = Vec::new();
    // period_rel -> manifest
    let mut period_manifests: Vec<(String, Value)> = Vec::new();
    for xa_symbol in &xa_symbols {
        for xb_symbol in &xb_symbols {
            let mut replacements = BTreeMap::new();
            replacements.insert("XA".to_string(), xa_symbol.clone());
            replacements.insert("XB".to_string(), xb_symbol.clone());
            let replaced = substitute_poscar(&poscar, &replacements);
            let formula = build_formula(&replaced.elements, &replaced.counts);
            let case_rel = build_case_relative_path(xa_symbol, xb_symbol, &formula)?;
            let case_dir = format!("{output_dir}/{case_rel}");
            std::fs::create_dir_all(&case_dir)?;

            let structure_path = format!("{case_dir}/{structure_filename}");
            write_poscar(&structure_path, &replaced)?;

            write_case_input_config(&format!("{case_dir}/input.toml"), &root_config)?;

            let period_rel = Path::new(&case_rel)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let case_dirname = Path::new(&case_rel)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let manifest_entry = json!({
                "formula": formula,
                "xb_symbol": xb_symbol,
                "case_dir": case_dirname,
            });
            if let Some((_, manifest)) = period_manifests.iter_mut().find(|(r, _)| *r == period_rel) {
                manifest["cases"].as_array_mut().unwrap().push(manifest_entry);
            } else {
                period_manifests.push((
                    period_rel.clone(),
                    json!({
                        "input_structure": input_abs,
                        "xa_symbol": xa_symbol,
                        "pressures": pressures,
                        "cases": [manifest_entry],
                    }),
                ));
            }
            cases.push(CaseInfo {
                case_rel,
                formula,
                xa_symbol: xa_symbol.clone(),
                xb_symbol: xb_symbol.clone(),
                pressures: pressures.clone(),
            });
        }
    }

    for (period_rel, manifest) in &period_manifests {
        let manifest_path = format!("{output_dir}/{period_rel}/workflow_manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string_pretty(manifest)?)
            .with_context(|| format!("write {manifest_path}"))?;
    }

    Ok(GenerateResult {
        root_config,
        cases,
        pressures,
        structure_filename,
    })
}
