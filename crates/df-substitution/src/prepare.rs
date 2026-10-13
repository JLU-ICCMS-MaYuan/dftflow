//! 本地赝势库准备（复刻 `substitutionflow/prepare_potcar.py` 与
//! `substitutionflow/prepare_qe_pseudo.py`）。交互式 stdin 选号与 Python 一致。

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::selection::{symbol_to_atomic_number, ELEMENT_SYMBOLS};
use crate::structure::{
    load_substitutionflow_config, read_poscar, resolve_input_structure_path,
};
use crate::load_config_file;

pub const ORBITAL_LABELS: [(&str, usize); 6] = [("s", 0), ("p", 1), ("d", 2), ("f", 3), ("g", 4), ("h", 5)];

fn orbital_label(l_value: usize) -> String {
    ORBITAL_LABELS
        .iter()
        .find(|(_, v)| *v == l_value)
        .map(|(label, _)| label.to_string())
        .unwrap_or_else(|| format!("l{l_value}"))
}

fn format_occ(value: f64) -> String {
    if (value - value.round()).abs() < 1e-6 {
        format!("{}", value.round() as i64)
    } else {
        // Python `f"{v:g}"`：6 位有效数字。
        let formatted = format!("{value:.6}");
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

fn sorted_symbols_by_z<'a>(symbols: impl Iterator<Item = &'a String>) -> Result<Vec<String>> {
    let mut list: Vec<String> = symbols.cloned().collect();
    list.sort_by_key(|s| symbol_to_atomic_number(s).unwrap_or(999));
    Ok(list)
}

/// 元素集合 = 输入结构第 6 行元素，XA/XB 展开为选择列表。
pub fn discover_required_elements(config_file: &str) -> Result<Vec<String>> {
    let config = load_substitutionflow_config(config_file)?;
    let ht = config.get("substitutionflow").cloned().unwrap_or(Value::Null);
    let structure_path = resolve_input_structure_path(config_file)?;
    let poscar = read_poscar(&structure_path)?;
    let mut symbols: Vec<String> = Vec::new();
    for element in &poscar.elements {
        if element == "XA" {
            let sel = ht.get("XA").and_then(|v| v.as_str()).context("[substitutionflow].XA 缺失")?;
            for s in crate::selection::selection_to_symbols(sel)? {
                if !symbols.contains(&s) {
                    symbols.push(s);
                }
            }
        } else if element == "XB" {
            let sel = ht.get("XB").and_then(|v| v.as_str()).context("[substitutionflow].XB 缺失")?;
            for s in crate::selection::selection_to_symbols(sel)? {
                if !symbols.contains(&s) {
                    symbols.push(s);
                }
            }
        } else if !symbols.contains(element) {
            symbols.push(element.clone());
        }
    }
    sorted_symbols_by_z(symbols.iter())
}

// ---------------------------------------------------------------------------
// POTCAR
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PotcarCandidate {
    pub element: String,
    pub candidate_name: String,
    pub potcar_path: String,
    pub zval: f64,
    pub config_text: String,
}

fn parse_potcar_candidate(potcar_path: &str, element: &str, candidate_name: &str) -> Result<PotcarCandidate> {
    let text = std::fs::read_to_string(potcar_path).with_context(|| format!("read {potcar_path}"))?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 2 {
        bail!("POTCAR 内容不完整: {potcar_path}");
    }
    let re = Regex::new(r"ZVAL\s*=\s*([0-9.]+)").unwrap();
    let zval = match re.captures(&text).and_then(|c| c[1].parse::<f64>().ok()) {
        Some(v) => v,
        None => lines[1]
            .trim()
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("无法从 POTCAR 中解析 ZVAL: {potcar_path}"))?,
    };
    let config_text = extract_valence_configuration(&lines, zval);
    Ok(PotcarCandidate {
        element: element.to_string(),
        candidate_name: candidate_name.to_string(),
        potcar_path: potcar_path.to_string(),
        zval,
        config_text,
    })
}

fn extract_valence_configuration(lines: &[&str], zval: f64) -> String {
    let Some(start_index) = lines.iter().position(|l| l.trim() == "Atomic configuration") else {
        return "unknown".to_string();
    };
    if start_index + 2 >= lines.len() {
        return "unknown".to_string();
    }
    let re = Regex::new(r"(\d+)\s+entries").unwrap();
    let Some(caps) = re.captures(lines[start_index + 1]) else {
        return "unknown".to_string();
    };
    let entry_count: usize = caps[1].parse().unwrap_or(0);
    let mut entries: Vec<(usize, usize, f64)> = Vec::new();
    for line in &lines[start_index + 2..] {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let (Ok(n), Ok(l), Ok(occ)) = (
            parts[0].parse::<usize>(),
            parts[1].parse::<usize>(),
            parts[4].parse::<f64>(),
        ) else {
            continue;
        };
        entries.push((n, l, occ));
        if entries.len() >= entry_count {
            break;
        }
    }
    if entries.is_empty() {
        return "unknown".to_string();
    }
    let mut remaining = zval;
    let mut valence_entries: Vec<(usize, usize, f64)> = Vec::new();
    for (n, l, occ) in entries.iter().rev() {
        if remaining <= 1e-8 {
            break;
        }
        let selected = occ.min(remaining);
        if selected > 1e-8 {
            valence_entries.push((*n, *l, selected));
            remaining -= selected;
        }
    }
    valence_entries.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let text = valence_entries
        .iter()
        .map(|(n, l, occ)| format!("{n}{}{}", orbital_label(*l), format_occ(*occ)))
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() { "unknown".to_string() } else { text }
}

fn discover_candidate_groups(
    pot_root: &str,
    required_elements: &[String],
) -> Result<std::collections::BTreeMap<String, Vec<PotcarCandidate>>> {
    let pot_root = shellexpand(pot_root);
    if !Path::new(&pot_root).is_dir() {
        bail!("赝势目录不存在: {pot_root}");
    }
    let mut grouped: std::collections::BTreeMap<String, Vec<PotcarCandidate>> =
        required_elements.iter().map(|e| (e.clone(), Vec::new())).collect();
    for entry_name in crate::list_sorted(&pot_root)? {
        let entry_path = format!("{pot_root}/{entry_name}");
        if !Path::new(&entry_path).is_dir() {
            continue;
        }
        let base_symbol = entry_name.split('_').next().unwrap_or("").to_string();
        if !ELEMENT_SYMBOLS.contains(&base_symbol.as_str())
            || !required_elements.contains(&base_symbol)
        {
            continue;
        }
        let potcar_path = format!("{entry_path}/POTCAR");
        if !Path::new(&potcar_path).is_file() {
            continue;
        }
        grouped
            .get_mut(&base_symbol)
            .context("候选组缺失")?
            .push(parse_potcar_candidate(&potcar_path, &base_symbol, &entry_name)?);
    }
    for (element, candidates) in grouped.iter_mut() {
        if candidates.is_empty() {
            bail!("在 {pot_root} 中找不到元素 {element} 的 POTCAR 候选");
        }
        candidates.sort_by(|a, b| {
            let exact_a = usize::from(a.candidate_name != a.element);
            let exact_b = usize::from(b.candidate_name != b.element);
            (exact_a, a.candidate_name.clone()).cmp(&(exact_b, b.candidate_name.clone()))
        });
    }
    Ok(grouped)
}

fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    path.to_string()
}

/// 解析 `元素 = 候选名` 选择记录文件。
pub fn parse_selection_file(selection_file: &str) -> Result<std::collections::BTreeMap<String, String>> {
    if !Path::new(selection_file).exists() {
        bail!("找不到赝势选择文件: {selection_file}");
    }
    let text = std::fs::read_to_string(selection_file)?;
    let mut selections = std::collections::BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((element, candidate)) = line.split_once('=') else {
            bail!("赝势选择文件格式错误: {raw_line}");
        };
        let element = element.trim().to_string();
        let candidate = candidate.trim().to_string();
        if element.is_empty() || candidate.is_empty() {
            bail!("赝势选择文件格式错误: {raw_line}");
        }
        selections.insert(element, candidate);
    }
    Ok(selections)
}

fn write_selection_log(
    selected: &std::collections::BTreeMap<String, PotcarCandidate>,
    output_dir: &str,
) -> Result<String> {
    let dir = format!("{output_dir}/potcar_lib");
    std::fs::create_dir_all(&dir)?;
    let log_path = format!("{dir}/potcar_lib.log");
    let mut text = String::new();
    for element in sorted_symbols_by_z(selected.keys())? {
        text.push_str(&format!(
            "{} = {}\n",
            element,
            selected[&element].candidate_name
        ));
    }
    std::fs::write(&log_path, text)?;
    Ok(log_path)
}

fn prompt_choice(candidates_len: usize) -> Result<usize> {
    let stdin = io::stdin();
    loop {
        print!("请输入编号: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        stdin.lock().read_line(&mut answer)?;
        let answer = answer.trim();
        if !answer.is_empty() && answer.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(idx) = answer.parse::<usize>() {
                if (1..=candidates_len).contains(&idx) {
                    return Ok(idx - 1);
                }
            }
        }
        println!("输入无效，请重新输入候选编号。");
    }
}

fn select_candidates(
    candidate_groups: &std::collections::BTreeMap<String, Vec<PotcarCandidate>>,
    output_dir: &str,
    preselected: &std::collections::BTreeMap<String, String>,
) -> Result<std::collections::BTreeMap<String, PotcarCandidate>> {
    let mut selected = std::collections::BTreeMap::new();
    for element in sorted_symbols_by_z(candidate_groups.keys())? {
        let candidates = &candidate_groups[&element];
        if let Some(pre) = preselected.get(&element) {
            let Some(matched) = candidates.iter().find(|c| &c.candidate_name == pre) else {
                bail!("元素 {element} 的已选赝势 {pre} 在当前 --pot 目录中不存在。");
            };
            println!("元素 {element} 使用记录文件中的赝势: {}", matched.candidate_name);
            selected.insert(element.clone(), matched.clone());
            write_selection_log(&selected, output_dir)?;
            continue;
        }
        if candidates.len() == 1 {
            println!(
                "元素 {element} 只有一个赝势候选，自动选择: {}",
                candidates[0].candidate_name
            );
            selected.insert(element.clone(), candidates[0].clone());
            write_selection_log(&selected, output_dir)?;
            continue;
        }
        println!("元素 {element} 检测到 {} 个赝势候选：", candidates.len());
        for (index, candidate) in candidates.iter().enumerate() {
            println!(
                "No.{}: {}: {}, {}",
                index + 1,
                candidate.candidate_name,
                format_occ(candidate.zval),
                candidate.config_text
            );
        }
        let chosen = prompt_choice(candidates.len())?;
        selected.insert(element.clone(), candidates[chosen].clone());
        write_selection_log(&selected, output_dir)?;
    }
    Ok(selected)
}

/// `substitution_prepare_potcar` 主流程。返回 log 路径。
pub fn prepare_potcar_library(config_file: &str, pot_root: &str) -> Result<String> {
    let output_dir = std::env::current_dir()?.to_string_lossy().into_owned();
    let required_elements = discover_required_elements(config_file)?;
    println!("需要准备的元素: {}", required_elements.join(" "));
    let candidate_groups = discover_candidate_groups(pot_root, &required_elements)?;
    let log_path = format!("{output_dir}/potcar_lib/potcar_lib.log");
    let preselected = if Path::new(&log_path).exists() {
        println!("读取已有赝势选择记录: {log_path}");
        parse_selection_file(&log_path)?
    } else {
        println!("找不到赝势选择记录文件: {log_path}");
        Default::default()
    };
    let selected = select_candidates(&candidate_groups, &output_dir, &preselected)?;
    let potcar_lib_dir = format!("{output_dir}/potcar_lib");
    std::fs::create_dir_all(&potcar_lib_dir)?;
    for (element, candidate) in &selected {
        std::fs::copy(&candidate.potcar_path, format!("{potcar_lib_dir}/{element}"))?;
    }
    let log_path = write_selection_log(&selected, &output_dir)?;
    println!("已生成 {potcar_lib_dir}");
    println!("已写入赝势选择记录: {log_path}");
    Ok(log_path)
}

// ---------------------------------------------------------------------------
// QE 赝势
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QePseudoCandidate {
    pub element: String,
    pub filename: String,
    pub source_path: String,
    pub z_valence: Option<f64>,
    pub config_text: String,
}

fn parse_qe_pseudo_candidate(source_path: &str, element: &str) -> Result<QePseudoCandidate> {
    let content = std::fs::read_to_string(source_path)
        .with_context(|| format!("read {source_path}"))?;
    let z_valence = extract_z_valence(&content);
    let config_text = extract_qe_valence_configuration(&content);
    Ok(QePseudoCandidate {
        element: element.to_string(),
        filename: Path::new(source_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        source_path: source_path.to_string(),
        z_valence,
        config_text,
    })
}

fn extract_z_valence(content: &str) -> Option<f64> {
    let patterns = [
        r#"\bz_valence\b\s*=\s*["']?\s*([0-9.+\-EeDd]+)"#,
        r#"\bZ_VALENCE\b\s*=\s*["']?\s*([0-9.+\-EeDd]+)"#,
        r"(?i)<z_valence>\s*([0-9.+\-EeDd]+)\s*</z_valence>",
    ];
    for pattern in patterns {
        let re = Regex::new(pattern).ok()?;
        if let Some(caps) = re.captures(content) {
            let text = caps[1].replace(['D', 'd'], "E");
            if let Ok(v) = text.parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

fn format_configuration_entries(entries: &[(usize, usize, f64)]) -> String {
    let text = entries
        .iter()
        .filter(|(_, _, occ)| *occ > 1e-8)
        .map(|(n, l, occ)| format!("{n}{}{}", orbital_label(*l), format_occ(*occ)))
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() { "unknown".to_string() } else { text }
}

fn extract_qe_valence_configuration(content: &str) -> String {
    let block_re = Regex::new(
        r"(?is)Valence configuration:\s*(.*?)\s*(?:Generation configuration:|<PP_INPUTFILE>|</PP_INFO>)",
    )
    .unwrap();
    if let Some(caps) = block_re.captures(content) {
        let mut entries: Vec<(usize, usize, f64)> = Vec::new();
        let nl_re = Regex::new(r"^(\d+)([A-Za-z])$").unwrap();
        for raw_line in caps[1].lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.to_lowercase().starts_with("nl ") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }
            let Some(nl) = nl_re.captures(parts[0]) else { continue };
            let (Ok(n), Ok(l), Ok(occ)) = (
                parts[1].parse::<usize>(),
                parts[2].parse::<usize>(),
                parts[3].replace(['D', 'd'], "E").parse::<f64>(),
            ) else {
                continue;
            };
            if nl[1].parse::<usize>().unwrap_or(0) != n {
                continue;
            }
            entries.push((n, l, occ));
        }
        let formatted = format_configuration_entries(&entries);
        if formatted != "unknown" {
            return formatted;
        }
    }
    let patterns = [
        r#"(?i)\bvalence_configuration\b\s*=\s*["']([^"']+)["']"#,
        r#"(?i)\b(?:valence_configuration|electron_configuration|configuration)\b\s*=\s*["']([^"']+)["']"#,
        r"(?i)<(?:valence_configuration|electron_configuration|configuration)>\s*([^<]+?)\s*</(?:valence_configuration|electron_configuration|configuration)>",
    ];
    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(content) {
            let text = caps[1].split_whitespace().collect::<Vec<_>>().join(" ");
            if !text.is_empty() {
                return text;
            }
        }
    }
    if let Some(z_valence) = extract_z_valence(content) {
        let entries = extract_numeric_configuration_entries(content);
        if !entries.is_empty() {
            let mut remaining = z_valence;
            let mut valence_entries: Vec<(usize, usize, f64)> = Vec::new();
            for (n, l, occ) in entries.iter().rev() {
                if remaining <= 1e-8 {
                    break;
                }
                let selected = occ.min(remaining);
                if selected > 1e-8 {
                    valence_entries.push((*n, *l, selected));
                    remaining -= selected;
                }
            }
            valence_entries.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
            let formatted = format_configuration_entries(&valence_entries);
            if formatted != "unknown" {
                return formatted;
            }
        }
    }
    "unknown".to_string()
}

fn extract_numeric_configuration_entries(content: &str) -> Vec<(usize, usize, f64)> {
    let mut entries = Vec::new();
    let int_re = Regex::new(r"^[0-9]+$").unwrap();
    let signed_int_re = Regex::new(r"^-?[0-9]+$").unwrap();
    let float_re = Regex::new(r"^[-+]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[EeDd][-+]?[0-9]+)?$").unwrap();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        if !int_re.is_match(parts[0]) || !int_re.is_match(parts[1]) || !signed_int_re.is_match(parts[2])
            || !float_re.is_match(parts[3])
        {
            continue;
        }
        let (Ok(n), Ok(l), Ok(occ)) = (
            parts[1].parse::<usize>(),
            parts[2].parse::<usize>(),
            parts[3].replace(['D', 'd'], "E").parse::<f64>(),
        ) else {
            continue;
        };
        entries.push((n, l, occ));
    }
    entries
}

fn discover_qe_candidate_groups(
    pseudo_root: &str,
    required_elements: &[String],
) -> Result<std::collections::BTreeMap<String, Vec<QePseudoCandidate>>> {
    let pseudo_root = shellexpand(pseudo_root);
    if !Path::new(&pseudo_root).is_dir() {
        bail!("QE 赝势目录不存在: {pseudo_root}");
    }
    let mut groups: std::collections::BTreeMap<String, Vec<QePseudoCandidate>> =
        required_elements.iter().map(|e| (e.clone(), Vec::new())).collect();
    for entry_name in crate::list_sorted(&pseudo_root)? {
        let entry_path = format!("{pseudo_root}/{entry_name}");
        if !Path::new(&entry_path).is_file() {
            continue;
        }
        let token = qf_input::species::extract_pseudo_element_token(&entry_name);
        if token.is_empty() {
            continue;
        }
        let Some(matched_element) = required_elements
            .iter()
            .find(|e| e.to_lowercase() == token.to_lowercase())
            .cloned()
        else {
            continue;
        };
        groups
            .get_mut(&matched_element)
            .context("候选组缺失")?
            .push(parse_qe_pseudo_candidate(&entry_path, &matched_element)?);
    }
    for (element, candidates) in groups.iter_mut() {
        if candidates.is_empty() {
            bail!("在 {pseudo_root} 中找不到元素 {element} 对应的 QE 赝势文件");
        }
        candidates.sort_by(|a, b| a.filename.cmp(&b.filename));
    }
    Ok(groups)
}

fn format_float(value: Option<f64>) -> String {
    value.map(format_occ).unwrap_or_else(|| "unknown".to_string())
}

/// `substitution_prepare_qe_pseudo` 主流程。返回 log 路径。
pub fn prepare_qe_pseudo_library(config_file: &str, pseudo_root: &str) -> Result<String> {
    let output_dir = std::env::current_dir()?
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| std::env::current_dir().unwrap().to_string_lossy().into_owned());
    let required_elements = discover_required_elements(config_file)?;
    println!("需要准备的 QE 元素: {}", required_elements.join(" "));
    let candidate_groups = discover_qe_candidate_groups(pseudo_root, &required_elements)?;
    let log_path = format!("{output_dir}/qe_pseudo_lib/qe_pseudo_lib.log");
    let preselected = if Path::new(&log_path).exists() {
        println!("读取已有 QE 赝势选择记录: {log_path}");
        parse_selection_file(&log_path)?
    } else {
        println!("找不到 QE 赝势选择记录文件: {log_path}");
        Default::default()
    };

    let mut selected = std::collections::BTreeMap::new();
    for element in sorted_symbols_by_z(candidate_groups.keys())? {
        let candidates = &candidate_groups[&element];
        if let Some(pre) = preselected.get(&element) {
            let Some(matched) = candidates.iter().find(|c| &c.filename == pre) else {
                bail!("元素 {element} 的已选 QE 赝势 {pre} 在当前 --pp 目录中不存在。");
            };
            println!("元素 {element} 使用记录文件中的 QE 赝势: {}", matched.filename);
            selected.insert(element.clone(), matched.clone());
        } else if candidates.len() == 1 {
            println!(
                "元素 {element} 只有一个 QE 赝势候选，自动选择: {}",
                candidates[0].filename
            );
            selected.insert(element.clone(), candidates[0].clone());
        } else {
            println!("元素 {element} 检测到 {} 个 QE 赝势候选：", candidates.len());
            for (index, candidate) in candidates.iter().enumerate() {
                println!(
                    "No.{}: {}: {}, {}",
                    index + 1,
                    candidate.filename,
                    format_float(candidate.z_valence),
                    candidate.config_text
                );
            }
            let chosen = prompt_choice(candidates.len())?;
            selected.insert(element.clone(), candidates[chosen].clone());
        }
        // 每次选择即重写日志。
        let dir = format!("{output_dir}/qe_pseudo_lib");
        std::fs::create_dir_all(&dir)?;
        let mut text = String::new();
        for element in sorted_symbols_by_z(selected.keys())? {
            text.push_str(&format!("{} = {}\n", element, selected[&element].filename));
        }
        std::fs::write(&log_path, text)?;
    }
    let target_dir = format!("{output_dir}/qe_pseudo_lib");
    std::fs::create_dir_all(&target_dir)?;
    for candidate in selected.values() {
        std::fs::copy(&candidate.source_path, format!("{target_dir}/{}", candidate.filename))?;
    }
    println!("已生成 {target_dir}");
    println!("已写入 QE 赝势选择记录: {log_path}");
    Ok(log_path)
}
