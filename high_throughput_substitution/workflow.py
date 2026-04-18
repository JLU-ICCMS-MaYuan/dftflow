#!/usr/bin/env python3
import os
import subprocess
from typing import Dict, List, Optional

from public.job import submit_script, write_job_script

from high_throughput_substitution.status import (
    STEP_ORDER,
    create_initial_status,
    discover_cases,
    load_case_status,
    refresh_status,
    summarize_status,
)
from high_throughput_substitution.structure_substitution import generate_cases


def run_cli_command(command: List[str], cwd: str) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def _pressure_key(pressure) -> str:
    return f"{pressure:g}" if isinstance(pressure, (int, float)) else str(pressure)


def ensure_workflow_tree(input_path: str, config_file: str, output_dir: str) -> Dict:
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


def _setup_commands(structure_filename: str, pressure) -> Dict[str, List[str]]:
    pressure_key = _pressure_key(pressure)
    return {
        "vasp_opt": ["vasp_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key],
    }


def _run_commands(case_dir: str, structure_filename: str, pressure) -> Dict[str, List[str]]:
    pressure_key = _pressure_key(pressure)
    return {
        "vasp_opt": ["vasp_opt", "-i", structure_filename, "-c", "input.toml", "-p", pressure_key, "-r"],
        "vasp_scf": ["vasp_scf", "-i", "vasp_opt/CONTCAR", "-c", "../input.toml", "-r"],
        "vasp_eledos": ["vasp_eledos", "-i", "vasp_scf/POSCAR", "-c", "../input.toml", "-r"],
        "vasp_process_eledos": _build_process_eledos_command(case_dir, structure_filename),
    }


def _step_cwd(case_dir: str, pressure, step: str) -> str:
    if step == "vasp_opt":
        return case_dir
    return os.path.join(case_dir, _pressure_key(pressure))


def _first_unfinished_step(case_status: Dict, pressure_key: str) -> Optional[str]:
    pressure_status = case_status["pressures"][pressure_key]
    for step in STEP_ORDER:
        if pressure_status[step] != "finished":
            return step
    return None


def run_case_pressure(output_dir: str, case_meta: Dict, pressure) -> None:
    refresh_status(output_dir)
    pressure_key = _pressure_key(pressure)
    case_status = load_case_status(case_meta["case_dir"])
    first_step = _first_unfinished_step(case_status, pressure_key)
    if first_step is None:
        return
    if case_status["pressures"][pressure_key][first_step] == "running":
        return

    commands = _run_commands(case_meta["case_dir"], case_meta["structure_filename"], pressure)
    started = False
    for step in STEP_ORDER:
        if step == first_step:
            started = True
        if not started:
            continue
        try:
            run_cli_command(commands[step], _step_cwd(case_meta["case_dir"], pressure, step))
            refresh_status(output_dir)
        except Exception:
            refresh_status(output_dir)
            raise


def generate_opt_only(case_meta: Dict, pressure) -> None:
    command = _setup_commands(case_meta["structure_filename"], pressure)["vasp_opt"]
    run_cli_command(command, case_meta["case_dir"])


def build_submit_script(output_dir: str, script_name: str = "submit_high_throughput_substitution.sh") -> str:
    refresh_status(output_dir)
    cases = discover_cases(output_dir)
    first_case = next(iter(cases.values()), None)
    header = ""
    if first_case:
        case_input = os.path.join(first_case["case_dir"], "input.toml")
        try:
            import tomllib
            with open(case_input, "rb") as f:
                header = tomllib.load(f).get("slurm", {}).get("header", "")
        except Exception:
            header = ""

    commands: List[str] = []
    for case_meta in cases.values():
        structure_filename = _infer_structure_filename(case_meta["case_dir"])
        case_status = load_case_status(case_meta["case_dir"])
        for pressure in case_meta["pressures"]:
            pressure_key = _pressure_key(pressure)
            first_step = _first_unfinished_step(case_status, pressure_key)
            if first_step is None:
                continue
            if case_status["pressures"][pressure_key][first_step] == "running":
                continue
            started = False
            step_commands = _run_commands(case_meta["case_dir"], structure_filename, pressure)
            for step in STEP_ORDER:
                if step == first_step:
                    started = True
                if not started:
                    continue
                commands.append(f'cd "{_step_cwd(case_meta["case_dir"], pressure, step)}"')
                commands.append(" ".join(step_commands[step]))

    script_path = os.path.join(output_dir, script_name)
    write_job_script(script_path, header, commands)
    return script_path


def run_workflow(input_path: str, config_file: str, output_dir: str, run_calc: bool = False, submit: bool = False) -> str:
    result = ensure_workflow_tree(input_path, config_file, output_dir)
    cases = discover_cases(output_dir)
    for case_meta in cases.values():
        case_meta["structure_filename"] = result["structure_filename"]
        for pressure in case_meta["pressures"]:
            generate_opt_only(case_meta, pressure)
    refresh_status(output_dir)

    if run_calc:
        for case_meta in cases.values():
            for pressure in case_meta["pressures"]:
                try:
                    run_case_pressure(output_dir, case_meta, pressure)
                except Exception:
                    continue
        return summarize_status(refresh_status(output_dir))

    script_path = build_submit_script(output_dir)
    if submit:
        submit_script(output_dir, os.path.basename(script_path))
    return script_path


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

    script_path = build_submit_script(output_dir)
    if submit:
        submit_script(output_dir, os.path.basename(script_path))
    return summarize_status(refresh_status(output_dir))
