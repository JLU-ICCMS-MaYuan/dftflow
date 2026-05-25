#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from public.job import add_run_submit_arguments
from qeflow.phonon import console
from qeflow.phonon._shared import (
    create_context,
    prepare_root_scf_step,
    print_stage,
    refresh_workflow_status,
    root_step_scripts,
    run_root_scripts,
    write_aggregate_script,
)
from qeflow.phonon.ph_step import prepare_ph_nosplit_step
from qeflow.phonon.process_phonon import PROCESS_SCRIPT, prepare_process_phonon_step, run_process_phonon_steps
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_q2r import prepare_q2r_step

submit_script = console.submit_script


def run_phonon_nosplit_workflow(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
) -> int:
    context = create_context(config_file, struct_file, workflow_name="qe_phonon_nosplit")
    prepare_root_scf_step(context)
    prepare_ph_nosplit_step(context, dyn0_only=False)
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)
    console.generated_scripts(["run_scf.sh", "run_ph.sh", "run_q2r.sh", "run_matdyn_phband.sh", "run_matdyn_phdos.sh"])

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    remaining_scripts = root_step_scripts(workflow, include_ph=True)
    if run_calc:
        pre_scripts = [script for script in remaining_scripts if script in {"run_scf.sh", "run_ph.sh"}]
        run_root_scripts(
            context,
            pre_scripts,
            labels={
                "run_scf.sh": "运行根目录 SCF",
                "run_ph.sh": "运行根目录 PH",
            },
        )
        run_process_phonon_steps(context, include_plot=True)
    elif submit_job and remaining_scripts:
        pre_scripts = [script for script in remaining_scripts if script in {"run_scf.sh", "run_ph.sh"}]
        process_needed = any(script in set(remaining_scripts) for script in {"run_q2r.sh", "run_matdyn_phband.sh", "run_matdyn_phdos.sh"})
        if process_needed:
            prepare_process_phonon_step(context)
        aggregate = write_aggregate_script(
            context,
            "submit_phonon_workflow.sh",
            [*pre_scripts, *([PROCESS_SCRIPT] if process_needed else [])],
        )
        console.generated(os.path.basename(aggregate))
        submit_script(context.workspace, os.path.basename(aggregate))
        console.next_step("查看对应 slurm 输出，等待声子工作流完成")
    else:
        console.next_step("声子工作流已完成或没有待提交步骤")
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO 声子总工作流，不拆分 q 点")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接推进工作流", "提交当前阶段脚本")
    args = parser.parse_args()
    run_phonon_nosplit_workflow(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
