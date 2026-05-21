#!/usr/bin/env python3
import os
import re
import shlex
import subprocess
from typing import Iterable, Optional, Sequence


PREFIX = "[qeflow]"


def info(message: str) -> None:
    print(f"{PREFIX} {message}")


def stage(stage_no: object, message: str) -> None:
    info(f"{stage_no}. {message}")


def generated(script_name: str) -> None:
    info(f"已生成: {os.path.basename(script_name)}")


def generated_scripts(script_names: Iterable[str]) -> None:
    names = [os.path.basename(name) for name in script_names]
    if names:
        info("已生成脚本: " + ", ".join(names))


def submitted(script_name: str, job_id: Optional[str] = None) -> None:
    suffix = f" -> Job {job_id}" if job_id else ""
    info(f"已提交: {os.path.basename(script_name)}{suffix}")


def next_step(message: str) -> None:
    info(f"下一步: {message}")


def resume(step_no: int, total: int, label: str) -> None:
    next_step(f"从 {step_no}/{total} {label} 开始")


def failure(step: str, reasons: Sequence[str], next_action: Optional[str] = None) -> None:
    info(f"失败: {step}")
    if reasons:
        info("原因:")
        for reason in reasons:
            print(f"{PREFIX}   - {reason}")
    if next_action:
        next_step(next_action)


def completion(message: str) -> None:
    info(f"完成: {message}")


def command_step_line(step_no: int, total: int, label: str, command: str) -> str:
    return f"{PREFIX} {step_no}/{total} {label}: {command}"


def echo_command_step(step_no: int, total: int, label: str, command: str) -> str:
    return f"echo {shlex.quote(command_step_line(step_no, total, label, command))}"


def echo_completion(message: str) -> str:
    return f"echo {shlex.quote(f'{PREFIX} 完成: {message}')}"


def submit_script(work_dir: str, script_name: str, announce: bool = True) -> Optional[str]:
    cmd = f"sbatch {shlex.quote(script_name)}"
    try:
        result = subprocess.run(
            cmd,
            shell=True,
            check=True,
            cwd=work_dir,
            text=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError as exc:
        detail = (exc.stderr or exc.stdout or "").strip()
        message = f"{work_dir} 目录中的 Slurm 提交失败: {cmd}"
        if detail:
            message = f"{message}\n{detail}"
        raise RuntimeError(message) from exc

    output = (result.stdout or "").strip()
    match = re.search(r"Submitted batch job\s+(\S+)", output)
    job_id = match.group(1) if match else None
    if announce:
        submitted(script_name, job_id)
    if announce and output and job_id is None:
        info(output)
    return job_id
