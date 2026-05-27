#!/usr/bin/env python3
import os
import shlex
import subprocess
from typing import Dict, Iterable, Optional

from qeflow.phonon import console
from qeflow.phonon.context import PhononStepContext
from qeflow.phonon.status_machine import refresh_workflow_status


submit_script = console.submit_script


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
    console.stage(stage_no, message)


def run_root_scripts(
    context: PhononStepContext,
    scripts: Iterable[str],
    labels: Optional[Dict[str, str]] = None,
    stage_no: int = 1,
) -> int:
    mapping = labels or {}
    current_stage = stage_no
    for script_name in scripts:
        print_stage(current_stage, mapping.get(script_name, f"运行 {script_name}"))
        _active_run_shell()(f"bash {shlex.quote(script_name)}", context.workspace)
        refresh_workflow_status(context.workspace, context.prefix)
        current_stage += 1
    return current_stage


def _active_run_shell():
    return run_shell
