#!/usr/bin/env python3
import argparse
import os
import shlex
import shutil
import signal
import subprocess
import time
from typing import Optional

from public.job import add_run_submit_arguments
from qeflow.phonon.context import PhononStepContext, create_context
from qeflow.phonon.root_steps import prepare_root_scf_step
from qeflow.phonon.runner import print_stage, submit_or_run
from qeflow.phonon.status_machine import refresh_workflow_status
from qeflow.input_deck.ph import resolve_grid, write_ph_input
from qeflow.input_deck.scripts import write_root_run_script


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


def build_qf_gen_dyn0_command(context: PhononStepContext) -> str:
    """构造 qf-gen-dyn0 的 shell 命令，供本地运行或写进聚合提交脚本使用。"""
    ph_cfg = context.config["qe_phonon"]["ph"]
    nq1, nq2, nq3 = resolve_grid(ph_cfg, "qpoints", ("nq1", "nq2", "nq3"))
    outdir = context.config["qe_phonon"]["scf"].get("outdir", "./out")
    xml_path = os.path.join(context.workspace, outdir, f"{context.prefix}.save", "data-file-schema.xml")
    binary = (
        context.config["qe_phonon"].get("qf_gen_dyn0")
        or os.environ.get("QF_GEN_DYN0")
        or "qf-gen-dyn0"
    )
    resolved = shutil.which(binary) or binary
    return (
        f"{shlex.quote(resolved)} --xml {shlex.quote(xml_path)} "
        f"--nq1 {nq1} --nq2 {nq2} --nq3 {nq3} "
        f"--prefix {shlex.quote(context.prefix)} --out-dir {shlex.quote(context.workspace)}"
    )


def run_dyn0_via_rust(context: PhononStepContext) -> None:
    """用 Rust 工具 qf-gen-dyn0 生成 dyn0 + q_points.json，替代原先“跑 ph.x 仅出 dyn0 再杀掉”。"""
    binary = (
        context.config["qe_phonon"].get("qf_gen_dyn0")
        or os.environ.get("QF_GEN_DYN0")
        or "qf-gen-dyn0"
    )
    if not (os.path.isabs(binary) and os.path.exists(binary)) and shutil.which(binary) is None:
        raise RuntimeError(
            "找不到 qf-gen-dyn0 二进制：请先 `cargo build -p qf-phonon --bin qf-gen-dyn0` "
            "并把可执行文件放到 PATH，或在 input.toml 的 [qe_phonon] 配置 qf_gen_dyn0 绝对路径。"
        )
    command = build_qf_gen_dyn0_command(context)
    print_stage(2, "运行根目录 dyn0 预处理（qf-gen-dyn0）")
    subprocess.run(command, shell=True, cwd=context.workspace, check=True)
    dyn0_path = os.path.join(context.workspace, f"{context.prefix}.dyn0")
    if not (os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0):
        raise RuntimeError("qf-gen-dyn0 未生成有效的 dyn0 文件。")


def run_ph_nosplit_step(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
    dyn0_only: bool = False,
) -> int:
    context = create_context(config_file, struct_file)
    if dyn0_only:
        if run_calc:
            run_dyn0_via_rust(context)
        refresh_workflow_status(context.workspace, context.prefix)
        return 0
    prepare_root_scf_step(context)
    script = prepare_ph_nosplit_step(context, dyn0_only=False)
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
