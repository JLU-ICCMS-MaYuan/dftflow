#!/usr/bin/env python3
import os
from typing import Dict, List, Tuple

from qeflow.phonon import console
from qeflow.qe_check.checks import (
    check_merge,
    check_ph,
    check_q2r,
    check_scf,
    collect_split_task_states,
    summarize_task_states,
)
from qeflow.io_utils import dump_json


WORKFLOW_STATUS_FILE = "workflow_status.json"
TASK_STATUS_FILE = "task_status.json"


def ph_tasks(workflow: Dict[str, object]) -> Dict[str, str]:
    ph_payload = workflow.get("ph", {})
    if not isinstance(ph_payload, dict):
        return {}
    tasks = ph_payload.get("tasks", {})
    if not isinstance(tasks, dict):
        return {}
    return {
        str(q_name): str(state)
        for q_name, state in tasks.items()
        if isinstance(q_name, str) and isinstance(state, str)
    }


def refresh_split_task_status(workspace: str) -> Dict[str, str]:
    task_states = collect_split_task_states(workspace)
    for q_name, state in task_states.items():
        q_dir = os.path.join(workspace, q_name)
        payload = {"directory": q_name, "state": state}
        dump_json(os.path.join(q_dir, TASK_STATUS_FILE), payload)
    return task_states


def refresh_workflow_status(workspace: str, prefix: str) -> Dict[str, object]:
    from qeflow.qe_check.checks import check_matdyn, check_phonon_plot

    matdyn_phband = check_matdyn(
        workspace,
        "matdyn_phband.out",
        f"{prefix}.phband.freq",
        archived_products=[
            os.path.join("phband", f"{prefix}.phband.freq"),
            os.path.join("phband", f"{prefix}_band.labelinfo.dat"),
        ],
    )
    matdyn_phdos = check_matdyn(
        workspace,
        "matdyn_phdos.out",
        f"{prefix}.phdos.dat",
        archived_products=[os.path.join("phdos", f"{prefix}.phdos.dat")],
    )
    workflow = {
        "scf": check_scf(workspace),
        "ph": check_ph(workspace, prefix),
        "merge": check_merge(workspace, prefix),
        "q2r": check_q2r(workspace, prefix),
        "matdyn_phband": matdyn_phband,
        "matdyn_phdos": matdyn_phdos,
        "phonon_plot": check_phonon_plot(workspace, prefix, matdyn_phband, matdyn_phdos),
    }
    workflow["ph"]["tasks"] = refresh_split_task_status(workspace)
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
    tasks = ph_tasks(workflow)
    if not tasks:
        return False
    return summarize_task_states(tasks.values()) == "finished"


def split_task_needs_run(workflow: Dict[str, object]) -> bool:
    tasks = ph_tasks(workflow)
    if not tasks:
        return False
    return summarize_task_states(tasks.values()) != "finished"


def print_status(prefix: str, workflow: Dict[str, object]) -> None:
    print_resume_point(workflow)


def split_resume_step(workflow: Dict[str, object]) -> Tuple[int, str]:
    if workflow["scf"] != "finished":
        return 1, "运行 SCF"
    if dyn0_state(workflow) != "finished":
        return 2, "运行 dyn0"
    if not all_split_tasks_finished(workflow):
        return 3, "提交 split q 点任务"
    if merge_state(workflow) != "finished":
        return 4, "合并 split q 点结果"
    if workflow["q2r"] != "finished":
        return 5, "运行 q2r"
    if workflow["matdyn_phband"] != "finished":
        return 6, "运行 matdyn 声子谱"
    if workflow["matdyn_phdos"] != "finished":
        return 8, "运行 matdyn 声子 DOS"
    if workflow.get("phonon_plot", "unstarted") != "finished":
        return 10, "绘图并写出 CSV"
    return 11, "完成声子回收"


def print_resume_point(workflow: Dict[str, object]) -> None:
    step_no, label = split_resume_step(workflow)
    console.resume(step_no, 11, label)


def root_step_scripts(workflow: Dict[str, object], include_ph: bool) -> List[str]:
    steps: List[str] = []
    if workflow["scf"] != "finished":
        steps.append("run_scf.sh")
    if include_ph and ph_state(workflow) != "finished":
        steps.append("run_ph.sh")
    if workflow["q2r"] != "finished":
        steps.append("run_q2r.sh")
    if workflow["matdyn_phband"] != "finished":
        steps.append("run_matdyn_phband.sh")
    if workflow["matdyn_phdos"] != "finished":
        steps.append("run_matdyn_phdos.sh")
    return steps
