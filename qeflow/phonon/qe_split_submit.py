#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from qeflow.phonon import console
from qeflow.phonon.context import create_context
from qeflow.phonon.q_index_map import iter_q_directory_names
from qeflow.phonon.status_machine import (
    dyn0_state,
    ph_tasks,
    print_resume_point,
    refresh_workflow_status,
)
from qeflow.phonon.split_prepare import prepare_split_assign_coords_step, prepare_split_assign_index_step

submit_script = console.submit_script


def _submit_q_point(work_dir: str) -> Optional[str]:
    if submit_script is console.submit_script:
        return submit_script(work_dir, "run_split.sh", announce=False)
    return submit_script(work_dir, "run_split.sh")


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
        console.failure(
            "3/11 split q 点提交",
            [f"dyn0 未完成: {dyn0_state(workflow)}"],
            "等待 dyn0 完成后重新执行 qe_phonon_split_index -s 或 qe_phonon_split_coords -s",
        )
        print_resume_point(workflow)
        return 1

    _prepare_split_by_mode(context, mode)
    console.generated_scripts(["run_split.sh"])
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    tasks = ph_tasks(workflow)

    submitted_names = []
    skipped_finished = []
    job_ids = []
    for q_name in iter_q_directory_names(context.workspace):
        state = tasks.get(q_name, "unstarted")
        if state == "finished":
            skipped_finished.append(q_name)
            continue
        job_id = _submit_q_point(os.path.join(context.workspace, q_name))
        submitted_names.append(q_name)
        if job_id:
            job_ids.append(f"{q_name}={job_id}")

    parts = []
    if submitted_names:
        parts.append("提交 " + ", ".join(submitted_names))
    if skipped_finished:
        parts.append("跳过 finished: " + ", ".join(skipped_finished))
    console.info("split q 点: " + ("; ".join(parts) if parts else "无待提交任务"))
    if job_ids:
        console.info("split jobs: " + ", ".join(job_ids))
    print_resume_point(refresh_workflow_status(context.workspace, context.prefix))
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
