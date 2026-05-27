#!/usr/bin/env python3
import os
import shlex
from typing import Optional

from qeflow.phonon import console
from qeflow.phonon import runner as runner_module
from qeflow.phonon.archive import (
    build_collect_phband_commands,
    build_collect_phdos_commands,
    collect_phband_outputs,
    collect_phdos_outputs,
    root_phband_outputs_exist,
    root_phdos_outputs_exist,
)
from qeflow.phonon.context import PhononStepContext, create_context
from qeflow.phonon.phonon_plot import plot_phonon_outputs
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_q2r import prepare_q2r_step
from qeflow.phonon.runner import print_stage
from qeflow.phonon.script_plan import build_labeled_commands
from qeflow.phonon.status_machine import refresh_workflow_status
from qeflow.qe_input.phonon_input import write_root_run_script


PROCESS_SCRIPT = "submit_process_phonon.sh"


def prepare_process_phonon_step(context: PhononStepContext) -> str:
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)
    config_arg = shlex.quote(context.config_file)
    commands = build_labeled_commands(
        [
            ("运行 q2r", "bash run_q2r.sh"),
            ("运行 matdyn 声子谱", "bash run_matdyn_phband.sh"),
            ("归档声子谱结果", f"qe_process_phonon -c {config_arg} --collect phband"),
            ("运行 matdyn 声子 DOS", "bash run_matdyn_phdos.sh"),
            ("归档声子 DOS 结果", f"qe_process_phonon -c {config_arg} --collect phdos"),
            ("绘图并写出 CSV", f"qe_process_phonon -c {config_arg} --plot"),
        ],
        workflow_name=os.path.basename(context.workspace),
        prefix=context.prefix,
        work_dir=context.workspace,
        stage="process_phonon",
    )
    return write_root_run_script(
        context.workspace,
        PROCESS_SCRIPT,
        str(context.config.get("slurm", {}).get("header", "")),
        ["set -e", *commands],
        announce=False,
    )


def prepare_matdyn_phband_collect_step(context: PhononStepContext) -> str:
    prepare_matdyn_phband_step(context)
    return write_root_run_script(
        context.workspace,
        "submit_matdyn_phband.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        ["bash run_matdyn_phband.sh", *build_collect_phband_commands(context.prefix)],
        announce=False,
    )


def prepare_matdyn_phdos_collect_step(context: PhononStepContext) -> str:
    prepare_matdyn_phdos_step(context)
    return write_root_run_script(
        context.workspace,
        "submit_matdyn_phdos.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        ["bash run_matdyn_phdos.sh", *build_collect_phdos_commands(context.prefix)],
        announce=False,
    )


def run_process_phonon_steps(context: PhononStepContext, include_plot: bool) -> int:
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)
    stage_no = 1
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["q2r"] != "finished":
        stage_no = _run_process_action(context, stage_no, "运行 q2r", lambda: _active_run_shell()("bash run_q2r.sh", context.workspace))
    if workflow["matdyn_phband"] != "finished":
        if root_phband_outputs_exist(context):
            stage_no = _run_process_action(context, stage_no, "归档 phband", lambda: collect_phband_outputs(context))
        else:
            stage_no = _run_process_action(context, stage_no, "运行 matdyn_phband", lambda: _active_run_shell()("bash run_matdyn_phband.sh", context.workspace))
            stage_no = _run_process_action(context, stage_no, "归档 phband", lambda: collect_phband_outputs(context))
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["matdyn_phdos"] != "finished":
        if root_phdos_outputs_exist(context):
            stage_no = _run_process_action(context, stage_no, "归档 phdos", lambda: collect_phdos_outputs(context))
        else:
            stage_no = _run_process_action(context, stage_no, "运行 matdyn_phdos", lambda: _active_run_shell()("bash run_matdyn_phdos.sh", context.workspace))
            stage_no = _run_process_action(context, stage_no, "归档 phdos", lambda: collect_phdos_outputs(context))
    if include_plot:
        print_stage(stage_no, "绘制声子谱和声子 DOS")
        try:
            plot_phonon_outputs(context)
        finally:
            refresh_workflow_status(context.workspace, context.prefix)
    return 0


def _run_process_action(context: PhononStepContext, stage_no: int, label: str, action) -> int:
    print_stage(stage_no, label)
    try:
        action()
    except Exception:
        refresh_workflow_status(context.workspace, context.prefix)
        raise
    refresh_workflow_status(context.workspace, context.prefix)
    return stage_no + 1


def _active_run_shell():
    return runner_module.run_shell


def run_process_phonon(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
    collect_only: object,
    plot_only: bool,
) -> int:
    context = create_context(config_file, struct_file)
    if run_calc:
        run_process_phonon_steps(context, include_plot=True)
    elif submit_job:
        prepare_process_phonon_step(context)
        console.generated(PROCESS_SCRIPT)
        console.submit_script(context.workspace, PROCESS_SCRIPT)
    elif collect_only:
        target = "all" if collect_only is True else str(collect_only)
        if target in {"all", "phband"}:
            collect_phband_outputs(context)
        if target in {"all", "phdos"}:
            collect_phdos_outputs(context)
    elif plot_only:
        plot_phonon_outputs(context)
    else:
        prepare_process_phonon_step(context)
    refresh_workflow_status(context.workspace, context.prefix)
    if run_calc:
        console.completion("q2r + matdyn + collect + plot")
    return 0
