//! 跨 QE/VASP 高通量工作流编排（复刻 `substitutionflow/workflow.py`）。

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use std::path::Path;
use std::process::Command;

use crate::status::{
    case_uses_mattersim, create_initial_status, discover_cases, ensure_case_status_file,
    get_case_phonon_workflow, get_case_phonon_workdir, get_case_step_order, load_case_status,
    refresh_status, save_case_status, summarize_status,
};
use crate::structure::{
    generate_cases, load_substitutionflow_config, resolve_input_structure_path,
    write_case_input_config,
};
use crate::{pressure_key, shlex_join, shlex_quote, write_job_script};

pub const STEP_DISPLAY_NAME: [(&str, &str); 6] = [
    ("mattersim_opt", "mattersim"),
    ("vasp_opt", "vasp_opt"),
    ("vasp_scf", "vasp_scf"),
    ("vasp_eledos", "vasp_eledos"),
    ("vasp_process_eledos", "vasp_process_eledos"),
    ("qe_phonon", "qe_phonon"),
];

fn step_display_name(step: &str) -> &str {
    STEP_DISPLAY_NAME
        .iter()
        .find(|(name, _)| *name == step)
        .map(|(_, display)| *display)
        .unwrap_or(step)
}

fn meta_str(meta: &Map<String, Value>, key: &str) -> String {
    meta.get(key)
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .unwrap_or_default()
}

fn meta_pressures(meta: &Map<String, Value>) -> Vec<Value> {
    meta.get("pressures")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub fn run_cli_command(command: &[String], cwd: &str) -> Result<()> {
    if command.is_empty() {
        bail!("空命令");
    }
    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .status()
        .with_context(|| format!("运行 {command:?} 于 {cwd}"))?;
    if !status.success() {
        bail!("命令失败（{}）: {command:?} @ {cwd}", status);
    }
    Ok(())
}

pub fn submit_script(work_dir: &str, script_name: &str) -> Result<()> {
    let cmd = format!("safe_sbatch {}", shlex_quote(script_name));
    println!("\n正在进入 {work_dir} 目录提交: {cmd}");
    let output = Command::new("bash")
        .arg("-c")
        .arg(&cmd)
        .current_dir(work_dir)
        .status()
        .with_context(|| format!("提交 {script_name}"))?;
    if !output.success() {
        bail!("{work_dir} 目录中的 Slurm 提交失败: {cmd}");
    }
    Ok(())
}

pub fn ensure_workflow_tree(config_file: &str, output_dir: &str) -> Result<crate::structure::GenerateResult> {
    let input_path = resolve_input_structure_path(config_file)?;
    let result = generate_cases(&input_path, config_file, output_dir)?;
    create_initial_status(output_dir)?;
    Ok(result)
}

fn elements_for_total_projection(case_dir: &str, structure_filename: &str) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(format!("{case_dir}/{structure_filename}"))?;
    let line = text.lines().nth(5).context("POSCAR 缺少元素行")?;
    Ok(line.split_whitespace().map(|s| s.to_string()).collect())
}

fn infer_structure_filename(case_dir: &str) -> String {
    let Ok(names) = std::fs::read_dir(case_dir) else {
        return "structure.vasp".to_string();
    };
    let mut candidates: Vec<String> = names
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_lowercase().ends_with(".vasp"))
        .collect();
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| "structure.vasp".to_string())
}

fn build_process_eledos_command(case_dir: &str, structure_filename: &str) -> Result<Vec<String>> {
    let elements = elements_for_total_projection(case_dir, structure_filename)?;
    let mut cmd = vec![
        "vasp_process_eledos".to_string(),
        "-d".to_string(),
        "./vasp_eledos".to_string(),
        "--elements-total".to_string(),
    ];
    cmd.extend(elements);
    Ok(cmd)
}

fn build_qe_phonon_command_for_mode(case_dir: &str, submit_mode: bool) -> Result<Vec<String>> {
    let case_config = std::fs::canonicalize(format!("{case_dir}/input.toml"))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| format!("{case_dir}/input.toml"));
    let workflow_name = get_case_phonon_workflow(case_dir)?;
    let action_flag = if submit_mode && workflow_name != "qe_phonon_nosplit" {
        "-s"
    } else {
        "-r"
    };
    Ok(vec![
        workflow_name,
        "-i".into(),
        "vasp_opt/CONTCAR".into(),
        "-c".into(),
        case_config,
        action_flag.into(),
    ])
}

fn run_commands_for_mode(
    case_dir: &str,
    structure_filename: &str,
    pressure: &Value,
    use_mattersim: bool,
    submit_mode: bool,
) -> Result<Vec<(&'static str, Vec<String>)>> {
    let pk = pressure_key(pressure);
    let vasp_opt_input = if use_mattersim {
        format!("{pk}/mattersim_opt/CONTCAR")
    } else {
        structure_filename.to_string()
    };
    Ok(vec![
        (
            "mattersim_opt",
            vec![
                "mattersim_opt".into(),
                "-i".into(),
                structure_filename.into(),
                "-c".into(),
                "input.toml".into(),
                "-p".into(),
                pk.clone(),
                "-r".into(),
            ],
        ),
        (
            "vasp_opt",
            vec![
                "vasp_opt".into(),
                "-i".into(),
                vasp_opt_input,
                "-c".into(),
                "input.toml".into(),
                "-p".into(),
                pk.clone(),
                "-r".into(),
            ],
        ),
        (
            "vasp_scf",
            vec!["vasp_scf".into(), "-i".into(), "vasp_opt/CONTCAR".into(), "-c".into(), "../input.toml".into(), "-r".into()],
        ),
        (
            "vasp_eledos",
            vec!["vasp_eledos".into(), "-i".into(), "vasp_scf/POSCAR".into(), "-c".into(), "../input.toml".into(), "-r".into()],
        ),
        (
            "vasp_process_eledos",
            build_process_eledos_command(case_dir, structure_filename)?,
        ),
        (
            "qe_phonon",
            build_qe_phonon_command_for_mode(case_dir, submit_mode)?,
        ),
    ])
}

fn setup_commands(structure_filename: &str, pressure: &Value) -> Vec<(&'static str, Vec<String>)> {
    let pk = pressure_key(pressure);
    vec![
        (
            "mattersim_opt",
            vec!["mattersim_opt".into(), "-i".into(), structure_filename.into(), "-c".into(), "input.toml".into(), "-p".into(), pk.clone()],
        ),
        (
            "vasp_opt",
            vec!["vasp_opt".into(), "-i".into(), structure_filename.into(), "-c".into(), "input.toml".into(), "-p".into(), pk.clone()],
        ),
    ]
}

fn step_cwd(case_dir: &str, pressure: &Value, step: &str) -> String {
    if step == "mattersim_opt" || step == "vasp_opt" {
        return case_dir.to_string();
    }
    format!("{case_dir}/{}", pressure_key(pressure))
}

fn first_unfinished_step(
    case_status: &Value,
    pressure_key_str: &str,
    step_order: &[&str],
) -> Option<String> {
    let pressure_status = case_status
        .get("pressures")
        .and_then(|p| p.get(pressure_key_str))?;
    for step in step_order {
        let state = pressure_status.get(*step).and_then(|v| v.as_str()).unwrap_or("unstarted");
        if state != "finished" {
            return Some(step.to_string());
        }
    }
    None
}

fn step_display(step_order: &[&str], step: &str) -> String {
    let index = step_order.iter().position(|s| *s == step).unwrap_or(0) + 1;
    format!("{index:02}.{:13}", step_display_name(step))
}

fn step_cli_family(step: &str) -> &'static str {
    if step.starts_with("vasp_") {
        "vaspflow"
    } else if step.starts_with("qe_") {
        "qeflow"
    } else {
        "substitutionflow"
    }
}

fn render_progress_bar(current: usize, total: usize, width: usize) -> String {
    let safe_total = total.max(1);
    let clamped = current.min(safe_total);
    let filled = width * clamped / safe_total;
    format!("[{}{}] {clamped}/{safe_total}", "#".repeat(filled), "-".repeat(width - filled))
}

fn print_progress(
    event: &str,
    state: &str,
    case_meta: &Map<String, Value>,
    pressure: &Value,
    step_order: &[&str],
    step: &str,
    reason: &str,
) {
    let case_display_path = format!("{}/{}", meta_str(case_meta, "case_display_rel"), pressure_key(pressure));
    println!(
        "[{}] {} state:{state:<10} event:{event:<8} formula:{:<12} pressure:{:<4} {} reason:{}",
        step_cli_family(step),
        step_display(step_order, step),
        meta_str(case_meta, "formula"),
        pressure_key(pressure),
        case_display_path,
        reason,
    );
}

fn print_case_pressure_progress(
    stage: &str,
    current: usize,
    total: usize,
    case_meta: &Map<String, Value>,
    pressure: &Value,
    first_step: Option<&str>,
) {
    let first_step_text = first_step.map(step_display_name).unwrap_or("all_finished");
    println!(
        "{stage}: {} first:{:<18} formula:{:<12} pressure:{:<4} case:{}/{}",
        render_progress_bar(current, total, 24),
        first_step_text,
        meta_str(case_meta, "formula"),
        pressure_key(pressure),
        meta_str(case_meta, "case_display_rel"),
        pressure_key(pressure),
    );
}

pub fn generate_opt_only(case_meta: &Map<String, Value>, pressure: &Value) -> Result<()> {
    let case_dir = &meta_str(case_meta, "case_dir");
    let structure_filename = case_meta
        .get("structure_filename")
        .and_then(|v| v.as_str())
        .unwrap_or("structure.vasp");
    let use_mattersim = case_uses_mattersim(case_dir);
    let commands = setup_commands(structure_filename, pressure);
    let first_step = if use_mattersim { "mattersim_opt" } else { "vasp_opt" };
    let (_, command) = commands
        .into_iter()
        .find(|(name, _)| *name == first_step)
        .context("setup 命令缺失")?;
    run_cli_command(&command, case_dir)
}

fn load_step_order_and_structure(case_meta: &mut Map<String, Value>) -> Result<Vec<&'static str>> {
    let case_dir = meta_str(case_meta, "case_dir");
    let structure_filename = infer_structure_filename(&case_dir);
    case_meta.insert(
        "structure_filename".into(),
        Value::String(structure_filename),
    );
    Ok(get_case_step_order(&case_dir))
}

fn run_case_pressure_inner(
    output_dir: &str,
    case_meta: &Map<String, Value>,
    pressure: &Value,
    refresh_before: bool,
) -> Result<()> {
    if refresh_before {
        refresh_status(output_dir)?;
    }
    let pk = pressure_key(pressure);
    let case_dir = meta_str(case_meta, "case_dir");
    let case_status = load_case_status(&case_dir)?;
    let step_order = get_case_step_order(&case_dir);
    let use_mattersim = case_uses_mattersim(&case_dir);
    let structure_filename = case_meta
        .get("structure_filename")
        .and_then(|v| v.as_str())
        .unwrap_or("structure.vasp");
    let first_step = match first_unfinished_step(&case_status, &pk, &step_order) {
        Some(s) => s,
        None => {
            print_progress("skip", "finished", case_meta, pressure, &step_order, step_order[step_order.len() - 1], "all_finished");
            return Ok(());
        }
    };
    let first_state = case_status
        .get("pressures")
        .and_then(|p| p.get(&pk))
        .and_then(|p| p.get(&first_step))
        .and_then(|v| v.as_str())
        .unwrap_or("unstarted")
        .to_string();
    if first_state == "error" {
        print_progress("break", "error", case_meta, pressure, &step_order, &first_step, "error");
        return Ok(());
    }
    let commands = run_commands_for_mode(&case_dir, structure_filename, pressure, use_mattersim, false)?;
    let mut started = false;
    for step in &step_order {
        if *step == first_step {
            started = true;
        }
        if !started {
            continue;
        }
        print_progress("run", "unstarted", case_meta, pressure, &step_order, step, "");
        let (_, command) = commands
            .iter()
            .find(|(name, _)| name == step)
            .context("步骤命令缺失")?
            .clone();
        let cwd = step_cwd(&case_dir, pressure, step);
        match run_cli_command(&command, &cwd) {
            Ok(()) => {
                refresh_status(output_dir)?;
                let case_status = load_case_status(&case_dir)?;
                let step_state = case_status
                    .get("pressures")
                    .and_then(|p| p.get(&pk))
                    .and_then(|p| p.get(*step))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unstarted")
                    .to_string();
                if step_state != "finished" {
                    print_progress("break", &step_state, case_meta, pressure, &step_order, step, "post_refresh");
                    break;
                }
            }
            Err(_) => {
                refresh_status(output_dir)?;
                print_progress("break", "error", case_meta, pressure, &step_order, step, "exception");
                return Err(anyhow::anyhow!("步骤 {step} 执行失败"));
            }
        }
    }
    Ok(())
}

fn load_submit_groups_from_config(config_file: &str) -> Result<usize> {
    let config = match crate::load_config_file(config_file) {
        Ok(c) => c,
        Err(_) => return Ok(1),
    };
    let submit_groups = config
        .get("substitutionflow")
        .and_then(|s| s.get("submit_groups"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    if submit_groups < 1 {
        bail!("[substitutionflow].submit_groups 必须大于等于 1。");
    }
    Ok(submit_groups as usize)
}

fn load_submit_groups_from_output_dir(output_dir: &str) -> Result<usize> {
    let cases = discover_cases(output_dir)?;
    let Some(first) = cases.first() else {
        return Ok(1);
    };
    load_submit_groups_from_config(&format!("{}/input.toml", meta_str(first, "case_dir")))
}

fn load_slurm_header(output_dir: &str) -> String {
    let Ok(cases) = discover_cases(output_dir) else {
        return String::new();
    };
    let Some(first) = cases.first() else {
        return String::new();
    };
    crate::load_config_file(&format!("{}/input.toml", meta_str(first, "case_dir")))
        .ok()
        .and_then(|c| {
            c.get("slurm")
                .and_then(|s| s.get("header"))
                .and_then(|h| h.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

fn echo_step(step: &str, message: &str) -> String {
    format!(
        "echo {}",
        shlex_quote(&format!("[{}] {}", step_cli_family(step), message))
    )
}

#[derive(Clone)]
struct Unit {
    case_dir: String,
    case_rel: String,
    formula: String,
    pressure_key: String,
    first_step: String,
    commands: Vec<String>,
}

fn append_submit_step_commands(
    commands: &mut Vec<String>,
    unit: &Unit,
    step: &str,
    work_dir: &str,
    command: &str,
) {
    let display = step_display_name(step);
    commands.extend([
        echo_step(step, &"-".repeat(60)),
        echo_step(step, &format!("case: {}", unit.case_rel)),
        echo_step(step, &format!("formula: {}", unit.formula)),
        echo_step(step, &format!("pressure: {}", unit.pressure_key)),
        echo_step(step, &format!("step: {display}")),
        echo_step(step, &format!("work_dir: {work_dir}")),
        echo_step(step, &format!("command: {command}")),
        format!("if ! cd \"{work_dir}\"; then"),
        echo_step(step, &format!("failed: {display}")),
        "  exit 1".to_string(),
        "fi".to_string(),
        format!("if ! {command}; then"),
        echo_step(step, &format!("failed: {display}")),
        "  exit 1".to_string(),
        "fi".to_string(),
        echo_step(step, &format!("done: {display}")),
    ]);
}

fn collect_configured_units(output_dir: &str) -> Result<Vec<Unit>> {
    let cases = discover_cases(output_dir)?;
    let mut units = Vec::new();
    for case_meta in &cases {
        let case_dir = meta_str(case_meta, "case_dir");
        let structure_filename = infer_structure_filename(&case_dir);
        let step_order = get_case_step_order(&case_dir);
        let use_mattersim = case_uses_mattersim(&case_dir);
        for pressure in meta_pressures(case_meta) {
            let step_commands =
                run_commands_for_mode(&case_dir, &structure_filename, &pressure, use_mattersim, true)?;
            let mut commands: Vec<String> = Vec::new();
            let unit = Unit {
                case_dir: case_dir.clone(),
                case_rel: meta_str(case_meta, "case_rel"),
                formula: meta_str(case_meta, "formula"),
                pressure_key: pressure_key(&pressure),
                first_step: step_order[0].to_string(),
                commands: Vec::new(),
            };
            for step in &step_order {
                let command = step_commands
                    .iter()
                    .find(|(name, _)| name == step)
                    .map(|(_, c)| c.clone())
                    .context("步骤命令缺失")?;
                append_submit_step_commands(
                    &mut commands,
                    &unit,
                    step,
                    &step_cwd(&case_dir, &pressure, step),
                    &shlex_join(&command),
                );
            }
            units.push(Unit { commands, ..unit });
        }
    }
    Ok(units)
}

fn collect_pending_units(output_dir: &str, refresh_before: bool) -> Result<Vec<Unit>> {
    if refresh_before {
        refresh_status(output_dir)?;
    }
    let cases = discover_cases(output_dir)?;
    let mut units = Vec::new();
    for case_meta in &cases {
        let case_dir = meta_str(case_meta, "case_dir");
        let structure_filename = infer_structure_filename(&case_dir);
        let case_status = load_case_status(&case_dir)?;
        let step_order = get_case_step_order(&case_dir);
        let use_mattersim = case_uses_mattersim(&case_dir);
        for pressure in meta_pressures(case_meta) {
            let pk = pressure_key(&pressure);
            let Some(first_step) = first_unfinished_step(&case_status, &pk, &step_order) else {
                continue;
            };
            let step_commands =
                run_commands_for_mode(&case_dir, &structure_filename, &pressure, use_mattersim, true)?;
            let mut commands: Vec<String> = Vec::new();
            let unit = Unit {
                case_dir: case_dir.clone(),
                case_rel: meta_str(case_meta, "case_rel"),
                formula: meta_str(case_meta, "formula"),
                pressure_key: pk,
                first_step: first_step.clone(),
                commands: Vec::new(),
            };
            let mut started = false;
            for step in &step_order {
                if *step == first_step {
                    started = true;
                }
                if !started {
                    continue;
                }
                let command = step_commands
                    .iter()
                    .find(|(name, _)| name == step)
                    .map(|(_, c)| c.clone())
                    .context("步骤命令缺失")?;
                append_submit_step_commands(
                    &mut commands,
                    &unit,
                    step,
                    &step_cwd(&case_dir, &pressure, step),
                    &shlex_join(&command),
                );
            }
            units.push(Unit { commands, ..unit });
        }
    }
    Ok(units)
}

fn split_pending_units(units: Vec<Unit>, submit_groups: usize) -> Vec<Vec<Unit>> {
    if units.is_empty() {
        return Vec::new();
    }
    let actual_groups = units.len().min(submit_groups).max(1);
    let base_size = units.len() / actual_groups;
    let remainder = units.len() % actual_groups;
    let mut groups = Vec::new();
    let mut start = 0usize;
    for index in 0..actual_groups {
        let size = base_size + usize::from(index < remainder);
        groups.push(units[start..start + size].to_vec());
        start += size;
    }
    groups
}

struct ScriptBundle {
    total_units: usize,
    configured_groups: usize,
    actual_groups: usize,
    scripts: Vec<(String, usize)>,
}

fn build_submit_scripts_from_units(
    output_dir: &str,
    submit_groups: usize,
    units: Vec<Unit>,
) -> Result<ScriptBundle> {
    let header = load_slurm_header(output_dir);
    let groups = split_pending_units(units, submit_groups);
    let mut scripts = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let script_name = format!("submit_group_{:02}.sh", index + 1);
        let script_path = format!("{output_dir}/{script_name}");
        let mut commands: Vec<String> = Vec::new();
        for unit in group {
            commands.extend(unit.commands.clone());
        }
        write_job_script(&script_path, &header, &commands)?;
        scripts.push((script_name, group.len()));
    }
    Ok(ScriptBundle {
        total_units: scripts.iter().map(|(_, c)| c).sum(),
        configured_groups: submit_groups,
        actual_groups: scripts.len(),
        scripts,
    })
}

fn format_submit_summary(bundle: &ScriptBundle, total_label: &str) -> String {
    let mut lines = vec![
        format!("{total_label}: {}", bundle.total_units),
        format!("计划分组数: {}", bundle.configured_groups),
        format!("实际生成组数: {}", bundle.actual_groups),
    ];
    for (name, count) in &bundle.scripts {
        lines.push(format!("{name}: {count} 个完整压强点单元"));
    }
    lines.join("\n")
}

fn submit_group_scripts(output_dir: &str, bundle: &ScriptBundle) -> Result<()> {
    let total = bundle.scripts.len();
    for (index, (name, count)) in bundle.scripts.iter().enumerate() {
        println!(
            "提交进度: {} group:{name}",
            render_progress_bar(index + 1, total, 24)
        );
        println!("提交 {name}，包含 {count} 个完整压强点单元");
        submit_script(output_dir, name)?;
    }
    Ok(())
}

/// `run_workflow`。
pub fn run_workflow(config_file: &str, output_dir: &str, run_calc: bool, submit: bool) -> Result<String> {
    let result = ensure_workflow_tree(config_file, output_dir)?;
    let mut cases = discover_cases(output_dir)?;
    for case_meta in &mut cases {
        case_meta.insert(
            "structure_filename".into(),
            Value::String(result.structure_filename.clone()),
        );
        for pressure in meta_pressures(case_meta) {
            generate_opt_only(case_meta, &pressure)?;
        }
    }

    if run_calc {
        let mut items: Vec<(&Map<String, Value>, Value)> = Vec::new();
        for case_meta in &cases {
            for pressure in meta_pressures(case_meta) {
                items.push((case_meta, pressure));
            }
        }
        let total_items = items.len();
        for (index, (case_meta, pressure)) in items.iter().enumerate() {
            let pk = pressure_key(pressure);
            let case_dir = meta_str(case_meta, "case_dir");
            let case_status = load_case_status(&case_dir)?;
            let step_order = get_case_step_order(&case_dir);
            let first_step = first_unfinished_step(&case_status, &pk, &step_order);
            print_case_pressure_progress("运行进度", index + 1, total_items, case_meta, pressure, first_step.as_deref());
            let refreshed = refresh_status(output_dir)?;
            let _ = refreshed;
            let case_status = load_case_status(&case_dir)?;
            let use_mattersim = case_uses_mattersim(&case_dir);
            let commands = run_commands_for_mode(
                &case_dir,
                case_meta
                    .get("structure_filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("structure.vasp"),
                pressure,
                use_mattersim,
                false,
            )?;
            for step in &step_order {
                let current_state = case_status
                    .get("pressures")
                    .and_then(|p| p.get(&pk))
                    .and_then(|p| p.get(*step))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unstarted")
                    .to_string();
                if current_state == "finished" {
                    print_progress("skip", "finished", case_meta, pressure, &step_order, step, "finished");
                    continue;
                }
                if current_state == "error" {
                    print_progress("break", "error", case_meta, pressure, &step_order, step, "error");
                    break;
                }
                print_progress("run", "unstarted", case_meta, pressure, &step_order, step, "");
                let (_, command) = commands
                    .iter()
                    .find(|(name, _)| name == step)
                    .context("步骤命令缺失")?
                    .clone();
                let cwd = step_cwd(&case_dir, pressure, step);
                if run_cli_command(&command, &cwd).is_err() {
                    refresh_status(output_dir)?;
                    break;
                }
                refresh_status(output_dir)?;
                let case_status = load_case_status(&case_dir)?;
                let step_state = case_status
                    .get("pressures")
                    .and_then(|p| p.get(&pk))
                    .and_then(|p| p.get(*step))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unstarted")
                    .to_string();
                if step_state != "finished" {
                    print_progress("break", &step_state, case_meta, pressure, &step_order, step, "post_refresh");
                    break;
                }
            }
        }
        return Ok(summarize_status(&refresh_status(output_dir)?));
    }

    let submit_groups = load_submit_groups_from_config(config_file)?;
    let script_bundle = build_submit_scripts_from_units(output_dir, submit_groups, collect_configured_units(output_dir)?)?;
    if submit {
        submit_group_scripts(output_dir, &script_bundle)?;
    }
    Ok(format_submit_summary(&script_bundle, "总生成单元数"))
}

fn sync_qe_phonon_poscars(output_dir: &str) -> Result<usize> {
    let cases = discover_cases(output_dir)?;
    let mut repaired = 0usize;
    for case_meta in &cases {
        let case_dir = meta_str(case_meta, "case_dir");
        let qe_workdir = get_case_phonon_workdir(&case_dir).unwrap_or_default();
        for pressure in meta_pressures(case_meta) {
            let pk = pressure_key(&pressure);
            let pressure_dir = format!("{case_dir}/{pk}");
            let qe_dir = format!("{pressure_dir}/{qe_workdir}");
            let contcar = format!("{pressure_dir}/vasp_opt/CONTCAR");
            if Path::new(&qe_dir).is_dir() && Path::new(&contcar).exists() {
                std::fs::copy(&contcar, format!("{qe_dir}/POSCAR"))?;
                repaired += 1;
            }
        }
    }
    Ok(repaired)
}

fn prepare_continue_workspace(
    output_dir: &str,
    config_file: &str,
) -> Result<(Value, ScriptBundle, usize)> {
    rewrite_case_input_configs(output_dir, config_file)?;
    let summary = refresh_status(output_dir)?;
    let repaired = sync_qe_phonon_poscars(output_dir)?;
    let submit_groups = load_submit_groups_from_output_dir(output_dir)?;
    let bundle = build_submit_scripts_from_units(
        output_dir,
        submit_groups,
        collect_pending_units(output_dir, false)?,
    )?;
    Ok((summary, bundle, repaired))
}

fn rewrite_case_input_configs(output_dir: &str, config_file: &str) -> Result<usize> {
    let root_config = load_substitutionflow_config(config_file)?;
    let cases = discover_cases(output_dir)?;
    let count = cases.len();
    for case_meta in &cases {
        write_case_input_config(&format!("{}/input.toml", meta_str(case_meta, "case_dir")), &root_config)?;
    }
    Ok(count)
}

fn rewrite_qe_phonon_workflow_status_errors(case_dir: &str, pressure_key_str: &str) -> Result<usize> {
    let pressure_dir = format!("{case_dir}/{pressure_key_str}");
    let qe_workdir = get_case_phonon_workdir(case_dir).unwrap_or_default();
    let workflow_status_path = format!("{pressure_dir}/{qe_workdir}/workflow_status.json");
    if !Path::new(&workflow_status_path).exists() {
        return Ok(0);
    }
    let mut workflow_status = match crate::load_config_file(&workflow_status_path) {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };
    let mut rewritten = 0usize;
    fn rewrite_payload(payload: &mut Value, rewritten: &mut usize) {
        match payload {
            Value::Object(obj) => {
                for (_key, value) in obj.iter_mut() {
                    if let Value::String(s) = value {
                        if s == "error" {
                            *value = Value::String("unstarted".into());
                            *rewritten += 1;
                        }
                    } else {
                        rewrite_payload(value, rewritten);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    rewrite_payload(item, rewritten);
                }
            }
            _ => {}
        }
    }
    rewrite_payload(&mut workflow_status, &mut rewritten);
    if rewritten > 0 {
        std::fs::write(
            &workflow_status_path,
            serde_json::to_string_pretty(&workflow_status)?,
        )?;
    }
    Ok(rewritten)
}

/// `rewrite_workflow_states`。
pub fn rewrite_workflow_states(output_dir: &str, config_file: &str) -> Result<String> {
    let rewritten_case_configs = rewrite_case_input_configs(output_dir, config_file)?;
    let summary = refresh_status(output_dir)?;
    let repaired_pressures = sync_qe_phonon_poscars(output_dir)?;
    let cases = discover_cases(output_dir)?;
    let mut rewritten_count = 0usize;
    let mut rewritten_qe_count = 0usize;
    for case_meta in &cases {
        let case_dir = meta_str(case_meta, "case_dir");
        let step_order = get_case_step_order(&case_dir);
        let mut case_status = load_case_status(&case_dir)?;
        let mut rewritten_pressures: Vec<String> = Vec::new();
        for pressure in meta_pressures(case_meta) {
            let pk = pressure_key(&pressure);
            let status_obj = case_status
                .get_mut("pressures")
                .and_then(|p| p.get_mut(&pk))
                .and_then(|v| v.as_object_mut())
                .context("pressures 应为对象")?;
            for step in &step_order {
                if status_obj
                    .get(*step)
                    .and_then(|v| v.as_str())
                    .unwrap_or("unstarted")
                    != "error"
                {
                    continue;
                }
                status_obj.insert(step.to_string(), Value::String("unstarted".into()));
                rewritten_count += 1;
                if !rewritten_pressures.contains(&pk) {
                    rewritten_pressures.push(pk.clone());
                }
            }
        }
        for pk in &rewritten_pressures {
            rewritten_qe_count += rewrite_qe_phonon_workflow_status_errors(&case_dir, pk)?;
        }
        save_case_status(&case_dir, &case_status)?;
    }
    create_initial_status(output_dir)?;
    let submit_groups = load_submit_groups_from_output_dir(output_dir)?;
    let script_bundle = build_submit_scripts_from_units(
        output_dir,
        submit_groups,
        collect_pending_units(output_dir, false)?,
    )?;
    Ok(format!(
        "{}\n{}\n重写步骤数: {rewritten_count}\n重写 qe_phonon 子步骤数: {rewritten_qe_count}\n同步 qe_phonon POSCAR 压强点数: {repaired_pressures}\n重写 case 配置数: {rewritten_case_configs}",
        summarize_status(&summary),
        format_submit_summary(&script_bundle, "总未完成单元数"),
    ))
}

/// `continue_workflow`。
pub fn continue_workflow(
    output_dir: &str,
    run_calc: bool,
    submit: bool,
    config_file: &str,
    rewrite_states: bool,
) -> Result<String> {
    if rewrite_states && !run_calc && !submit {
        return rewrite_workflow_states(output_dir, config_file);
    }

    let mut cases = Vec::new();
    if rewrite_states {
        rewrite_case_input_configs(output_dir, config_file)?;
        refresh_status(output_dir)?;
        sync_qe_phonon_poscars(output_dir)?;
        cases = discover_cases(output_dir)?;
    } else if !run_calc && !submit {
        let (summary, script_bundle, repaired) = prepare_continue_workspace(output_dir, config_file)?;
        return Ok(format!(
            "{}\n{}\n同步 qe_phonon POSCAR 压强点数: {repaired}",
            summarize_status(&summary),
            format_submit_summary(&script_bundle, "总未完成单元数"),
        ));
    } else {
        let (_summary, script_bundle, _repaired) = prepare_continue_workspace(output_dir, config_file)?;
        let _ = script_bundle;
        cases = discover_cases(output_dir)?;
    }

    for case_meta in &mut cases {
        load_step_order_and_structure(case_meta)?;
    }

    if run_calc {
        let mut items: Vec<(Map<String, Value>, Value)> = Vec::new();
        for case_meta in &cases {
            for pressure in meta_pressures(case_meta) {
                items.push((case_meta.clone(), pressure));
            }
        }
        let total_items = items.len();
        for (index, (case_meta, pressure)) in items.iter().enumerate() {
            let pk = pressure_key(pressure);
            let case_dir = meta_str(&case_meta, "case_dir");
            let case_status = load_case_status(&case_dir)?;
            let step_order = get_case_step_order(&case_dir);
            let first_step = first_unfinished_step(&case_status, &pk, &step_order);
            print_case_pressure_progress("运行进度", index + 1, total_items, &case_meta, pressure, first_step.as_deref());
            if run_case_pressure_inner(output_dir, &case_meta, pressure, false).is_err() {
                continue;
            }
        }
        return Ok(summarize_status(&refresh_status(output_dir)?));
    }

    let submit_groups = load_submit_groups_from_output_dir(output_dir)?;
    let script_bundle = build_submit_scripts_from_units(
        output_dir,
        submit_groups,
        collect_pending_units(output_dir, false)?,
    )?;
    submit_group_scripts(output_dir, &script_bundle)?;
    Ok(format_submit_summary(&script_bundle, "总未完成单元数"))
}

/// `ensure_case_status_file` 的对外复用（generate 流程不用）。
pub fn ensure_status_for(case_meta: &Map<String, Value>) -> Result<Value> {
    ensure_case_status_file(case_meta)
}
