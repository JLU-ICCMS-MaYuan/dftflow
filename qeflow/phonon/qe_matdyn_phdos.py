#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from public.job import add_run_submit_arguments, submit_script
from qeflow.phonon._shared import (
    PhononStepContext,
    create_context,
    print_stage,
    refresh_workflow_status,
    run_shell,
)
from qeflow.qe_input.builders import write_matdyn_phdos_input, write_root_run_script


def prepare_matdyn_phdos_step(context: PhononStepContext) -> str:
    write_matdyn_phdos_input(context.workspace, context.config, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_matdyn_phdos.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['matdyn']} < matdyn_phdos.in > matdyn_phdos.out"],
    )


def run_matdyn_phdos_step(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
) -> int:
    context = create_context(config_file, struct_file)
    script = prepare_matdyn_phdos_step(context)
    if run_calc:
        print_stage(1, "运行 matdyn_phdos")
        run_shell(f"bash {os.path.basename(script)}", context.workspace)
        from qeflow.phonon.process_phonon import collect_phdos_outputs

        print_stage(2, "归档 phdos")
        collect_phdos_outputs(context)
    elif submit_job:
        from qeflow.phonon.process_phonon import prepare_matdyn_phdos_collect_step

        print_stage(1, "提交 matdyn_phdos")
        submit_script(context.workspace, os.path.basename(prepare_matdyn_phdos_collect_step(context)))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO matdyn.x 声子 DOS 单步任务")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接运行 matdyn.x 声子 DOS", "提交 matdyn.x 声子 DOS 脚本")
    args = parser.parse_args()
    run_matdyn_phdos_step(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
