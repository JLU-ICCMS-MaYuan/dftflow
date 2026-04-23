#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from public.job import add_run_submit_arguments, submit_script
from qeflow.phonon._shared import (
    PhononStepContext,
    all_split_tasks_finished,
    create_context,
    dyn0_state,
    iter_q_directory_names,
    load_q_index_entries,
    merge_state,
    parse_dyn0_qpoints,
    ph_state,
    prepare_root_scf_step,
    print_stage,
    print_status,
    refresh_workflow_status,
    root_has_running_step,
    root_step_scripts,
    run_shell,
    write_aggregate_script,
)
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_merge import prepare_merge_step, run_merge_step
from qeflow.phonon.qe_ph_nosplit import prepare_ph_nosplit_step, run_ph_to_dyn0
from qeflow.phonon.qe_q2r import prepare_q2r_step
from qeflow.qe_check.checks import check_scf
from qeflow.qe_input.builders import write_scf_input, write_split_ph_input_assign_index, write_split_run_script
from qeflow.qe_input.fileio import dump_json, load_json


_run_shell = run_shell

def prepare_split_assign_index_step(context: PhononStepContext) -> int:
    dyn0_path = os.path.join(context.workspace, f"{context.prefix}.dyn0")
    qpoints = parse_dyn0_qpoints(dyn0_path)
    slurm_header = str(context.config.get("slurm", {}).get("header", ""))
    q_map_payload = {"mode": "split_assign_index", "qpoints": []}
    for dir_index, qpoint in enumerate(qpoints):
        q_name = f"{dir_index + 1:02d}"
        q_dir = os.path.join(context.workspace, q_name)
        os.makedirs(q_dir, exist_ok=True)
        write_scf_input(q_dir, context.config, context.struct_info, context.prefix)
        write_split_ph_input_assign_index(q_dir, context.config, context.prefix, int(qpoint["index"]))
        write_split_run_script(q_dir, slurm_header, context.commands["pw"], context.commands["ph"])
        q_map_payload["qpoints"].append(
            {
                "directory": q_name,
                "index": int(qpoint["index"]),
                "q_crystal": qpoint["q_crystal"],
            }
        )
    dump_json(os.path.join(context.workspace, "q_index_map.json"), q_map_payload)
    return len(q_map_payload["qpoints"])


def _ensure_split_materialized(context: PhononStepContext) -> dict:
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if dyn0_state(workflow) == "finished" and not load_json(os.path.join(context.workspace, "q_index_map.json")):
        print_stage(2, "生成 split q 点输入")
        prepare_split_assign_index_step(context)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
    return workflow


def run_split_assign_index_step(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
) -> int:
    context = create_context(config_file, struct_file)
    if check_scf(context.workspace) != "finished":
        raise RuntimeError("split 模式依赖根目录已完成的 scf.out。")
    prepare_root_scf_step(context)
    prepare_ph_nosplit_step(context, dyn0_only=True)
    prepare_merge_step(context)
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if all_split_tasks_finished(workflow):
        if merge_state(workflow) != "finished":
            print_stage(4, "执行 merge")
            if run_merge_step(config_file, struct_file, run_calc=True, submit_job=False) != 0:
                return 1
            workflow = refresh_workflow_status(context.workspace, context.prefix)
        if submit_job:
            remaining_scripts = root_step_scripts(workflow, include_ph=False)
            if remaining_scripts and not root_has_running_step(workflow, include_ph=False):
                print_stage(5, "提交 q2r + matdyn 后处理")
                aggregate = write_aggregate_script(context, "submit_post_stage.sh", remaining_scripts)
                submit_script(context.workspace, os.path.basename(aggregate))
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    if dyn0_state(workflow) != "finished":
        if run_calc:
            print_stage(1, "运行根目录 dyn0 预处理")
            run_ph_to_dyn0(
                context.workspace,
                context.commands["ph"],
                os.path.join(context.workspace, f"{context.prefix}.dyn0"),
                os.path.join(context.workspace, "ph.out"),
            )
        elif submit_job:
            print_stage(1, "提交根目录 dyn0 预处理")
            submit_script(context.workspace, "run_ph.sh")
            return 0
        elif not load_json(os.path.join(context.workspace, "q_index_map.json")):
            print_stage(1, "生成 split q 点输入")
            prepare_split_assign_index_step(context)

    workflow = _ensure_split_materialized(context)
    q_names = iter_q_directory_names(context.workspace)
    if run_calc or submit_job:
        split_task_no = 1
        for q_name in q_names:
            if run_calc:
                print_stage(f"3_{split_task_no}", f"运行 split 任务 {q_name}")
                _run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
            else:
                print_stage(f"3_{split_task_no}", f"提交 split 任务 {q_name}")
                submit_script(os.path.join(context.workspace, q_name), "run_split.sh")
            split_task_no += 1
    refresh_workflow_status(context.workspace, context.prefix)
    if not (run_calc or submit_job):
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO split ph.x，按不可约 q 点编号拆分")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接运行 split 任务", "提交 split 任务")
    args = parser.parse_args()
    run_split_assign_index_step(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
