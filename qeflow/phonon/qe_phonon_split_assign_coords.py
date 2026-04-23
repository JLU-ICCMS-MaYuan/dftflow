#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from public.job import add_run_submit_arguments, submit_script
from qeflow.phonon._shared import (
    all_split_tasks_finished,
    any_split_task_running,
    create_context,
    dyn0_state,
    merge_state,
    prepare_root_scf_step,
    print_stage,
    print_status,
    refresh_workflow_status,
    root_has_running_step,
    root_step_scripts,
    run_root_scripts,
    split_task_needs_run,
    write_aggregate_script,
    write_split_stage_script,
)
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_merge import prepare_merge_step, run_merge_step
from qeflow.phonon.qe_ph_nosplit import prepare_ph_nosplit_step, run_ph_to_dyn0
from qeflow.phonon.qe_ph_split_assign_coords import prepare_split_assign_coords_step, _run_shell as split_run_shell
from qeflow.phonon.qe_q2r import prepare_q2r_step


def _ensure_split_materialized(config_file: str, struct_file: Optional[str]) -> dict:
    context = create_context(config_file, struct_file, workflow_name="qe_phonon_split_coords")
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if dyn0_state(workflow) == "finished" and not os.path.exists(os.path.join(context.workspace, "q_index_map.json")):
        print_stage(3, "生成 split q 点输入")
        prepare_split_assign_coords_step(context)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
    return workflow


def run_phonon_split_assign_coords_workflow(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
) -> int:
    context = create_context(config_file, struct_file, workflow_name="qe_phonon_split_coords")
    prepare_root_scf_step(context)
    prepare_ph_nosplit_step(context, dyn0_only=True)
    prepare_merge_step(context)
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)

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
            stage_scripts = []
            if workflow["scf"] not in {"finished", "running"}:
                stage_scripts.append("run_scf.sh")
            if dyn0_state(workflow) not in {"finished", "running"}:
                stage_scripts.append("run_ph.sh")
            if stage_scripts:
                print_stage(1, "提交前计算阶段")
                aggregate = write_aggregate_script(context, "submit_pre_stage.sh", stage_scripts)
                submit_script(context.workspace, os.path.basename(aggregate))
            print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
            return 0

    workflow = _ensure_split_materialized(config_file, struct_file)

    if split_task_needs_run(workflow):
        if run_calc:
            tasks = workflow.get("tasks", {})
            if isinstance(tasks, dict):
                split_task_no = 1
                for q_name in sorted(tasks):
                    task_payload = tasks[q_name]
                    if isinstance(task_payload, dict) and task_payload.get("state") != "finished":
                        print_stage(f"3_{split_task_no}", f"运行 split 任务 {q_name}")
                        split_run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
                        workflow = refresh_workflow_status(context.workspace, context.prefix)
                        split_task_no += 1
        elif submit_job and not any_split_task_running(workflow):
            print_stage(2, "提交 split 阶段")
            aggregate = write_split_stage_script(context, workflow)
            submit_script(context.workspace, os.path.basename(aggregate))
            print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
            return 0

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if all_split_tasks_finished(workflow):
        if run_calc:
            if merge_state(workflow) != "finished":
                print_stage(4, "执行 merge")
                if run_merge_step(
                    config_file,
                    struct_file,
                    run_calc=True,
                    submit_job=False,
                    workflow_name="qe_phonon_split_coords",
                ) != 0:
                    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
                    return 1
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            run_root_scripts(
                context,
                root_step_scripts(workflow, include_ph=False),
                labels={
                    "run_q2r.sh": "运行 q2r",
                    "run_matdyn_phband.sh": "运行 matdyn_phband",
                    "run_matdyn_phdos.sh": "运行 matdyn_phdos",
                },
                stage_no=5,
            )
        elif submit_job and not root_has_running_step(workflow, include_ph=False):
            if merge_state(workflow) != "finished":
                print_stage(3, "提交 merge 阶段")
                aggregate = write_aggregate_script(context, "submit_merge_stage.sh", ["run_merge.sh"])
                submit_script(context.workspace, os.path.basename(aggregate))
                print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
                return 0
            remaining_scripts = root_step_scripts(workflow, include_ph=False)
            if remaining_scripts:
                print_stage(4, "提交后处理阶段")
                aggregate = write_aggregate_script(context, "submit_post_stage.sh", remaining_scripts)
                submit_script(context.workspace, os.path.basename(aggregate))
    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO 声子总工作流，按不可约 q 点坐标拆分")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接推进工作流", "提交当前阶段脚本")
    args = parser.parse_args()
    run_phonon_split_assign_coords_workflow(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
