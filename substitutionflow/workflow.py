#!/usr/bin/env python3
import os
import shutil
import subprocess
import json
import shlex
import sys
from collections import OrderedDict
from typing import Dict, List, Optional, Tuple

from public.config_templates import load_config_file
from public.job import submit_script, write_job_script

from substitutionflow.status import (
    case_uses_mattersim,
    create_initial_status,
    discover_cases,
    get_case_phonon_workdir,
    get_case_phonon_workflow,
    get_case_step_order,
    load_case_status,
    load_workflow_status,
    refresh_status,
    save_case_status,
    summarize_status,
)
from substitutionflow.structure_substitution import (
    generate_cases,
    load_substitutionflow_config,
    resolve_input_structure_path,
    write_case_input_config,
)

STEP_DISPLAY_NAME = {
    "mattersim_opt": "mattersim",
    "vasp_opt": "vasp_opt",
    "vasp_scf": "vasp_scf",
    "vasp_eledos": "vasp_eledos",
    "vasp_process_eledos": "vasp_process_eledos",
    "qe_phonon": "qe_phonon",
}


def run_cli_command(command: List[str], cwd: str) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def _pressure_key(pressure) -> str:
    return f"{pressure:g}" if isinstance(pressure, (int, float)) else str(pressure)


def ensure_workflow_tree(config_file: str, output_dir: str) -> Dict:
    input_path = resolve_input_structure_path(config_file)
    result = generate_cases(input_path, config_file, output_dir)
    create_initial_status(output_dir)
    return result


def _elements_for_total_projection(case_dir: str, structure_filename: str) -> List[str]:
    poscar_path = os.path.join(case_dir, structure_filename)
    with open(poscar_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
    return lines[5].split()


def _infer_structure_filename(case_dir: str) -> str:
    candidates = [name for name in os.listdir(case_dir) if name.lower().endswith(".vasp")]
    if candidates:
        return sorted(candidates)[0]
    return "structure.vasp"


def _build_process_eledos_command(case_dir: str, structure_filename: str) -> List[str]:
    elements = _elements_for_total_projection(case_dir, structure_filename)
    return [
        "vasp_process_eledos",
        "-d",
        "./vasp_eledos",
        "--elements-total",
        *elements,
    ]


def _build_qe_phonon_command(case_dir: str) -> List[str]:
    return _build_qe_phonon_command_for_mode(case_dir, submit_mode=False)


def _build_qe_phonon_command_for_mode(case_dir: str, submit_mode: bool) -> List[str]:
    case_config = os.path.abspath(os.path.join(case_dir, "input.toml"))
    workflow_name = get_case_phonon_workflow(case_dir)
    action_flag = "-r"
    if submit_mode and workflow_name in {"qe_phonon_split_index", "qe_phonon_split_coords"}:
        action_flag = "-s"
    return [
        workflow_name,
        "-i",
        "vasp_opt/CONTCAR",
        "-c",
        case_config,
        action_flag,
    ]


def _setup_commands(structure_filename: str, pressure) -> Dict[str, List[str]]:
    pressure_key = _pressure_key(pressure)
    return {
        "mattersim_opt": ["mattersim_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key],
        "vasp_opt": ["vasp_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key],
    }


def _run_commands(case_dir: str, structure_filename: str, pressure, use_mattersim: bool) -> Dict[str, List[str]]:
    return _run_commands_for_mode(case_dir, structure_filename, pressure, use_mattersim, submit_mode=False)


def _run_commands_for_mode(
    case_dir: str,
    structure_filename: str,
    pressure,
    use_mattersim: bool,
    submit_mode: bool,
) -> Dict[str, List[str]]:
    pressure_key = _pressure_key(pressure)
    vasp_opt_input = os.path.join(pressure_key, "mattersim_opt", "CONTCAR") if use_mattersim else structure_filename
    return {
        "mattersim_opt": ["mattersim_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key, "-r"],
        "vasp_opt": ["vasp_opt", "-i", vasp_opt_input, "-c", "input.toml", "-p", pressure_key, "-r"],
        "vasp_scf": ["vasp_scf", "-i", "vasp_opt/CONTCAR", "-c", "../input.toml", "-r"],
        "vasp_eledos": ["vasp_eledos", "-i", "vasp_scf/POSCAR", "-c", "../input.toml", "-r"],
        "vasp_process_eledos": _build_process_eledos_command(case_dir, structure_filename),
        "qe_phonon": _build_qe_phonon_command_for_mode(case_dir, submit_mode=submit_mode),
    }


def _step_cwd(case_dir: str, pressure, step: str, use_mattersim: bool) -> str:
    if step in {"mattersim_opt", "vasp_opt"}:
        return case_dir
    return os.path.join(case_dir, _pressure_key(pressure))


def _first_unfinished_step(case_status: Dict, pressure_key: str, step_order: Tuple[str, ...]) -> Optional[str]:
    pressure_status = case_status["pressures"][pressure_key]
    for step in step_order:
        if pressure_status[step] != "finished":
            return step
    return None


def _step_display(step_order: Tuple[str, ...], step: str) -> str:
    step_index = step_order.index(step) + 1
    step_name = STEP_DISPLAY_NAME.get(step, step)
    return f"{step_index:02d}.{step_name:<13}"


def _print_progress(
    event: str,
    state: str,
    case_meta: Dict,
    pressure,
    step_order: Tuple[str, ...],
    step: str,
    **fields: str,
) -> None:
    case_display_path = os.path.join(case_meta["case_display_rel"], _pressure_key(pressure))
    reason_value = fields.pop("reason", "")
    extra_parts = [f"reason:{reason_value:<10}"]
    for key, value in fields.items():
        display_key = key[:-1] if key.endswith("_") else key
        extra_parts.append(f"{display_key}:{value}")
    line = (
        f"{_step_display(step_order, step)} "
        f"state:{state:<10} "
        f"event:{event:<8} "
        f"formula:{case_meta['formula']:<12} "
        f"pressure:{_pressure_key(pressure):<4}"
    )
    if extra_parts:
        line += " " + " ".join(extra_parts)
    line += f" case:{case_display_path}"
    print(line)


def _print_case_stage_progress(stage: str, current: int, total: int, case_meta: Dict) -> None:
    safe_total = max(total, 1)
    width = 24
    filled = int(width * min(current, safe_total) / safe_total)
    bar = f"[{'#' * filled}{'-' * (width - filled)}] {min(current, safe_total)}/{safe_total}"
    sys.stdout.write(
        f"\r{stage}: {bar} {case_meta['formula']} {case_meta['case_display_rel']}"
    )
    if current >= safe_total:
        sys.stdout.write("\n")
    sys.stdout.flush()


def _print_compact_stage_progress(stage: str, current: int, total: int, case_label: str) -> None:
    sys.stdout.write(f"\r{stage}: {_render_progress_bar(current, total)} {case_label}")
    if current >= max(total, 1):
        sys.stdout.write("\n")
    sys.stdout.flush()


def _render_progress_bar(current: int, total: int, width: int = 24) -> str:
    safe_total = max(total, 1)
    filled = int(width * min(current, safe_total) / safe_total)
    return f"[{'#' * filled}{'-' * (width - filled)}] {min(current, safe_total)}/{safe_total}"


def _print_case_pressure_progress(
    stage: str,
    current: int,
    total: int,
    case_meta: Dict,
    pressure,
    first_step: Optional[str],
) -> None:
    first_step_text = STEP_DISPLAY_NAME.get(first_step or "", first_step or "all_finished")
    print(
        f"{stage}: {_render_progress_bar(current, total)} "
        f"first:{first_step_text:<18} "
        f"formula:{case_meta['formula']:<12} "
        f"pressure:{_pressure_key(pressure):<4} "
        f"case:{os.path.join(case_meta['case_display_rel'], _pressure_key(pressure))}"
    )


def _collect_case_pressure_items(cases: Dict[str, Dict]) -> List[Tuple[Dict, object]]:
    items: List[Tuple[Dict, object]] = []
    for case_meta in cases.values():
        for pressure in case_meta["pressures"]:
            items.append((case_meta, pressure))
    return items


def _print_submit_unit_progress(script_info: Dict) -> None:
    for unit in script_info["units"]:
        print(
            f"  unit:first:{STEP_DISPLAY_NAME.get(unit['first_step'], unit['first_step']):<18} "
            f"formula:{unit['formula']:<12} "
            f"pressure:{unit['pressure_key']:<4} "
            f"case:{unit['case_rel']}"
        )


def run_case_pressure(output_dir: str, case_meta: Dict, pressure, refresh_before: bool = True) -> None:
    if refresh_before:
        refresh_status(output_dir)
    pressure_key = _pressure_key(pressure)
    case_status = load_case_status(case_meta["case_dir"])
    step_order = get_case_step_order(case_meta["case_dir"])
    use_mattersim = case_uses_mattersim(case_meta["case_dir"])
    first_step = _first_unfinished_step(case_status, pressure_key, step_order)
    if first_step is None:
        _print_progress("skip", "finished", case_meta, pressure, step_order, step_order[-1], reason="all_finished")
        return
    first_state = case_status["pressures"][pressure_key][first_step]
    if first_state == "error":
        _print_progress("break", "error", case_meta, pressure, step_order, first_step, reason="error")
        return

    commands = _run_commands(case_meta["case_dir"], case_meta["structure_filename"], pressure, use_mattersim)
    started = False
    for step in step_order:
        if step == first_step:
            started = True
        if not started:
            continue
        _print_progress("run", "unstarted", case_meta, pressure, step_order, step)
        try:
            run_cli_command(commands[step], _step_cwd(case_meta["case_dir"], pressure, step, use_mattersim))
            refresh_status(output_dir)
            case_status = load_case_status(case_meta["case_dir"])
            step_state = case_status["pressures"][pressure_key][step]
            if step_state != "finished":
                _print_progress("break", step_state, case_meta, pressure, step_order, step, reason="post_refresh")
                break
        except Exception:
            refresh_status(output_dir)
            _print_progress("break", "error", case_meta, pressure, step_order, step, reason="exception")
            raise


def generate_opt_only(case_meta: Dict, pressure) -> None:
    use_mattersim = case_uses_mattersim(case_meta["case_dir"])
    setup_commands = _setup_commands(case_meta["structure_filename"], pressure)
    first_step = "mattersim_opt" if use_mattersim else "vasp_opt"
    run_cli_command(setup_commands[first_step], case_meta["case_dir"])


def _load_toml_file(path: str) -> Dict:
    return load_config_file(path)


def _load_submit_groups_from_config(config_file: str) -> int:
    try:
        config = _load_toml_file(config_file)
    except Exception:
        return 1
    submit_groups = config.get("substitutionflow", {}).get("submit_groups", 1)
    try:
        submit_groups = int(submit_groups)
    except Exception as exc:
        raise ValueError("[substitutionflow].submit_groups 必须是整数。") from exc
    if submit_groups < 1:
        raise ValueError("[substitutionflow].submit_groups 必须大于等于 1。")
    return submit_groups


def _load_submit_groups_from_output_dir(output_dir: str) -> int:
    cases = discover_cases(output_dir)
    first_case = next(iter(cases.values()), None)
    if not first_case:
        return 1
    return _load_submit_groups_from_config(os.path.join(first_case["case_dir"], "input.toml"))


def _load_slurm_header(output_dir: str) -> str:
    cases = discover_cases(output_dir)
    first_case = next(iter(cases.values()), None)
    if not first_case:
        return ""
    case_input = os.path.join(first_case["case_dir"], "input.toml")
    try:
        return _load_toml_file(case_input).get("slurm", {}).get("header", "")
    except Exception:
        return ""


def _collect_configured_units(output_dir: str) -> List[Dict]:
    cases = discover_cases(output_dir)
    units: List[Dict] = []
    for case_meta in cases.values():
        structure_filename = _infer_structure_filename(case_meta["case_dir"])
        step_order = get_case_step_order(case_meta["case_dir"])
        use_mattersim = case_uses_mattersim(case_meta["case_dir"])
        for pressure in case_meta["pressures"]:
            step_commands = _run_commands_for_mode(
                case_meta["case_dir"],
                structure_filename,
                pressure,
                use_mattersim,
                submit_mode=True,
            )
            commands: List[str] = []
            step_names: List[str] = []
            for step in step_order:
                step_names.append(step)
                commands.append(f'cd "{_step_cwd(case_meta["case_dir"], pressure, step, use_mattersim)}"')
                commands.append(shlex.join(step_commands[step]))
            units.append(
                {
                    "case_dir": case_meta["case_dir"],
                    "case_rel": case_meta["case_rel"],
                    "formula": case_meta["formula"],
                    "pressure": pressure,
                    "pressure_key": _pressure_key(pressure),
                    "structure_filename": structure_filename,
                    "first_step": step_order[0],
                    "steps": step_names,
                    "commands": commands,
                }
            )
    return units


def _collect_pending_units(output_dir: str, refresh_before: bool = True) -> List[Dict]:
    if refresh_before:
        refresh_status(output_dir)
    cases = discover_cases(output_dir)
    units: List[Dict] = []
    for case_meta in cases.values():
        structure_filename = _infer_structure_filename(case_meta["case_dir"])
        case_status = load_case_status(case_meta["case_dir"])
        step_order = get_case_step_order(case_meta["case_dir"])
        use_mattersim = case_uses_mattersim(case_meta["case_dir"])
        for pressure in case_meta["pressures"]:
            pressure_key = _pressure_key(pressure)
            first_step = _first_unfinished_step(case_status, pressure_key, step_order)
            if first_step is None:
                continue
            commands: List[str] = []
            started = False
            step_commands = _run_commands_for_mode(
                case_meta["case_dir"],
                structure_filename,
                pressure,
                use_mattersim,
                submit_mode=True,
            )
            step_names: List[str] = []
            for step in step_order:
                if step == first_step:
                    started = True
                if not started:
                    continue
                step_names.append(step)
                commands.append(f'cd "{_step_cwd(case_meta["case_dir"], pressure, step, use_mattersim)}"')
                commands.append(shlex.join(step_commands[step]))
            units.append(
                {
                    "case_dir": case_meta["case_dir"],
                    "case_rel": case_meta["case_rel"],
                    "formula": case_meta["formula"],
                    "pressure": pressure,
                    "pressure_key": pressure_key,
                    "structure_filename": structure_filename,
                    "first_step": first_step,
                    "steps": step_names,
                    "commands": commands,
                }
            )
    return units


def _split_pending_units(units: List[Dict], submit_groups: int) -> List[List[Dict]]:
    if not units:
        return []
    actual_groups = min(len(units), submit_groups)
    base_size = len(units) // actual_groups
    remainder = len(units) % actual_groups
    groups: List[List[Dict]] = []
    start = 0
    for index in range(actual_groups):
        size = base_size + (1 if index < remainder else 0)
        groups.append(units[start:start + size])
        start += size
    return groups


def _build_submit_scripts_from_units(output_dir: str, submit_groups: int, units: List[Dict]) -> Dict:
    header = _load_slurm_header(output_dir)
    groups = _split_pending_units(units, submit_groups)
    script_infos: List[Dict] = []
    for index, group in enumerate(groups, start=1):
        script_name = f"submit_group_{index:02d}.sh"
        script_path = os.path.join(output_dir, script_name)
        commands: List[str] = []
        for unit in group:
            commands.extend(unit["commands"])
        write_job_script(script_path, header, commands)
        script_infos.append(
            {
                "script_name": script_name,
                "script_path": script_path,
                "unit_count": len(group),
                "units": group,
            }
        )
    return {
        "total_units": len(units),
        "configured_groups": submit_groups,
        "actual_groups": len(script_infos),
        "scripts": script_infos,
    }


def build_submit_scripts(output_dir: str, submit_groups: int, refresh_before: bool = True) -> Dict:
    return _build_submit_scripts_from_units(
        output_dir,
        submit_groups,
        _collect_pending_units(output_dir, refresh_before=refresh_before),
    )


def build_configured_submit_scripts(output_dir: str, submit_groups: int) -> Dict:
    return _build_submit_scripts_from_units(output_dir, submit_groups, _collect_configured_units(output_dir))


def _format_submit_summary(script_bundle: Dict, total_label: str) -> str:
    lines = [
        f"{total_label}: {script_bundle['total_units']}",
        f"计划分组数: {script_bundle['configured_groups']}",
        f"实际生成组数: {script_bundle['actual_groups']}",
    ]
    for script_info in script_bundle["scripts"]:
        lines.append(f"{script_info['script_name']}: {script_info['unit_count']} 个完整压强点单元")
    return "\n".join(lines)


def _submit_group_scripts(output_dir: str, script_bundle: Dict) -> None:
    total_scripts = len(script_bundle["scripts"])
    for index, script_info in enumerate(script_bundle["scripts"], start=1):
        print(f"提交进度: {_render_progress_bar(index, total_scripts)} group:{script_info['script_name']}")
        print(f"提交 {script_info['script_name']}，包含 {script_info['unit_count']} 个完整压强点单元")
        _print_submit_unit_progress(script_info)
        submit_script(output_dir, script_info["script_name"])


def run_workflow(config_file: str, output_dir: str, run_calc: bool = False, submit: bool = False) -> str:
    result = ensure_workflow_tree(config_file, output_dir)
    cases = discover_cases(output_dir)
    for case_meta in cases.values():
        case_meta["structure_filename"] = result["structure_filename"]
        for pressure in case_meta["pressures"]:
            generate_opt_only(case_meta, pressure)

    if run_calc:
        case_pressure_items = _collect_case_pressure_items(cases)
        total_items = len(case_pressure_items)
        for index, (case_meta, pressure) in enumerate(case_pressure_items, start=1):
            pressure_key = _pressure_key(pressure)
            case_status = load_case_status(case_meta["case_dir"])
            step_order = get_case_step_order(case_meta["case_dir"])
            first_step = _first_unfinished_step(case_status, pressure_key, step_order)
            _print_case_pressure_progress("运行进度", index, total_items, case_meta, pressure, first_step)
            try:
                refresh_status(output_dir)
                case_status = load_case_status(case_meta["case_dir"])
                step_order = get_case_step_order(case_meta["case_dir"])
                use_mattersim = case_uses_mattersim(case_meta["case_dir"])
                commands = _run_commands(case_meta["case_dir"], case_meta["structure_filename"], pressure, use_mattersim)
                for step in step_order:
                    current_state = case_status["pressures"][pressure_key][step]
                    if current_state == "finished":
                        _print_progress("skip", "finished", case_meta, pressure, step_order, step, reason="finished")
                        continue
                    if current_state == "error":
                        _print_progress("break", "error", case_meta, pressure, step_order, step, reason="error")
                        break
                    _print_progress("run", "unstarted", case_meta, pressure, step_order, step)
                    run_cli_command(commands[step], _step_cwd(case_meta["case_dir"], pressure, step, use_mattersim))
                    refresh_status(output_dir)
                    case_status = load_case_status(case_meta["case_dir"])
                    step_state = case_status["pressures"][pressure_key][step]
                    if step_state != "finished":
                        _print_progress("break", step_state, case_meta, pressure, step_order, step, reason="post_refresh")
                        break
            except Exception:
                refresh_status(output_dir)
                continue
        return summarize_status(refresh_status(output_dir))

    submit_groups = _load_submit_groups_from_config(config_file)
    script_bundle = build_configured_submit_scripts(output_dir, submit_groups)
    if submit:
        _submit_group_scripts(output_dir, script_bundle)
    return _format_submit_summary(script_bundle, "总生成单元数")


def _repair_qe_phonon_inputs(case_dir: str, pressure_key: str) -> None:
    pressure_dir = os.path.join(case_dir, pressure_key)
    qe_workdir = os.path.join(pressure_dir, get_case_phonon_workdir(case_dir))
    if not os.path.isdir(qe_workdir):
        return

    contcar_path = os.path.join(pressure_dir, "vasp_opt", "CONTCAR")
    if os.path.exists(contcar_path):
        shutil.copyfile(contcar_path, os.path.join(qe_workdir, "POSCAR"))


def _sync_qe_phonon_poscars(output_dir: str) -> int:
    cases = discover_cases(output_dir)
    repaired_pressures = 0
    for case_meta in cases.values():
        for pressure in case_meta["pressures"]:
            pressure_key = _pressure_key(pressure)
            pressure_dir = os.path.join(case_meta["case_dir"], pressure_key)
            qe_workdir = os.path.join(pressure_dir, get_case_phonon_workdir(case_meta["case_dir"]))
            contcar_path = os.path.join(pressure_dir, "vasp_opt", "CONTCAR")
            if not os.path.isdir(qe_workdir) or not os.path.exists(contcar_path):
                continue
            _repair_qe_phonon_inputs(case_meta["case_dir"], pressure_key)
            repaired_pressures += 1
    return repaired_pressures


def rewrite_case_input_configs(output_dir: str, config_file: str) -> int:
    return _rewrite_case_input_configs(output_dir, config_file, compact_progress=False)


def _rewrite_case_input_configs(output_dir: str, config_file: str, compact_progress: bool) -> int:
    root_config = load_substitutionflow_config(config_file)
    cases = discover_cases(output_dir)
    rewritten = 0
    total_cases = len(cases)
    for index, case_meta in enumerate(cases.values(), start=1):
        if compact_progress:
            _print_compact_stage_progress("重写配置", index, total_cases, case_meta["case_display_rel"])
        else:
            _print_case_stage_progress("重写配置", index, total_cases, case_meta)
        write_case_input_config(os.path.join(case_meta["case_dir"], "input.toml"), root_config)
        rewritten += 1
    return rewritten


def _rewrite_qe_phonon_workflow_status_errors(case_dir: str, pressure_key: str) -> int:
    pressure_dir = os.path.join(case_dir, pressure_key)
    qe_workdir = os.path.join(pressure_dir, get_case_phonon_workdir(case_dir))
    workflow_status_path = os.path.join(qe_workdir, "workflow_status.json")
    if not os.path.exists(workflow_status_path):
        return 0

    try:
        with open(workflow_status_path, "r", encoding="utf-8") as f:
            workflow_status = json.load(f)
    except Exception:
        return 0

    rewritten = 0

    def rewrite_payload(payload):
        nonlocal rewritten
        if isinstance(payload, dict):
            for key, value in payload.items():
                if isinstance(value, str):
                    if value == "error":
                        payload[key] = "unstarted"
                        rewritten += 1
                else:
                    rewrite_payload(value)
        elif isinstance(payload, list):
            for item in payload:
                rewrite_payload(item)

    rewrite_payload(workflow_status)
    if rewritten > 0:
        with open(workflow_status_path, "w", encoding="utf-8") as f:
            json.dump(workflow_status, f, indent=2, ensure_ascii=False)
    return rewritten


def _record_rewrite_result(
    grouped_results: "OrderedDict[Tuple[str, str], Dict[str, object]]",
    case_meta: Dict,
    pressure_key: str,
    step: str,
) -> None:
    key = (case_meta["case_display_rel"], pressure_key)
    entry = grouped_results.setdefault(
        key,
        {
            "formula": case_meta["formula"],
            "case_display_rel": case_meta["case_display_rel"],
            "pressure_key": pressure_key,
            "steps": [],
        },
    )
    steps = entry["steps"]
    if isinstance(steps, list) and step not in steps:
        steps.append(step)


def _format_rewrite_results_table(grouped_results: "OrderedDict[Tuple[str, str], Dict[str, object]]") -> str:
    if not grouped_results:
        return ""
    formula_width = 20
    pressure_width = 10
    steps_width = 25
    lines = [
        "重写结果:",
        f"{'FORMULA':<{formula_width}} {'PRESSURE':<{pressure_width}} {'STEPS':<{steps_width}} CASE",
    ]
    for entry in grouped_results.values():
        steps = ",".join(entry["steps"]) if isinstance(entry["steps"], list) else ""
        lines.append(
            f"{entry['formula']:<{formula_width}} "
            f"{entry['pressure_key']:<{pressure_width}} "
            f"{steps:<{steps_width}} "
            f"{entry['case_display_rel']}"
        )
    return "\n".join(lines)


def _collect_pre_refresh_error_steps(output_dir: str) -> Dict[Tuple[str, str], set]:
    cases = discover_cases(output_dir)
    error_steps: Dict[Tuple[str, str], set] = {}
    for case_meta in cases.values():
        case_status = load_case_status(case_meta["case_dir"])
        for pressure_key, pressure_status in case_status["pressures"].items():
            key = (case_meta["case_display_rel"], pressure_key)
            error_steps[key] = {
                step_name
                for step_name, step_state in pressure_status.items()
                if step_state == "error"
            }
    return error_steps


def _prepare_continue_workspace(output_dir: str, config_file: str) -> Tuple[Dict, Dict, int, Dict]:
    rewrite_case_input_configs(output_dir, config_file)
    summary = refresh_status(output_dir, show_progress=True)
    repaired_pressures = _sync_qe_phonon_poscars(output_dir)
    submit_groups = _load_submit_groups_from_output_dir(output_dir)
    script_bundle = build_submit_scripts(output_dir, submit_groups, refresh_before=False)
    return summary, script_bundle, repaired_pressures, discover_cases(output_dir)


def rewrite_workflow_states(output_dir: str, config_file: Optional[str] = None) -> str:
    if not config_file:
        raise ValueError("rewrite_workflow_states() 需要显式提供 config_file。")
    rewritten_case_configs = _rewrite_case_input_configs(output_dir, config_file, compact_progress=True)
    pre_refresh_error_steps = _collect_pre_refresh_error_steps(output_dir)
    summary = refresh_status(output_dir, show_progress=True, progress_style="compact")
    repaired_pressures = _sync_qe_phonon_poscars(output_dir)
    cases = discover_cases(output_dir)
    rewritten_count = 0
    rewritten_qe_count = 0
    grouped_results: "OrderedDict[Tuple[str, str], Dict[str, object]]" = OrderedDict()

    total_cases = len(cases)
    for index, case_meta in enumerate(cases.values(), start=1):
        _print_compact_stage_progress("重写状态", index, total_cases, case_meta["case_display_rel"])
        case_status = load_case_status(case_meta["case_dir"])
        step_order = get_case_step_order(case_meta["case_dir"])
        rewritten_pressures = set()
        for pressure in case_meta["pressures"]:
            pressure_key = _pressure_key(pressure)
            pressure_status = case_status["pressures"][pressure_key]
            for step in step_order:
                if pressure_status.get(step) != "error":
                    continue
                pressure_status[step] = "unstarted"
                rewritten_count += 1
                rewritten_pressures.add(pressure_key)
                key = (case_meta["case_display_rel"], pressure_key)
                if step in pre_refresh_error_steps.get(key, set()):
                    _record_rewrite_result(grouped_results, case_meta, pressure_key, step)
        for pressure_key in sorted(rewritten_pressures, key=lambda value: list(case_status["pressures"]).index(value)):
            rewritten_qe_count += _rewrite_qe_phonon_workflow_status_errors(case_meta["case_dir"], pressure_key)
        save_case_status(case_meta["case_dir"], case_status)

    summary = create_initial_status(output_dir)
    submit_groups = _load_submit_groups_from_output_dir(output_dir)
    script_bundle = build_submit_scripts(output_dir, submit_groups, refresh_before=False)
    lines = [
        summarize_status(summary),
        _format_submit_summary(script_bundle, "总未完成单元数"),
        f"重写步骤数: {rewritten_count}",
        f"重写 qe_phonon 子步骤数: {rewritten_qe_count}",
        f"同步 qe_phonon POSCAR 压强点数: {repaired_pressures}",
        f"重写 case 配置数: {rewritten_case_configs}",
    ]
    rewrite_results_table = _format_rewrite_results_table(grouped_results)
    if rewrite_results_table:
        lines.append(rewrite_results_table)
    return "\n".join(lines)


def continue_workflow(output_dir: str, run_calc: bool = False, submit: bool = False, config_file: Optional[str] = None) -> str:
    if not config_file:
        raise ValueError("continue_workflow() 需要显式提供 config_file。")
    if not run_calc and not submit:
        summary, script_bundle, repaired_pressures, _cases = _prepare_continue_workspace(output_dir, config_file)
        return "\n".join(
            [
                summarize_status(summary),
                _format_submit_summary(script_bundle, "总未完成单元数"),
                f"同步 qe_phonon POSCAR 压强点数: {repaired_pressures}",
            ]
        )

    _summary, script_bundle, _repaired_pressures, cases = _prepare_continue_workspace(output_dir, config_file)
    for case_meta in cases.values():
        case_meta["structure_filename"] = _infer_structure_filename(case_meta["case_dir"])

    if run_calc:
        case_pressure_items = _collect_case_pressure_items(cases)
        total_items = len(case_pressure_items)
        for index, (case_meta, pressure) in enumerate(case_pressure_items, start=1):
            pressure_key = _pressure_key(pressure)
            case_status = load_case_status(case_meta["case_dir"])
            step_order = get_case_step_order(case_meta["case_dir"])
            first_step = _first_unfinished_step(case_status, pressure_key, step_order)
            _print_case_pressure_progress("运行进度", index, total_items, case_meta, pressure, first_step)
            try:
                run_case_pressure(output_dir, case_meta, pressure, refresh_before=False)
            except Exception:
                continue
        return summarize_status(refresh_status(output_dir))

    _submit_group_scripts(output_dir, script_bundle)
    return _format_submit_summary(script_bundle, "总未完成单元数")
