//! OUTCAR 启发式状态机与 `case_status.json`（复刻 `substitutionflow/status.py`）。

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::structure::load_substitutionflow_config;
use crate::{is_nonempty_file, list_sorted, load_config_file, pressure_key};

pub const STEP_ORDER: [&str; 6] = [
    "mattersim_opt",
    "vasp_opt",
    "vasp_scf",
    "vasp_eledos",
    "vasp_process_eledos",
    "qe_phonon",
];

pub const STATUS_ORDER: [&str; 3] = ["finished", "error", "unstarted"];

pub fn phonon_mode_to_workflow(mode: &str) -> Result<&'static str> {
    match mode {
        "nosplit" => Ok("qe_phonon_nosplit"),
        "split_index" => Ok("qe_phonon_split_index"),
        "split_coords" => Ok("qe_phonon_split_coords"),
        _ => bail!(
            "[substitutionflow].phonon_mode 只允许 nosplit, split_index, split_coords，当前值为 {mode:?}。"
        ),
    }
}

pub const OPT_CONVERGED_MARKER: &str =
    "reached required accuracy - stopping structural energy minimisation";
pub const SCF_REQUIRED_MARKERS: [&str; 3] = [
    "FREE ENERGIE OF THE ION-ELECTRON SYSTEM",
    "General timing and accounting informations for this job",
    "aborting loop because EDIFF is reached",
];
pub const SCF_FORBIDDEN_MARKER: &str = "in kB  *";

pub fn get_workflow_status_path(output_dir: &str) -> String {
    format!("{output_dir}/workflow_status.json")
}

pub fn get_case_status_path(case_dir: &str) -> String {
    format!("{case_dir}/case_status.json")
}

fn load_case_config(case_dir: &str) -> Result<Value> {
    load_config_file(&format!("{case_dir}/input.toml"))
}

pub fn case_uses_mattersim(case_dir: &str) -> bool {
    load_case_config(case_dir)
        .map(|c| c.get("mattersim_opt").is_some())
        .unwrap_or(false)
}

fn resolve_case_input_structure(case_dir: &str, fallback: &str) -> String {
    let Ok(config) = load_case_config(case_dir) else {
        return fallback.to_string();
    };
    let structure_filename = config
        .get("substitutionflow")
        .and_then(|s| s.get("structure_filename"))
        .and_then(|v| v.as_str())
        .unwrap_or("structure.vasp");
    let path = format!("{case_dir}/{structure_filename}");
    if Path::new(&path).exists() {
        std::fs::canonicalize(&path)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or(path)
    } else {
        fallback.to_string()
    }
}

pub fn get_case_step_order(case_dir: &str) -> Vec<&'static str> {
    if case_uses_mattersim(case_dir) {
        STEP_ORDER.to_vec()
    } else {
        STEP_ORDER.iter().copied().filter(|s| *s != "mattersim_opt").collect()
    }
}

pub fn get_case_phonon_mode(case_dir: &str) -> Result<String> {
    let config = load_case_config(case_dir)?;
    let mode = config
        .get("substitutionflow")
        .and_then(|s| s.get("phonon_mode"))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Null => "split_index".to_string(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "split_index".to_string());
    let mode = mode.trim().to_string();
    phonon_mode_to_workflow(&mode)?;
    Ok(mode)
}

pub fn get_case_phonon_workflow(case_dir: &str) -> Result<String> {
    get_case_phonon_mode(case_dir).map(|m| phonon_mode_to_workflow(&m).unwrap().to_string())
}

pub fn get_case_phonon_workdir(case_dir: &str) -> Result<String> {
    let config = load_case_config(case_dir)?;
    if let Some(wd) = config
        .get("qe_phonon")
        .and_then(|q| q.get("work_dir"))
        .and_then(|v| v.as_str())
    {
        return Ok(wd.to_string());
    }
    get_case_phonon_workflow(case_dir)
}

fn load_json(path: &str) -> Result<Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    serde_json::from_str(&text).with_context(|| format!("parse JSON {path}"))
}

fn backup_invalid_json(path: &str) -> Result<String> {
    let mut backup_path = format!("{path}.bad");
    let mut index = 1;
    while Path::new(&backup_path).exists() {
        backup_path = format!("{path}.bad.{index}");
        index += 1;
    }
    std::fs::rename(path, &backup_path)?;
    Ok(backup_path)
}

fn save_json(path: &str, payload: &Value) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(payload)?)?;
    Ok(())
}

/// case 发现：glob `*/period_*/workflow_manifest.json`。
pub fn discover_cases(output_dir: &str) -> Result<Vec<Map<String, Value>>> {
    let output_dir_abs = std::fs::canonicalize(output_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| output_dir.to_string());
    let output_dir_name = Path::new(&output_dir_abs)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut cases = Vec::new();
    let mut manifest_paths: Vec<String> = Vec::new();
    for xa in list_sorted(&output_dir_abs)? {
        let xa_dir = format!("{output_dir_abs}/{xa}");
        if !Path::new(&xa_dir).is_dir() {
            continue;
        }
        for period in list_sorted(&xa_dir)? {
            let candidate = format!("{xa_dir}/{period}/workflow_manifest.json");
            if Path::new(&candidate).is_file() {
                manifest_paths.push(candidate);
            }
        }
    }
    manifest_paths.sort();
    for manifest_path in &manifest_paths {
        let manifest = load_json(manifest_path)?;
        let period_dir = Path::new(manifest_path).parent().unwrap();
        let period_rel = period_dir
            .strip_prefix(&output_dir_abs)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let xa_dir_rel = Path::new(&period_rel)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let period_label = period_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(entries) = manifest.get("cases").and_then(|v| v.as_array()) else {
            continue;
        };
        for case_entry in entries {
            let case_dirname = case_entry
                .get("case_dir")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let case_dir = period_dir.join(case_dirname);
            let case_rel = case_dir
                .strip_prefix(&output_dir_abs)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let meta = json!({
                "case_rel": case_rel,
                "case_display_rel": format!("{output_dir_name}/{case_rel}"),
                "case_dir": case_dir.to_string_lossy(),
                "period_dir": period_dir.to_string_lossy(),
                "period_rel": period_rel,
                "xa_dir_rel": xa_dir_rel,
                "period_label": period_label,
                "input_structure": manifest.get("input_structure").cloned().unwrap_or(Value::Null),
                "xa_symbol": manifest.get("xa_symbol").cloned().unwrap_or(Value::Null),
                "formula": case_entry.get("formula").cloned().unwrap_or(Value::Null),
                "xb_symbol": case_entry.get("xb_symbol").cloned().unwrap_or(Value::Null),
                "pressures": manifest.get("pressures").cloned().unwrap_or(Value::Array(vec![])),
            });
            cases.push(meta.as_object().cloned().unwrap_or_default());
        }
    }
    Ok(cases)
}

fn meta_str(meta: &Map<String, Value>, key: &str) -> String {
    meta.get(key)
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .unwrap_or_default()
}

fn default_pressure_status(pressures: &[Value], step_order: &[&str]) -> Map<String, Value> {
    let mut result = Map::new();
    for pressure in pressures {
        let key = pressure_key(pressure);
        let steps: Map<String, Value> = step_order
            .iter()
            .map(|s| (s.to_string(), Value::String("unstarted".into())))
            .collect();
        result.insert(key, Value::Object(steps));
    }
    result
}

fn normalize_case_step_state(value: Option<&Value>) -> String {
    let state = match value {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(other) => other.to_string(),
        None => "unstarted".to_string(),
    };
    if STATUS_ORDER.contains(&state.as_str()) {
        state
    } else {
        "unstarted".to_string()
    }
}

pub fn build_default_case_status(case_meta: &Map<String, Value>) -> Result<Value> {
    let case_dir = &meta_str(case_meta, "case_dir");
    let step_order = get_case_step_order(case_dir);
    let pressures: Vec<Value> = case_meta
        .get("pressures")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let fallback_structure = meta_str(case_meta, "input_structure");
    Ok(json!({
        "formula": meta_str(case_meta, "formula"),
        "xa_symbol": meta_str(case_meta, "xa_symbol"),
        "xb_symbol": meta_str(case_meta, "xb_symbol"),
        "input_structure": resolve_case_input_structure(case_dir, &fallback_structure),
        "pressures": default_pressure_status(&pressures, &step_order),
    }))
}

pub fn load_case_status(case_dir: &str) -> Result<Value> {
    load_json(&get_case_status_path(case_dir))
}

pub fn save_case_status(case_dir: &str, case_status: &Value) -> Result<()> {
    save_json(&get_case_status_path(case_dir), case_status)
}

fn as_object_mut<'a>(value: &'a mut Value) -> anyhow::Result<&'a mut Map<String, Value>> {
    match value {
        Value::Object(obj) => Ok(obj),
        _ => bail!("status 载荷不是对象"),
    }
}

/// `_ensure_case_status_file`：缺失/损坏/压强集变化时重建，否则规范化补齐。
pub fn ensure_case_status_file(case_meta: &Map<String, Value>) -> Result<Value> {
    let case_dir = &meta_str(case_meta, "case_dir");
    let status_path = get_case_status_path(case_dir);
    let expected = build_default_case_status(case_meta)?;
    let step_order = get_case_step_order(case_dir);
    if !Path::new(&status_path).exists() {
        save_case_status(case_dir, &expected)?;
        return Ok(expected);
    }
    let current = match load_json(&status_path) {
        Ok(v) => v,
        Err(_) => {
            let backup = backup_invalid_json(&status_path)?;
            println!("发现损坏的 case_status.json，已备份并重建: {backup}");
            save_case_status(case_dir, &expected)?;
            return Ok(expected);
        }
    };
    if !current.is_object() {
        let backup = backup_invalid_json(&status_path)?;
        println!("发现结构异常的 case_status.json，已备份并重建: {backup}");
        save_case_status(case_dir, &expected)?;
        return Ok(expected);
    }
    let expected_pressures = expected
        .get("pressures")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let current_pressures = current
        .get("pressures")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut expected_keys: Vec<&String> = expected_pressures.keys().collect();
    let mut current_keys: Vec<&String> = current_pressures.keys().collect();
    expected_keys.sort();
    current_keys.sort();
    if expected_keys != current_keys {
        save_case_status(case_dir, &expected)?;
        return Ok(expected);
    }

    let mut current = current;
    {
        let obj = as_object_mut(&mut current)?;
        if let Value::Object(expected_obj) = &expected {
            for (key, value) in expected_obj {
                if key == "pressures" {
                    continue;
                }
                obj.insert(key.clone(), value.clone());
            }
        }
        let pressures_obj = obj
            .entry("pressures")
            .or_insert(Value::Object(Map::new()));
        let pressures = as_object_mut(pressures_obj)?;
        for (pressure_key, expected_steps) in expected_pressures {
            let entry = pressures
                .entry(pressure_key)
                .or_insert(Value::Object(Map::new()));
            let entry_obj = as_object_mut(entry)?;
            let mut normalized = Map::new();
            for step in &step_order {
                let state = normalize_case_step_state(entry_obj.get(*step));
                normalized.insert(step.to_string(), Value::String(state));
            }
            let _ = expected_steps;
            *entry = Value::Object(normalized);
        }
    }
    save_case_status(case_dir, &current)?;
    Ok(current)
}

fn safe_read_text(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn outcar_has_markers(outcar_path: &str, required: &[&str], forbidden: &[&str]) -> bool {
    if !Path::new(outcar_path).exists() {
        return false;
    }
    let content = safe_read_text(outcar_path);
    if required.iter().any(|m| !content.contains(m)) {
        return false;
    }
    if forbidden.iter().any(|m| content.contains(m)) {
        return false;
    }
    true
}

fn extract_nelm(outcar_path: &str) -> Option<usize> {
    let content = safe_read_text(outcar_path);
    let re = Regex::new(r"\bNELM\s*=\s*(\d+)").ok()?;
    re.captures(&content)?.get(1)?.as_str().parse().ok()
}

fn extract_iteration_count(outcar_path: &str) -> usize {
    let content = safe_read_text(outcar_path);
    Regex::new(r"(?m)^\s*Iteration")
        .ok()
        .map(|re| re.find_iter(&content).count())
        .unwrap_or(0)
}

fn is_scf_converged(outcar_path: &str) -> bool {
    if !outcar_has_markers(outcar_path, &SCF_REQUIRED_MARKERS, &[SCF_FORBIDDEN_MARKER]) {
        return false;
    }
    let Some(nelm) = extract_nelm(outcar_path) else {
        return false;
    };
    extract_iteration_count(outcar_path) < nelm
}

fn opt_has_trace(step_dir: &str) -> bool {
    let tracked = [
        "INCAR", "POSCAR", "POTCAR", "OUTCAR", "CONTCAR", "run_vasp.sh", "oneopt.sh", "fouropt.sh",
        "fopt.sh",
    ];
    tracked.iter().any(|n| Path::new(&format!("{step_dir}/{n}")).exists())
        || crate::directory_has_any_file(step_dir)
}

fn mattersim_has_trace(step_dir: &str) -> bool {
    let tracked = [
        "POSCAR",
        "CONTCAR",
        "run_mattersim.sh",
        "run_mattersim_opt.py",
        "mattersim_opt.json",
        "mattersim_opt.log",
    ];
    tracked.iter().any(|n| Path::new(&format!("{step_dir}/{n}")).exists())
        || crate::directory_has_any_file(step_dir)
}

fn mattersim_has_runtime_trace(step_dir: &str) -> bool {
    ["CONTCAR", "mattersim_opt.json", "mattersim_opt.log"]
        .iter()
        .any(|n| Path::new(&format!("{step_dir}/{n}")).exists())
}

fn generic_step_has_trace(step_dir: &str) -> bool {
    crate::directory_has_any_file(step_dir)
}

fn process_has_trace(eledos_dir: &str) -> bool {
    for path in [
        format!("{eledos_dir}/pdos_merged.csv"),
        format!("{eledos_dir}/pdos_merged.png"),
        format!("{eledos_dir}/TDOS.dat"),
    ] {
        if Path::new(&path).exists() {
            return true;
        }
    }
    match list_sorted(eledos_dir) {
        Ok(names) => names.iter().any(|n| n.starts_with("PDOS_") && n.ends_with(".dat")),
        Err(_) => false,
    }
}

fn detect_mattersim_opt_status(pressure_dir: &str) -> String {
    let step_dir = format!("{pressure_dir}/mattersim_opt");
    let json_path = format!("{step_dir}/mattersim_opt.json");
    let contcar_path = format!("{step_dir}/CONTCAR");
    if is_nonempty_file(&json_path) && is_nonempty_file(&contcar_path) {
        let payload = load_json(&json_path).unwrap_or(Value::Null);
        let stages = payload.get("stages");
        match stages {
            Some(Value::Object(stages)) if !stages.is_empty() => {
                if stages.values().all(|s| {
                    s.get("converged").and_then(|c| c.as_bool()).unwrap_or(false)
                }) {
                    return "finished".to_string();
                }
            }
            _ => {
                let stage1 = payload
                    .get("stage1")
                    .and_then(|s| s.get("converged"))
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false);
                let stage2 = payload
                    .get("stage2")
                    .and_then(|s| s.get("converged"))
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false);
                if stage1 && stage2 {
                    return "finished".to_string();
                }
            }
        }
    }
    if mattersim_has_runtime_trace(&step_dir) {
        return "error".to_string();
    }
    if mattersim_has_trace(&step_dir) {
        return "unstarted".to_string();
    }
    "unstarted".to_string()
}

fn detect_vasp_opt_status(pressure_dir: &str) -> String {
    let step_dir = format!("{pressure_dir}/vasp_opt");
    let outcar_path = format!("{step_dir}/OUTCAR");
    let contcar_path = format!("{step_dir}/CONTCAR");
    if is_nonempty_file(&contcar_path)
        && Path::new(&outcar_path).exists()
        && safe_read_text(&outcar_path).contains(OPT_CONVERGED_MARKER)
    {
        return "finished".to_string();
    }
    if opt_has_trace(&step_dir) {
        return "error".to_string();
    }
    "unstarted".to_string()
}

fn detect_vasp_scf_status(pressure_dir: &str) -> String {
    let step_dir = format!("{pressure_dir}/vasp_scf");
    let outcar_path = format!("{step_dir}/OUTCAR");
    let chgcar_path = format!("{step_dir}/CHGCAR");
    let wavecar_path = format!("{step_dir}/WAVECAR");
    if is_nonempty_file(&outcar_path)
        && is_nonempty_file(&chgcar_path)
        && is_nonempty_file(&wavecar_path)
        && is_scf_converged(&outcar_path)
    {
        return "finished".to_string();
    }
    if generic_step_has_trace(&step_dir) {
        return "error".to_string();
    }
    "unstarted".to_string()
}

fn detect_vasp_eledos_status(pressure_dir: &str) -> String {
    let step_dir = format!("{pressure_dir}/vasp_eledos");
    let outcar_path = format!("{step_dir}/OUTCAR");
    let doscar_path = format!("{step_dir}/DOSCAR");
    if is_nonempty_file(&outcar_path)
        && is_nonempty_file(&doscar_path)
        && is_scf_converged(&outcar_path)
    {
        return "finished".to_string();
    }
    if generic_step_has_trace(&step_dir) {
        return "error".to_string();
    }
    "unstarted".to_string()
}

fn detect_vasp_process_eledos_status(pressure_dir: &str) -> String {
    let eledos_dir = format!("{pressure_dir}/vasp_eledos");
    if let Ok(names) = list_sorted(&eledos_dir) {
        let at_ef = names
            .iter()
            .filter(|n| n.ends_with("_at_ef.csv"))
            .map(|n| format!("{eledos_dir}/{n}"))
            .filter(|p| is_nonempty_file(p))
            .count();
        if at_ef > 0 {
            return "finished".to_string();
        }
    }
    if process_has_trace(&eledos_dir) {
        return "error".to_string();
    }
    "unstarted".to_string()
}

fn normalize_qe_phonon_state(value: Option<&Value>) -> String {
    let state = match value {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Null) | None => "unstarted".to_string(),
        Some(other) => other.to_string(),
    };
    if STATUS_ORDER.contains(&state.as_str()) {
        state
    } else {
        "unstarted".to_string()
    }
}

fn summarize_qe_phonon_tasks(tasks_payload: Option<&Value>) -> String {
    let Some(Value::Object(tasks)) = tasks_payload else {
        return "unstarted".to_string();
    };
    if tasks.is_empty() {
        return "unstarted".to_string();
    }
    let states: Vec<String> = tasks.values().map(|v| normalize_qe_phonon_state(Some(v))).collect();
    if states.iter().all(|s| s == "finished") {
        return "finished".to_string();
    }
    if states.iter().any(|s| s == "error") {
        return "error".to_string();
    }
    "unstarted".to_string()
}

pub fn reduce_qe_phonon_workflow_status(workflow_name: &str, workflow_status: &Value) -> Result<String> {
    let ph = workflow_status.get("ph");
    let get = |key: &str| normalize_qe_phonon_state(workflow_status.get(key));
    let stage_states: Vec<String> = match workflow_name {
        "qe_phonon_nosplit" => vec![
            get("scf"),
            normalize_qe_phonon_state(ph.and_then(|p| p.get("state")).or(ph)),
            get("q2r"),
            get("matdyn_phband"),
            get("matdyn_phdos"),
            get("phonon_plot"),
        ],
        "qe_phonon_split_index" | "qe_phonon_split_coords" => vec![
            get("scf"),
            normalize_qe_phonon_state(ph.and_then(|p| p.get("dyn0"))),
            summarize_qe_phonon_tasks(ph.and_then(|p| p.get("tasks"))),
            get("merge"),
            get("q2r"),
            get("matdyn_phband"),
            get("matdyn_phdos"),
            get("phonon_plot"),
        ],
        _ => bail!("不支持的 qe_phonon 工作流: {workflow_name}"),
    };
    if !stage_states.is_empty() && stage_states.iter().all(|s| s == "finished") {
        return Ok("finished".to_string());
    }
    for state in &stage_states {
        if state != "finished" {
            return Ok(if state == "error" { "error".into() } else { "unstarted".into() });
        }
    }
    Ok("unstarted".to_string())
}

fn detect_qe_phonon_status(case_dir: &str, pressure_dir: &str) -> String {
    let Ok(workdir) = get_case_phonon_workdir(case_dir) else {
        return "unstarted".to_string();
    };
    let workflow_status_path = format!("{pressure_dir}/{workdir}/workflow_status.json");
    if is_nonempty_file(&workflow_status_path) {
        let workflow_status = load_json(&workflow_status_path).unwrap_or(Value::Null);
        let workflow_name = get_case_phonon_workflow(case_dir).unwrap_or_default();
        return reduce_qe_phonon_workflow_status(&workflow_name, &workflow_status)
            .unwrap_or_else(|_| "unstarted".to_string());
    }
    "unstarted".to_string()
}

pub fn detect_case_pressure_status(case_dir: &str, pressure_key: &str) -> Map<String, Value> {
    let pressure_dir = format!("{case_dir}/{pressure_key}");
    let step_order = get_case_step_order(case_dir);
    let mut status_map = Map::new();
    for step in step_order {
        let state = match step {
            "mattersim_opt" => detect_mattersim_opt_status(&pressure_dir),
            "vasp_opt" => detect_vasp_opt_status(&pressure_dir),
            "vasp_scf" => detect_vasp_scf_status(&pressure_dir),
            "vasp_eledos" => detect_vasp_eledos_status(&pressure_dir),
            "vasp_process_eledos" => detect_vasp_process_eledos_status(&pressure_dir),
            "qe_phonon" => detect_qe_phonon_status(case_dir, &pressure_dir),
            _ => continue,
        };
        status_map.insert(step.to_string(), Value::String(state));
    }
    status_map
}

fn build_workflow_summary(output_dir: &str, cases: &[Map<String, Value>]) -> Result<Value> {
    let mut case_index = Map::new();
    for case_meta in cases {
        let case_dir = &meta_str(case_meta, "case_dir");
        let case_status = load_case_status(case_dir)?;
        case_index.insert(
            meta_str(case_meta, "case_rel"),
            json!({
                "formula": meta_str(case_meta, "formula"),
                "xa_symbol": meta_str(case_meta, "xa_symbol"),
                "xb_symbol": meta_str(case_meta, "xb_symbol"),
                "input_structure": case_status.get("input_structure").cloned().unwrap_or(Value::Null),
                "pressures": case_status.get("pressures").cloned().unwrap_or(Value::Null),
            }),
        );
    }
    let root_output_dir = std::fs::canonicalize(output_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| output_dir.to_string());
    Ok(json!({
        "root_output_dir": root_output_dir,
        "cases": Value::Object(case_index),
    }))
}

pub fn save_workflow_status(output_dir: &str, workflow_status: &Value) -> Result<()> {
    save_json(&get_workflow_status_path(output_dir), workflow_status)
}

pub fn load_workflow_status(output_dir: &str) -> Result<Value> {
    load_json(&get_workflow_status_path(output_dir))
}

/// `create_initial_status`（并行重建各 case 状态文件 + 汇总）。
pub fn create_initial_status(output_dir: &str) -> Result<Value> {
    let cases = discover_cases(output_dir)?;
    let _: Vec<()> = cases
        .par_iter()
        .map(|case_meta| ensure_case_status_file(case_meta).map(|_| ()))
        .collect::<Result<Vec<_>>>()?;
    let summary = build_workflow_summary(output_dir, &cases)?;
    save_workflow_status(output_dir, &summary)?;
    Ok(summary)
}

/// `refresh_status`：重新探测各 case×pressure 的每步状态。
pub fn refresh_status(output_dir: &str) -> Result<Value> {
    let cases = discover_cases(output_dir)?;
    let _: Vec<()> = cases
        .par_iter()
        .map(|case_meta| {
            let case_dir = &meta_str(case_meta, "case_dir");
            let case_status = ensure_case_status_file(case_meta)?;
            let mut updated = json!({
                "formula": case_status.get("formula").cloned().unwrap_or(Value::Null),
                "xa_symbol": case_status.get("xa_symbol").cloned().unwrap_or(Value::Null),
                "xb_symbol": case_status.get("xb_symbol").cloned().unwrap_or(Value::Null),
                "input_structure": case_status.get("input_structure").cloned().unwrap_or(Value::Null),
                "pressures": {},
            });
            let pressures = updated
                .get_mut("pressures")
                .and_then(|v| v.as_object_mut())
                .context("pressures 应为对象")?;
            if let Some(pressure_map) = case_status.get("pressures").and_then(|v| v.as_object()) {
                for pressure_key in pressure_map.keys() {
                    pressures.insert(
                        pressure_key.clone(),
                        Value::Object(detect_case_pressure_status(case_dir, pressure_key)),
                    );
                }
            }
            save_case_status(case_dir, &updated)
        })
        .collect::<Result<Vec<_>>>()?;
    let summary = build_workflow_summary(output_dir, &cases)?;
    save_workflow_status(output_dir, &summary)?;
    Ok(summary)
}

pub fn summarize_status(workflow_status: &Value) -> String {
    let total_cases = workflow_status
        .get("cases")
        .and_then(|v| v.as_object())
        .map(|m| m.len())
        .unwrap_or(0);
    format!("总 case 数: {total_cases}")
}
