#!/usr/bin/env python3
import os
import subprocess
from typing import Dict, List, Optional, Tuple

from public.config_templates import load_config_file
from public.job import submit_script, write_job_script

from substitutionflow.status import (
    case_uses_mattersim,
    create_initial_status,
    discover_cases,
    get_case_phonon_workflow,
    get_case_step_order,
    load_case_status,
    refresh_status,
    summarize_status,
)
from substitutionflow.structure_substitution import generate_cases, resolve_input_structure_path


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
    return [
        get_case_phonon_workflow(case_dir),
        "-i",
        "vasp_opt/CONTCAR",
        "-c",
        "../input.toml",
        "-r",
    ]


def _setup_commands(structure_filename: str, pressure) -> Dict[str, List[str]]:
    pressure_key = _pressure_key(pressure)
    return {
        "mattersim_opt": ["mattersim_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key],
        "vasp_opt": ["vasp_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key],
    }


def _run_commands(case_dir: str, structure_filename: str, pressure, use_mattersim: bool) -> Dict[str, List[str]]:
    pressure_key = _pressure_key(pressure)
    vasp_opt_input = os.path.join(pressure_key, "mattersim_opt", "CONTCAR") if use_mattersim else structure_filename
    return {
        "mattersim_opt": ["mattersim_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key, "-r"],
        "vasp_opt": ["vasp_opt", "-i", vasp_opt_input, "-c", "input.toml", "-p", pressure_key, "-r"],
        "vasp_scf": ["vasp_scf", "-i", "vasp_opt/CONTCAR", "-c", "../input.toml", "-r"],
        "vasp_eledos": ["vasp_eledos", "-i", "vasp_scf/POSCAR", "-c", "../input.toml", "-r"],
        "vasp_process_eledos": _build_process_eledos_command(case_dir, structure_filename),
        "qe_phonon": _build_qe_phonon_command(case_dir),
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


def run_case_pressure(output_dir: str, case_meta: Dict, pressure) -> None:
    refresh_status(output_dir)
    pressure_key = _pressure_key(pressure)
    case_status = load_case_status(case_meta["case_dir"])
    step_order = get_case_step_order(case_meta["case_dir"])
    use_mattersim = case_uses_mattersim(case_meta["case_dir"])
    first_step = _first_unfinished_step(case_status, pressure_key, step_order)
    if first_step is None:
        return
    if case_status["pressures"][pressure_key][first_step] == "running":
        return

    commands = _run_commands(case_meta["case_dir"], case_meta["structure_filename"], pressure, use_mattersim)
    started = False
    for step in step_order:
        if step == first_step:
            started = True
        if not started:
            continue
        try:
            run_cli_command(commands[step], _step_cwd(case_meta["case_dir"], pressure, step, use_mattersim))
            updated_status = refresh_status(output_dir)
            case_status = load_case_status(case_meta["case_dir"])
            step_state = case_status["pressures"][pressure_key][step]
            if step_state != "finished":
                break
        except Exception:
            refresh_status(output_dir)
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
            step_commands = _run_commands(case_meta["case_dir"], structure_filename, pressure, use_mattersim)
            commands: List[str] = []
            step_names: List[str] = []
            for step in step_order:
                step_names.append(step)
                commands.append(f'cd "{_step_cwd(case_meta["case_dir"], pressure, step, use_mattersim)}"')
                commands.append(" ".join(step_commands[step]))
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


def _collect_pending_units(output_dir: str) -> List[Dict]:
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
            if case_status["pressures"][pressure_key][first_step] == "running":
                continue
            commands: List[str] = []
            started = False
            step_commands = _run_commands(case_meta["case_dir"], structure_filename, pressure, use_mattersim)
            step_names: List[str] = []
            for step in step_order:
                if step == first_step:
                    started = True
                if not started:
                    continue
                step_names.append(step)
                commands.append(f'cd "{_step_cwd(case_meta["case_dir"], pressure, step, use_mattersim)}"')
                commands.append(" ".join(step_commands[step]))
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


def build_submit_scripts(output_dir: str, submit_groups: int) -> Dict:
    return _build_submit_scripts_from_units(output_dir, submit_groups, _collect_pending_units(output_dir))


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
    for script_info in script_bundle["scripts"]:
        print(f"提交 {script_info['script_name']}，包含 {script_info['unit_count']} 个完整压强点单元")
        submit_script(output_dir, script_info["script_name"])


def run_workflow(config_file: str, output_dir: str, run_calc: bool = False, submit: bool = False) -> str:
    result = ensure_workflow_tree(config_file, output_dir)
    cases = discover_cases(output_dir)
    for case_meta in cases.values():
        case_meta["structure_filename"] = result["structure_filename"]
        for pressure in case_meta["pressures"]:
            generate_opt_only(case_meta, pressure)

    if run_calc:
        for case_meta in cases.values():
            for pressure in case_meta["pressures"]:
                try:
                    use_mattersim = case_uses_mattersim(case_meta["case_dir"])
                    commands = _run_commands(case_meta["case_dir"], case_meta["structure_filename"], pressure, use_mattersim)
                    step_order = get_case_step_order(case_meta["case_dir"])
                    for step in step_order:
                        run_cli_command(commands[step], _step_cwd(case_meta["case_dir"], pressure, step, use_mattersim))
                except Exception:
                    continue
        return summarize_status(refresh_status(output_dir))

    submit_groups = _load_submit_groups_from_config(config_file)
    script_bundle = build_configured_submit_scripts(output_dir, submit_groups)
    if submit:
        _submit_group_scripts(output_dir, script_bundle)
    return _format_submit_summary(script_bundle, "总生成单元数")


def continue_workflow(output_dir: str, run_calc: bool = False, submit: bool = False) -> str:
    refresh_status(output_dir)
    cases = discover_cases(output_dir)
    for case_meta in cases.values():
        case_meta["structure_filename"] = _infer_structure_filename(case_meta["case_dir"])

    if run_calc:
        for case_meta in cases.values():
            for pressure in case_meta["pressures"]:
                try:
                    run_case_pressure(output_dir, case_meta, pressure)
                except Exception:
                    continue
        return summarize_status(refresh_status(output_dir))

    submit_groups = _load_submit_groups_from_output_dir(output_dir)
    script_bundle = build_submit_scripts(output_dir, submit_groups)
    if submit:
        _submit_group_scripts(output_dir, script_bundle)
    return _format_submit_summary(script_bundle, "总未完成单元数")
