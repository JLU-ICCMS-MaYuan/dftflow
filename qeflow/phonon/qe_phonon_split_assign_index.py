#!/usr/bin/env python3
import argparse
from typing import Optional

from public.job import add_run_submit_arguments
from qeflow.phonon import console
from qeflow.phonon._shared import run_shell as split_run_shell
from qeflow.phonon.ph_step import run_ph_to_dyn0
from qeflow.phonon.split_workflow import SPLIT_WORKFLOW_SPECS, create_split_phonon_workflow


submit_script = console.submit_script


def run_phonon_split_assign_index_workflow(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
) -> int:
    workflow = create_split_phonon_workflow(
        "index",
        submit_script=submit_script,
        split_run_shell=split_run_shell,
        run_dyn0=run_ph_to_dyn0,
    )
    return workflow.run(config_file, struct_file, run_calc, submit_job)


def main() -> None:
    spec = SPLIT_WORKFLOW_SPECS["index"]
    parser = argparse.ArgumentParser(description=spec.description)
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接推进工作流", "提交当前阶段脚本")
    args = parser.parse_args()
    run_phonon_split_assign_index_workflow(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
