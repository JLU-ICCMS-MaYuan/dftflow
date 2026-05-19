#!/usr/bin/env python3
import argparse
import os
from typing import Optional

from public.job import add_run_submit_arguments, submit_script
from qeflow.phonon._shared import (
    create_context,
    prepare_root_scf_step,
    print_stage,
    print_status,
    refresh_workflow_status,
    root_has_running_step,
    root_step_scripts,
    run_root_scripts,
    write_aggregate_script,
)
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.ph_step import prepare_ph_nosplit_step
from qeflow.phonon.qe_q2r import prepare_q2r_step


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

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    remaining_scripts = root_step_scripts(workflow, include_ph=True)
    if run_calc:
        run_root_scripts(
            context,
            remaining_scripts,
            labels={
                "run_scf.sh": "运行根目录 SCF",
                "run_ph.sh": "运行根目录 PH",
                "run_q2r.sh": "运行 q2r",
                "run_matdyn_phband.sh": "运行 matdyn_phband",
                "run_matdyn_phdos.sh": "运行 matdyn_phdos",
            },
        )
    elif submit_job and remaining_scripts and not root_has_running_step(workflow, include_ph=True):
        print_stage(1, "提交整链剩余步骤")
        aggregate = write_aggregate_script(context, "submit_phonon_workflow.sh", remaining_scripts)
        submit_script(context.workspace, os.path.basename(aggregate))
    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
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
