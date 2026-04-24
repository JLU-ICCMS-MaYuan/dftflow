#!/usr/bin/env python3
import os
import shlex
import subprocess
from dataclasses import dataclass
from typing import Dict, Iterable, List, Optional

from public.job import submit_script
from qeflow.qe_check.checks import check_merge, check_ph, check_q2r, check_scf, summarize_task_states
from qeflow.qe_input.builders import (
    build_qe_commands,
    parse_poscar,
    resolve_prefix,
    write_root_run_script,
    write_scf_input,
)
from qeflow.qe_input.fileio import dump_json, list_split_directories, load_json, resolve_workspace


WORKFLOW_STATUS_FILE = "workflow_status.json"
TASK_STATUS_FILE = "task_status.json"


@dataclass
class PhononStepContext:
    config: Dict[str, object]
    workspace: str
    prefix: str
    struct_info: Dict[str, object]
    commands: Dict[str, str]


def run_shell(command: str, cwd: str) -> None:
    subprocess.run(command, shell=True, check=True, cwd=cwd)


def submit_or_run(run_calc: bool, submit_job: bool, cwd: str, script_name: str) -> bool:
    if run_calc:
        run_shell(f"bash {shlex.quote(script_name)}", cwd)
        return True
    if submit_job:
        submit_script(cwd, script_name)
        return True
    return False


def print_stage(stage_no: int, message: str) -> None:
    print(f"{stage_no}. {message}")


def create_context(
    config_file: str,
    struct_file: Optional[str],
    workflow_name: Optional[str] = None,
) -> PhononStepContext:
    config, _root_dir, workspace = resolve_workspace(
        config_file,
        struct_file,
        prepare_inputs=True,
        workflow_name=workflow_name,
    )
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


def parse_dyn0_qpoints(dyn0_path: str) -> List[Dict[str, object]]:
    if not os.path.exists(dyn0_path):
        raise FileNotFoundError(f"找不到 dyn0 文件: {dyn0_path}")
    with open(dyn0_path, "r", encoding="utf-8") as handle:
        lines = [line.strip() for line in handle if line.strip()]
    if len(lines) < 2:
        raise ValueError(f"dyn0 文件格式不完整: {dyn0_path}")
    counts = lines[0].split()
    if len(counts) != 3:
        raise ValueError(f"dyn0 第一行无法解析 nq1/nq2/nq3: {lines[0]}")
    try:
        q_count = int(lines[1].split()[0])
    except (IndexError, ValueError) as exc:
        raise ValueError(f"dyn0 第二行无法解析 q 点数量: {lines[1]}") from exc
    if len(lines) < 2 + q_count:
        raise ValueError(f"dyn0 文件中的 q 点数量不足: 期望 {q_count}，实际 {len(lines) - 2}")

    qpoints: List[Dict[str, object]] = []
    for index in range(q_count):
        raw_line = lines[2 + index]
        parts = raw_line.split()
        if len(parts) != 3:
            raise ValueError(f"dyn0 q 点行无法解析为 3 个分量: {raw_line}")
        q_crystal = [float(parts[0]), float(parts[1]), float(parts[2])]
        qpoints.append(
            {
                "index": index + 1,
                "q_crystal": q_crystal,
            }
        )
    return qpoints


def prepare_root_scf_step(context: PhononStepContext) -> str:
    write_scf_input(context.workspace, context.config, context.struct_info, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_scf.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['pw']} < scf.in > scf.out"],
    )


def load_q_index_entries(workspace: str) -> List[Dict[str, object]]:
    payload = load_json(os.path.join(workspace, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return []
    qpoints = payload.get("qpoints", [])
    return qpoints if isinstance(qpoints, list) else []


def load_q_index_mode(workspace: str) -> str:
    payload = load_json(os.path.join(workspace, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return ""
    mode = payload.get("mode")
    return str(mode) if isinstance(mode, str) else ""


def iter_q_directory_names(workspace: str) -> List[str]:
    names: List[str] = []
    for entry in load_q_index_entries(workspace):
        if not isinstance(entry, dict):
            continue
        directory = str(entry.get("directory", "")).strip()
        if directory:
            names.append(directory)
    return names


def refresh_split_task_status(workspace: str) -> Dict[str, Dict[str, str]]:
    from qeflow.qe_check.checks import check_split_task, get_running_workdirs

    running = get_running_workdirs()
    task_payload: Dict[str, Dict[str, str]] = {}
    q_names = iter_q_directory_names(workspace)
    if not q_names:
        q_names = [os.path.basename(q_dir) for q_dir in list_split_directories(workspace)]
    for q_name in q_names:
        q_dir = os.path.join(workspace, q_name)
        payload = {
            "directory": q_name,
            "state": check_split_task(q_dir, running),
        }
        dump_json(os.path.join(q_dir, TASK_STATUS_FILE), payload)
        task_payload[q_name] = payload
    return task_payload


def refresh_workflow_status(workspace: str, prefix: str) -> Dict[str, object]:
    from qeflow.qe_check.checks import check_matdyn, get_running_workdirs

    running = get_running_workdirs()
    workflow = {
        "scf": check_scf(workspace, running_dirs=running),
        "ph": check_ph(workspace, prefix, running_dirs=running),
        "merge": check_merge(workspace, prefix, running_dirs=running),
        "q2r": check_q2r(workspace, prefix, running_dirs=running),
        "matdyn_phband": check_matdyn(
            workspace,
            "matdyn_phband.out",
            f"{prefix}.phband.freq",
            running_dirs=running,
        ),
        "matdyn_phdos": check_matdyn(
            workspace,
            "matdyn_phdos.out",
            f"{prefix}.phdos.dat",
            running_dirs=running,
        ),
    }
    workflow["tasks"] = refresh_split_task_status(workspace)
    dump_json(os.path.join(workspace, WORKFLOW_STATUS_FILE), workflow)
    return workflow


def ph_state(workflow: Dict[str, object]) -> str:
    ph_payload = workflow["ph"]
    return str(ph_payload["state"]) if isinstance(ph_payload, dict) else str(ph_payload)


def dyn0_state(workflow: Dict[str, object]) -> str:
    ph_payload = workflow["ph"]
    return str(ph_payload["dyn0"]) if isinstance(ph_payload, dict) else "unstarted"


def merge_state(workflow: Dict[str, object]) -> str:
    return str(workflow.get("merge", "unstarted"))


def all_split_tasks_finished(workflow: Dict[str, object]) -> bool:
    tasks = workflow.get("tasks", {})
    if not isinstance(tasks, dict) or not tasks:
        return False
    return summarize_task_states(
        str(task["state"]) for task in tasks.values() if isinstance(task, dict)
    ) == "finished"


def any_split_task_running(workflow: Dict[str, object]) -> bool:
    tasks = workflow.get("tasks", {})
    if not isinstance(tasks, dict) or not tasks:
        return False
    return summarize_task_states(
        str(task["state"]) for task in tasks.values() if isinstance(task, dict)
    ) == "running"


def split_task_needs_run(workflow: Dict[str, object]) -> bool:
    tasks = workflow.get("tasks", {})
    if not isinstance(tasks, dict) or not tasks:
        return False
    return summarize_task_states(
        str(task["state"]) for task in tasks.values() if isinstance(task, dict)
    ) != "finished"


def print_status(prefix: str, workflow: Dict[str, object]) -> None:
    print(f"scf: {workflow['scf']}")
    ph_payload = workflow["ph"]
    if isinstance(ph_payload, dict):
        print(f"ph: {ph_payload['state']}")
        print(f"dyn0: {ph_payload['dyn0']}")
    print(f"merge: {merge_state(workflow)}")
    print(f"q2r: {workflow['q2r']}")
    print(f"matdyn_phband: {workflow['matdyn_phband']}")
    print(f"matdyn_phdos: {workflow['matdyn_phdos']}")
    tasks = workflow.get("tasks", {})
    if isinstance(tasks, dict) and tasks:
        for q_name in sorted(tasks):
            task_payload = tasks[q_name]
            if isinstance(task_payload, dict):
                print(f"{q_name}: {task_payload.get('state', 'unstarted')}")
    print(f"prefix: {prefix}")


def root_step_scripts(workflow: Dict[str, object], include_ph: bool) -> List[str]:
    steps: List[str] = []
    if workflow["scf"] not in {"finished", "running"}:
        steps.append("run_scf.sh")
    if include_ph and ph_state(workflow) not in {"finished", "running"}:
        steps.append("run_ph.sh")
    if workflow["q2r"] not in {"finished", "running"}:
        steps.append("run_q2r.sh")
    if workflow["matdyn_phband"] not in {"finished", "running"}:
        steps.append("run_matdyn_phband.sh")
    if workflow["matdyn_phdos"] not in {"finished", "running"}:
        steps.append("run_matdyn_phdos.sh")
    return steps


def root_has_running_step(workflow: Dict[str, object], include_ph: bool) -> bool:
    if workflow["scf"] == "running":
        return True
    if include_ph and ph_state(workflow) == "running":
        return True
    if workflow["q2r"] == "running":
        return True
    if workflow["matdyn_phband"] == "running":
        return True
    if workflow["matdyn_phdos"] == "running":
        return True
    return False


def write_aggregate_script(context: PhononStepContext, script_name: str, scripts: Iterable[str]) -> str:
    commands = [f"bash {script}" for script in scripts]
    return write_root_run_script(
        context.workspace,
        script_name,
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
    )


def write_split_stage_script(context: PhononStepContext, workflow: Dict[str, object]) -> str:
    tasks = workflow.get("tasks", {})
    commands: List[str] = []
    if isinstance(tasks, dict):
        for q_name in iter_q_directory_names(context.workspace):
            task_payload = tasks.get(q_name)
            if isinstance(task_payload, dict) and task_payload.get("state") not in {"finished", "running"}:
                commands.append(f"(cd {shlex.quote(q_name)} && bash run_split.sh)")
    return write_root_run_script(
        context.workspace,
        "submit_split_stage.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
    )


def submit_split_stage_tasks(context: PhononStepContext, workflow: Dict[str, object]) -> int:
    tasks = workflow.get("tasks", {})
    submitted = 0
    if not isinstance(tasks, dict):
        return submitted
    for q_name in iter_q_directory_names(context.workspace):
        task_payload = tasks.get(q_name)
        if isinstance(task_payload, dict) and task_payload.get("state") not in {"finished", "running"}:
            submit_script(os.path.join(context.workspace, q_name), "run_split.sh")
            submitted += 1
    return submitted


def run_root_scripts(context: PhononStepContext, scripts: Iterable[str], labels: Optional[Dict[str, str]] = None, stage_no: int = 1) -> int:
    mapping = labels or {}
    current_stage = stage_no
    for script_name in scripts:
        print_stage(current_stage, mapping.get(script_name, f"运行 {script_name}"))
        run_shell(f"bash {shlex.quote(script_name)}", context.workspace)
        refresh_workflow_status(context.workspace, context.prefix)
        current_stage += 1
    return current_stage
