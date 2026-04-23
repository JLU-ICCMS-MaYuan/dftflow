#!/usr/bin/env python3
import os
import shlex
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from typing import Dict, Optional

from qeflow.qe_check.checks import check_ph, check_q2r, check_scf, summarize_task_states
from qeflow.qe_input.builders import (
    build_qe_commands,
    derive_filenames,
    parse_poscar,
    resolve_prefix,
    write_matdyn_phband_input,
    write_matdyn_phdos_input,
    write_ph_input,
    write_q2r_input,
    write_root_run_script,
    write_scf_input,
    write_split_ph_input_assign_coords,
    write_split_ph_input_assign_index,
    write_split_run_script,
)
from qeflow.qe_input.fileio import dump_json, load_json, resolve_workspace
from qeflow.phonon.qgrid import generate_irreducible_qpoints, write_dyn0
from qeflow.phonon.status import refresh_workflow_status
from public.job import submit_script


def _run_shell(command: str, cwd: str) -> None:
    subprocess.run(command, shell=True, check=True, cwd=cwd)


def _submit_or_run(run_calc: bool, submit_job: bool, cwd: str, script_name: str) -> bool:
    if run_calc:
        _run_shell(f"bash {shlex.quote(script_name)}", cwd)
        return True
    if submit_job:
        submit_script(cwd, script_name)
        return True
    return False


def _format_state_line(label: str, state: str) -> str:
    return f"{label}: {state}"


def print_status(prefix: str, workflow: Dict[str, object]) -> None:
    print(_format_state_line("scf", str(workflow["scf"])))
    ph_payload = workflow["ph"]
    if isinstance(ph_payload, dict):
        print(_format_state_line("ph", str(ph_payload["state"])))
        print(_format_state_line("dyn0", str(ph_payload["dyn0"])))
    print(_format_state_line("q2r", str(workflow["q2r"])))
    print(_format_state_line("matdyn_phband", str(workflow["matdyn_phband"])))
    print(_format_state_line("matdyn_phdos", str(workflow["matdyn_phdos"])))
    tasks = workflow.get("tasks", {})
    if isinstance(tasks, dict) and tasks:
        for q_name in sorted(tasks):
            task_payload = tasks[q_name]
            if isinstance(task_payload, dict):
                print(_format_state_line(q_name, str(task_payload.get("state", "unstarted"))))
    print(f"prefix: {prefix}")


def run_ph_to_dyn0(workspace: str, ph_command: str, dyn0_path: str, output_path: str) -> None:
    command = f"{ph_command} < ph.in > ph.out"
    process = subprocess.Popen(
        command,
        shell=True,
        cwd=workspace,
        preexec_fn=os.setsid,
    )
    try:
        while True:
            if os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0:
                os.killpg(os.getpgid(process.pid), signal.SIGTERM)
                process.wait(timeout=10)
                return
            if os.path.exists(output_path):
                with open(output_path, "r", encoding="utf-8", errors="ignore") as handle:
                    content = handle.read().lower()
                if "%%%%%%%%%%%%%%" in content or "error in routine" in content:
                    process.wait(timeout=10)
                    raise RuntimeError("ph.x 在生成 dyn0 前已报错，终止 dyn0 模式。")
            if process.poll() is not None:
                if os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0:
                    return
                raise RuntimeError("ph.x 已退出，但未生成有效的 dyn0 文件。")
            time.sleep(0.5)
    finally:
        if process.poll() is None:
            os.killpg(os.getpgid(process.pid), signal.SIGTERM)
            process.wait(timeout=10)


def _resolve_xml_path(workspace: str, config: Dict[str, object], prefix: str) -> str:
    outdir = str(config["qe_phonon"]["scf"].get("outdir", "./out")).strip("'").strip('"')
    return os.path.join(workspace, outdir, f"{prefix}.save", "data-file-schema.xml")


def materialize_split_directories(
    workspace: str,
    config: Dict[str, object],
    prefix: str,
    mode: str,
) -> int:
    xml_path = _resolve_xml_path(workspace, config, prefix)
    if not os.path.exists(xml_path):
        raise FileNotFoundError(f"找不到不可约 q 点所需 XML: {xml_path}")
    qpoints = generate_irreducible_qpoints(
        xml_path,
        int(config["qe_phonon"]["ph"]["qpoints"][0]),
        int(config["qe_phonon"]["ph"]["qpoints"][1]),
        int(config["qe_phonon"]["ph"]["qpoints"][2]),
    )
    dyn0_path = os.path.join(workspace, derive_filenames(prefix)["dyn0"])
    write_dyn0(
        dyn0_path,
        int(config["qe_phonon"]["ph"]["qpoints"][0]),
        int(config["qe_phonon"]["ph"]["qpoints"][1]),
        int(config["qe_phonon"]["ph"]["qpoints"][2]),
        qpoints,
    )
    commands = build_qe_commands(config)
    slurm_header = str(config.get("slurm", {}).get("header", ""))
    struct_info = parse_poscar(os.path.join(workspace, "POSCAR"))
    q_map_payload = {"qpoints": []}
    for dir_index, qpoint in enumerate(qpoints):
        q_name = f"q_{dir_index:02d}"
        q_dir = os.path.join(workspace, q_name)
        os.makedirs(q_dir, exist_ok=True)
        write_scf_input(q_dir, config, struct_info, prefix)
        if mode == "split_assign_index":
            write_split_ph_input_assign_index(q_dir, config, prefix, int(qpoint["index"]))
        else:
            write_split_ph_input_assign_coords(q_dir, config, prefix, qpoint["q_crystal"])
        write_split_run_script(q_dir, slurm_header, commands["pw"], commands["ph"])
        q_map_payload["qpoints"].append(
            {
                "directory": q_name,
                "index": int(qpoint["index"]),
                "q_crystal": qpoint["q_crystal"],
                "q_cart": qpoint["q_cart"],
                "weight": qpoint["weight"],
            }
        )
    dump_json(os.path.join(workspace, "q_index_map.json"), q_map_payload)
    return len(q_map_payload["qpoints"])


@dataclass
class PhononStepContext:
    config: Dict[str, object]
    workspace: str
    prefix: str
    struct_info: Dict[str, object]
    commands: Dict[str, str]


def create_context(config_file: str, struct_file: Optional[str]) -> PhononStepContext:
    config, _root_dir, workspace = resolve_workspace(config_file, struct_file, prepare_inputs=True)
    struct_info = parse_poscar(os.path.join(workspace, "POSCAR"))
    prefix = resolve_prefix(config, struct_info)
    commands = build_qe_commands(config)
    return PhononStepContext(
        config=config,
        workspace=workspace,
        prefix=prefix,
        struct_info=struct_info,
        commands=commands,
    )


def prepare_scf(context: PhononStepContext) -> str:
    write_scf_input(context.workspace, context.config, context.struct_info, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_scf.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['pw']} < scf.in > scf.out"],
    )


def prepare_ph(context: PhononStepContext, dyn0_only: bool = False) -> str:
    write_ph_input(context.workspace, context.config, context.prefix, dyn0_only=dyn0_only)
    return write_root_run_script(
        context.workspace,
        "run_ph.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['ph']} < ph.in > ph.out"],
    )


def prepare_q2r(context: PhononStepContext) -> str:
    write_q2r_input(context.workspace, context.config, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_q2r.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['q2r']} < q2r.in > q2r.out"],
    )


def prepare_matdyn_phband(context: PhononStepContext) -> str:
    write_matdyn_phband_input(context.workspace, context.config, context.struct_info, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_matdyn_phband.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['matdyn']} < matdyn_phband.in > matdyn_phband.out"],
    )


def prepare_matdyn_phdos(context: PhononStepContext) -> str:
    write_matdyn_phdos_input(context.workspace, context.config, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_matdyn_phdos.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['matdyn']} < matdyn_phdos.in > matdyn_phdos.out"],
    )


def run_standalone_ph(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, dyn0_only: bool) -> int:
    context = create_context(config_file, struct_file)
    prepare_scf(context)
    script = prepare_ph(context, dyn0_only=dyn0_only)
    if dyn0_only and run_calc:
        run_ph_to_dyn0(
            context.workspace,
            context.commands["ph"],
            os.path.join(context.workspace, f"{context.prefix}.dyn0"),
            os.path.join(context.workspace, "ph.out"),
        )
    else:
        _submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_standalone_q2r(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool) -> int:
    context = create_context(config_file, struct_file)
    script = prepare_q2r(context)
    _submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_standalone_matdyn(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, target: str) -> int:
    context = create_context(config_file, struct_file)
    if target == "phband":
        script = prepare_matdyn_phband(context)
    else:
        script = prepare_matdyn_phdos(context)
    _submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_standalone_split(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, mode: str) -> int:
    context = create_context(config_file, struct_file)
    scf_state = check_scf(context.workspace)
    if scf_state != "finished":
        raise RuntimeError("split 模式依赖根目录已完成的 scf.out。")
    prepare_ph(context, dyn0_only=True)
    if run_calc:
        run_ph_to_dyn0(
            context.workspace,
            context.commands["ph"],
            os.path.join(context.workspace, f"{context.prefix}.dyn0"),
            os.path.join(context.workspace, "ph.out"),
        )
    elif submit_job:
        submit_script(context.workspace, "run_ph.sh")
        return 0
    count = materialize_split_directories(context.workspace, context.config, context.prefix, mode)
    if run_calc or submit_job:
        for index in range(count):
            q_name = f"q_{index:02d}"
            if run_calc:
                _run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
            else:
                submit_script(os.path.join(context.workspace, q_name), "run_split.sh")
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_phonon_workflow(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, mode: str) -> int:
    context = create_context(config_file, struct_file)
    prepare_scf(context)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["scf"] != "finished":
        _submit_or_run(run_calc, submit_job, context.workspace, "run_scf.sh")
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    if mode == "nosplit":
        prepare_ph(context, dyn0_only=False)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        ph_payload = workflow["ph"]
        ph_state = ph_payload["state"] if isinstance(ph_payload, dict) else ph_payload
        if ph_state != "finished":
            _submit_or_run(run_calc, submit_job, context.workspace, "run_ph.sh")
            print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
            return 0
        prepare_q2r(context)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if workflow["q2r"] != "finished":
            _submit_or_run(run_calc, submit_job, context.workspace, "run_q2r.sh")
            workflow = refresh_workflow_status(context.workspace, context.prefix)
            if submit_job or workflow["q2r"] != "finished":
                print_status(context.prefix, workflow)
                return 0
        prepare_matdyn_phband(context)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if workflow["matdyn_phband"] != "finished":
            _submit_or_run(run_calc, submit_job, context.workspace, "run_matdyn_phband.sh")
            workflow = refresh_workflow_status(context.workspace, context.prefix)
            if submit_job or workflow["matdyn_phband"] != "finished":
                print_status(context.prefix, workflow)
                return 0
        prepare_matdyn_phdos(context)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if workflow["matdyn_phdos"] != "finished":
            _submit_or_run(run_calc, submit_job, context.workspace, "run_matdyn_phdos.sh")
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    prepare_ph(context, dyn0_only=True)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    ph_payload = workflow["ph"]
    dyn0_state = ph_payload["dyn0"] if isinstance(ph_payload, dict) else "unstarted"
    if dyn0_state != "finished":
        if run_calc:
            run_ph_to_dyn0(
                context.workspace,
                context.commands["ph"],
                os.path.join(context.workspace, f"{context.prefix}.dyn0"),
                os.path.join(context.workspace, "ph.out"),
            )
        elif submit_job:
            submit_script(context.workspace, "run_ph.sh")
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    q_map = load_json(os.path.join(context.workspace, "q_index_map.json"))
    if not q_map:
        count = materialize_split_directories(context.workspace, context.config, context.prefix, mode)
        if run_calc or submit_job:
            for index in range(count):
                q_name = f"q_{index:02d}"
                if run_calc:
                    _run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
                else:
                    submit_script(os.path.join(context.workspace, q_name), "run_split.sh")
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    tasks = workflow.get("tasks", {})
    if isinstance(tasks, dict):
        task_summary = summarize_task_states(
            task["state"] for task in tasks.values() if isinstance(task, dict)
        )
        if task_summary != "finished":
            print_status(context.prefix, workflow)
            return 0

    prepare_q2r(context)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["q2r"] != "finished":
        _submit_or_run(run_calc, submit_job, context.workspace, "run_q2r.sh")
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if submit_job or workflow["q2r"] != "finished":
            print_status(context.prefix, workflow)
            return 0

    prepare_matdyn_phband(context)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["matdyn_phband"] != "finished":
        _submit_or_run(run_calc, submit_job, context.workspace, "run_matdyn_phband.sh")
        workflow = refresh_workflow_status(context.workspace, context.prefix)
        if submit_job or workflow["matdyn_phband"] != "finished":
            print_status(context.prefix, workflow)
            return 0

    prepare_matdyn_phdos(context)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["matdyn_phdos"] != "finished":
        _submit_or_run(run_calc, submit_job, context.workspace, "run_matdyn_phdos.sh")
    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
    return 0
