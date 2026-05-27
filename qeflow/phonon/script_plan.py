#!/usr/bin/env python3
import os
import shlex
from typing import Dict, Iterable, List, Optional, Sequence, Tuple

from qeflow.phonon import console
from qeflow.phonon.context import PhononStepContext
from qeflow.phonon.q_index_map import iter_q_directory_names
from qeflow.phonon.status_machine import ph_tasks
from qeflow.input_deck.scripts import write_root_run_script


submit_script = console.submit_script


def build_labeled_commands(
    steps: Sequence[Tuple[str, str]],
    *,
    start_index: int = 1,
    total_steps: Optional[int] = None,
    workflow_name: Optional[str] = None,
    prefix: Optional[str] = None,
    work_dir: Optional[str] = None,
    stage: Optional[str] = None,
) -> List[str]:
    commands: List[str] = []
    total = total_steps or len(steps)
    for index, (label, command) in enumerate(steps, start=1):
        step_no = start_index + index - 1
        step_label = f"{step_no}/{total} {label}"
        if workflow_name and prefix and work_dir:
            commands.extend(
                [
                    console.echo_qeflow("-" * 60),
                    console.echo_qeflow(f"workflow: {workflow_name}"),
                    console.echo_qeflow(f"prefix: {prefix}"),
                ]
            )
            if stage:
                commands.append(console.echo_qeflow(f"stage: {stage}"))
            commands.extend(
                [
                    console.echo_qeflow(f"step: {step_label}"),
                    console.echo_qeflow(f"work_dir: {work_dir}"),
                    console.echo_qeflow(f"command: {command}"),
                    f'if ! cd "{work_dir}"; then',
                    console.echo_qeflow(f"failed: {step_label}"),
                    "  exit 1",
                    "fi",
                    f"if ! {command}; then",
                    console.echo_qeflow(f"failed: {step_label}"),
                    "  exit 1",
                    "fi",
                    console.echo_qeflow(f"done: {step_label}"),
                ]
            )
        else:
            commands.append(console.echo_command_step(step_no, total, label, command))
            commands.append(command)
    return commands


def write_labeled_script(
    context: PhononStepContext,
    script_name: str,
    steps: Sequence[Tuple[str, str]],
    *,
    start_index: int = 1,
    total_steps: Optional[int] = None,
    stage: Optional[str] = None,
    completion_message: Optional[str] = None,
) -> str:
    commands = [
        "set -e",
        *build_labeled_commands(
            steps,
            start_index=start_index,
            total_steps=total_steps,
            workflow_name=os.path.basename(context.workspace),
            prefix=context.prefix,
            work_dir=context.workspace,
            stage=stage,
        ),
    ]
    if completion_message:
        commands.append(console.echo_completion(completion_message))
    return write_root_run_script(
        context.workspace,
        script_name,
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
        announce=False,
    )


def write_aggregate_script(context: PhononStepContext, script_name: str, scripts: Iterable[str]) -> str:
    script_list = list(scripts)
    commands = [
        "set -e",
        *build_labeled_commands(
            [(script, f"bash {script}") for script in script_list],
            workflow_name=os.path.basename(context.workspace),
            prefix=context.prefix,
            work_dir=context.workspace,
            stage=os.path.splitext(script_name)[0],
        ),
    ]
    return write_root_run_script(
        context.workspace,
        script_name,
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
        announce=False,
    )


def write_split_stage_script(context: PhononStepContext, workflow: Dict[str, object]) -> str:
    tasks = ph_tasks(workflow)
    commands: List[str] = []
    for q_name in iter_q_directory_names(context.workspace):
        state = tasks.get(q_name)
        if state != "finished":
            commands.append(f"(cd {shlex.quote(q_name)} && bash run_split.sh)")
    return write_root_run_script(
        context.workspace,
        "submit_split_stage.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
        announce=False,
    )


def submit_split_stage_tasks(context: PhononStepContext, workflow: Dict[str, object]) -> int:
    tasks = ph_tasks(workflow)
    submitted = 0
    for q_name in iter_q_directory_names(context.workspace):
        state = tasks.get(q_name)
        if state != "finished":
            _active_submit_script()(os.path.join(context.workspace, q_name), "run_split.sh")
            submitted += 1
    return submitted


def _active_submit_script():
    return submit_script
