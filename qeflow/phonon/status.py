#!/usr/bin/env python3
import os
from typing import Dict

from qeflow.phonon.checks import check_matdyn, check_ph, check_q2r, check_scf, get_running_workdirs
from qeflow.phonon.fileio import dump_json, list_split_directories


WORKFLOW_STATUS_FILE = "workflow_status.json"
TASK_STATUS_FILE = "task_status.json"


def refresh_split_task_status(workspace: str) -> Dict[str, Dict[str, str]]:
    running = get_running_workdirs()
    task_payload: Dict[str, Dict[str, str]] = {}
    from qeflow.phonon.checks import check_split_task

    for q_dir in list_split_directories(workspace):
        q_name = os.path.basename(q_dir)
        payload = {
            "directory": q_name,
            "state": check_split_task(q_dir, running),
        }
        dump_json(os.path.join(q_dir, TASK_STATUS_FILE), payload)
        task_payload[q_name] = payload
    return task_payload


def refresh_workflow_status(workspace: str, prefix: str) -> Dict[str, object]:
    running = get_running_workdirs()
    workflow = {
        "scf": check_scf(workspace, running_dirs=running),
        "ph": check_ph(workspace, prefix, running_dirs=running),
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
