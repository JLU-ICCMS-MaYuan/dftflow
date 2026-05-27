#!/usr/bin/env python3
import argparse
import os
import signal
import subprocess
import time
from typing import Optional

from public.job import add_run_submit_arguments
from qeflow.phonon.context import PhononStepContext, create_context
from qeflow.phonon.root_steps import prepare_root_scf_step
from qeflow.phonon.runner import print_stage, submit_or_run
from qeflow.phonon.status_machine import refresh_workflow_status
from qeflow.qe_input_deck.ph import write_ph_input
from qeflow.qe_input_deck.scripts import write_root_run_script


def prepare_ph_nosplit_step(context: PhononStepContext, dyn0_only: bool = False) -> str:
    write_ph_input(context.workspace, context.config, context.prefix, dyn0_only=dyn0_only)
    return write_root_run_script(
        context.workspace,
        "run_ph.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['ph']} < ph.in > ph.out"],
        announce=False,
    )


def run_ph_to_dyn0(workspace: str, ph_command: str, dyn0_path: str, output_path: str) -> None:
    # dyn0 预处理只针对根目录 ph.in；stderr 也重定向到 ph.out，避免主动终止时刷屏。
    command = f"{ph_command} < ph.in > ph.out 2>&1"
    process = subprocess.Popen(
        command,
        shell=True,
        cwd=workspace,
        preexec_fn=os.setsid,
    )
    terminated_after_dyn0 = False
    try:
        while True:
            if os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0:
                os.killpg(os.getpgid(process.pid), signal.SIGTERM)
                process.wait(timeout=10)
                terminated_after_dyn0 = True
                return
            if os.path.exists(output_path):
                with open(output_path, "r", encoding="utf-8", errors="ignore") as handle:
                    content = handle.read().lower()
                if "%%%%%%%%%%%%%%" in content or "error in routine" in content:
                    process.wait(timeout=10)
                    raise RuntimeError("ph.x 在生成 dyn0 前已报错，终止 dyn0 模式。")
            if process.poll() is not None:
                if os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0:
                    return
                raise RuntimeError("ph.x 已退出，但未生成有效的 dyn0 文件。")
            time.sleep(0.5)
    finally:
        if not terminated_after_dyn0 and process.poll() is None:
            os.killpg(os.getpgid(process.pid), signal.SIGTERM)
            process.wait(timeout=10)


def run_ph_nosplit_step(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
    dyn0_only: bool = False,
) -> int:
    context = create_context(config_file, struct_file)
    prepare_root_scf_step(context)
    script = prepare_ph_nosplit_step(context, dyn0_only=dyn0_only)
    if dyn0_only and run_calc:
        print_stage(1, "运行根目录 dyn0 预处理")
        run_ph_to_dyn0(
            context.workspace,
            context.commands["ph"],
            os.path.join(context.workspace, f"{context.prefix}.dyn0"),
            os.path.join(context.workspace, "ph.out"),
        )
    else:
        if run_calc:
            print_stage(1, "运行根目录 PH")
        elif submit_job:
            print_stage(1, "提交根目录 PH")
        submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO ph.x 单步任务")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    parser.add_argument("--dyn0", action="store_true", help="只运行到生成 dyn0 为止")
    add_run_submit_arguments(parser, "直接运行 ph.x", "提交 ph.x 脚本")
    args = parser.parse_args()
    run_ph_nosplit_step(args.config, args.input, args.run, args.submit, args.dyn0)


if __name__ == "__main__":
    main()
