#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from public.job import submit_script
from qeflow.phonon._shared import (
    create_context,
    dyn0_state,
    iter_q_directory_names,
    ph_tasks,
    print_stage,
    print_status,
    refresh_workflow_status,
)
from qeflow.phonon.split_prepare import prepare_split_assign_coords_step, prepare_split_assign_index_step


def _prepare_split_by_mode(context, mode: str) -> int:
    if mode == "index":
        return prepare_split_assign_index_step(context)
    if mode == "coords":
        return prepare_split_assign_coords_step(context)
    raise ValueError(f"不支持的 split 模式: {mode}")


def run_split_submit(
    config_file: str,
    struct_file: Optional[str],
    mode: str,
) -> int:
    workflow_name = f"qe_phonon_split_{mode}"
    context = create_context(config_file, struct_file, workflow_name=workflow_name)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if dyn0_state(workflow) != "finished":
        print(f"dyn0 未完成，不能提交 split q 点任务: {dyn0_state(workflow)}")
        print_status(context.prefix, workflow)
        return 1

    print_stage(1, "准备 split q 点目录")
    _prepare_split_by_mode(context, mode)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    tasks = ph_tasks(workflow)

    print_stage(2, "提交 split q 点任务")
    submitted = 0
    for q_name in iter_q_directory_names(context.workspace):
        state = tasks.get(q_name, "unstarted")
        if state == "finished":
            print(f"[DFTFlow] skip: q-point {q_name} finished")
            continue
        if state not in {"error", "unstarted"}:
            print(f"[DFTFlow] skip: q-point {q_name} state={state}")
            continue
        print(f"[DFTFlow] submit: q-point {q_name}")
        submit_script(os.path.join(context.workspace, q_name), "run_split.sh")
        submitted += 1

    print(f"[DFTFlow] submitted split q-point tasks: {submitted}")
    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="提交 QE split 声子 q 点任务")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    parser.add_argument("--mode", choices=("index", "coords"), required=True, help="split q 点物化模式")
    args = parser.parse_args()
    raise SystemExit(run_split_submit(args.config, args.input, args.mode))


if __name__ == "__main__":
    main()
