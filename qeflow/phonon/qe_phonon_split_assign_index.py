#!/usr/bin/env python3
import argparse
import os
import shlex
from typing import Optional

from public.job import add_run_submit_arguments
from qeflow.phonon import console
from qeflow.phonon._shared import (
    all_split_tasks_finished,
    create_context,
    dyn0_state,
    merge_state,
    ph_tasks,
    prepare_root_scf_step,
    print_stage,
    print_resume_point,
    refresh_workflow_status,
    root_step_scripts,
    run_shell as split_run_shell,
    run_root_scripts,
    split_task_needs_run,
    write_labeled_script,
)
from qeflow.phonon.qe_merge import run_merge_step
from qeflow.phonon.qe_split_submit import run_split_submit
from qeflow.phonon.ph_step import prepare_ph_nosplit_step, run_ph_to_dyn0
from qeflow.phonon.process_phonon import (
    prepare_matdyn_phband_step,
    prepare_matdyn_phdos_step,
    prepare_q2r_step,
    run_process_phonon_steps,
)
from qeflow.phonon.split_prepare import prepare_split_assign_index_step

submit_script = console.submit_script
ROOT_SCRIPT_NAMES = [
    "run_scf.sh",
    "run_ph.sh",
    "run_q2r.sh",
    "run_matdyn_phband.sh",
    "run_matdyn_phdos.sh",
]


def _ensure_split_materialized(config_file: str, struct_file: Optional[str]) -> dict:
    context = create_context(config_file, struct_file, workflow_name="qe_phonon_split_index")
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if dyn0_state(workflow) == "finished" and not os.path.exists(os.path.join(context.workspace, "q_index_map.json")):
        print_stage(3, "生成 split q 点输入")
        prepare_split_assign_index_step(context)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
    return workflow


def _write_pre_and_split_stage_script(context, workflow) -> str:
    steps = []
    if workflow["scf"] != "finished":
        steps.append(("运行 SCF", "bash run_scf.sh"))
    if dyn0_state(workflow) != "finished":
        steps.append(("运行 dyn0", "bash run_ph.sh"))
    steps.append(
        (
            "准备并提交 split q 点任务",
            f"qe_split_submit -c {shlex.quote(context.config_file)} --mode index",
        )
    )
    return write_labeled_script(
        context,
        "submit_pre_and_split_stage.sh",
        steps,
        start_index=1,
        total_steps=11,
        stage="pre_and_split",
        completion_message="1/11-3/11 pre + split 提交阶段",
    )


def _write_merge_and_process_stage_script(context, workflow) -> str:
    steps = [
        ("merge", f"qe_merge -c {shlex.quote(context.config_file)}"),
        ("q2r", "bash run_q2r.sh"),
        ("matdyn_phband", "bash run_matdyn_phband.sh"),
        ("collect phband", f"qe_process_phonon -c {shlex.quote(context.config_file)} --collect phband"),
        ("matdyn_phdos", "bash run_matdyn_phdos.sh"),
        ("collect phdos", f"qe_process_phonon -c {shlex.quote(context.config_file)} --collect phdos"),
        ("plot", f"qe_process_phonon -c {shlex.quote(context.config_file)} --plot"),
        ("done", "true"),
    ]
    return write_labeled_script(
        context,
        "submit_merge_and_process_phonon.sh",
        steps,
        start_index=4,
        total_steps=11,
        stage="merge_and_process",
        completion_message="4/11-11/11 merge + phonon 后处理阶段",
    )


def run_phonon_split_assign_index_workflow(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
) -> int:
    context = create_context(config_file, struct_file, workflow_name="qe_phonon_split_index")
    prepare_root_scf_step(context)
    prepare_ph_nosplit_step(context, dyn0_only=True)
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)
    console.generated_scripts(ROOT_SCRIPT_NAMES)

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["scf"] != "finished" or dyn0_state(workflow) != "finished":
        if run_calc:
            if workflow["scf"] != "finished":
                run_root_scripts(
                    context,
                    ["run_scf.sh"],
                    labels={"run_scf.sh": "运行根目录 SCF"},
                    stage_no=1,
                )
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            if dyn0_state(workflow) != "finished":
                print_stage(2, "运行根目录 dyn0 预处理")
                run_ph_to_dyn0(
                    context.workspace,
                    context.commands["ph"],
                    os.path.join(context.workspace, f"{context.prefix}.dyn0"),
                    os.path.join(context.workspace, "ph.out"),
                )
                workflow = refresh_workflow_status(context.workspace, context.prefix)
        elif submit_job:
            if workflow["scf"] != "finished" or dyn0_state(workflow) != "finished":
                aggregate = _write_pre_and_split_stage_script(context, workflow)
                console.generated(os.path.basename(aggregate))
                submit_script(context.workspace, os.path.basename(aggregate))
            console.next_step("等待 split q 点任务完成后，再执行 qe_phonon_split_index -s")
            return 0

    workflow = _ensure_split_materialized(config_file, struct_file)

    if split_task_needs_run(workflow):
        if run_calc:
            tasks = ph_tasks(workflow)
            split_task_no = 1
            for q_name in sorted(tasks):
                if tasks.get(q_name) != "finished":
                    print_stage(f"3_{split_task_no}", f"运行 split 任务 {q_name}")
                    split_run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
                    workflow = refresh_workflow_status(context.workspace, context.prefix)
                    split_task_no += 1
        elif submit_job:
            run_split_submit(config_file, struct_file, "index")
            print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
            return 0

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if all_split_tasks_finished(workflow):
        if run_calc:
            if merge_state(workflow) != "finished":
                print_stage(4, "执行 merge")
                if run_merge_step(
                    config_file,
                    struct_file,
                    workflow_name="qe_phonon_split_index",
                ) != 0:
                    print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
                    return 1
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            run_process_phonon_steps(context, include_plot=True)
        elif submit_job:
            remaining_scripts = root_step_scripts(workflow, include_ph=False)
            process_needed = bool(remaining_scripts) or workflow.get("phonon_plot") != "finished"
            if merge_state(workflow) != "finished" or process_needed:
                aggregate = _write_merge_and_process_stage_script(context, workflow)
                console.generated(os.path.basename(aggregate))
                submit_script(context.workspace, os.path.basename(aggregate))
                console.next_step("查看对应 slurm 输出，等待 4/11-11/11 完成")
                return 0
    print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO 声子总工作流，按不可约 q 点编号拆分")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接推进工作流", "提交当前阶段脚本")
    args = parser.parse_args()
    run_phonon_split_assign_index_workflow(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
