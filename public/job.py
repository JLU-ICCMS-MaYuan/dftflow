#!/usr/bin/env python3
import os
import subprocess


def write_job_script(script_path, header, commands, default_header="#!/bin/bash"):
    with open(script_path, "w") as f:
        if header:
            f.write(header.strip() + "\n\n")
        else:
            f.write(default_header + "\n")
        for command in commands:
            f.write(command if command.endswith("\n") else command + "\n")
    os.chmod(script_path, 0o755)
    print(f"生成运行脚本: {script_path}")


def _capture_output_states(work_dir, expected_outputs):
    states = {}
    for output in expected_outputs:
        output_path = output if os.path.isabs(output) else os.path.join(work_dir, output)
        if os.path.exists(output_path):
            states[output] = os.stat(output_path).st_mtime_ns
        else:
            states[output] = None
    return states


def _validate_expected_outputs(work_dir, expected_outputs, previous_states, command):
    missing_outputs = []
    stale_outputs = []
    for output in expected_outputs:
        output_path = output if os.path.isabs(output) else os.path.join(work_dir, output)
        if not os.path.exists(output_path):
            missing_outputs.append(output)
            continue
        previous_mtime = previous_states.get(output)
        if previous_mtime is not None and os.stat(output_path).st_mtime_ns <= previous_mtime:
            stale_outputs.append(output)

    if missing_outputs or stale_outputs:
        details = []
        if missing_outputs:
            details.append("缺少输出文件: " + ", ".join(missing_outputs))
        if stale_outputs:
            details.append("输出文件未在本次运行中更新: " + ", ".join(stale_outputs))
        raise RuntimeError(
            f"命令执行后未检测到预期结果，命令: {command}；" + "；".join(details)
        )


def execute_commands(
    work_dir,
    commands,
    work_label=None,
    error_prefix="执行命令出错",
    expected_outputs=None,
):
    original_dir = os.getcwd()
    work_dir_abs = os.path.abspath(work_dir)
    label = work_label or work_dir
    try:
        os.chdir(work_dir_abs)
        for command in commands:
            print(f"\n正在进入 {label} 目录执行: {command}")
            previous_states = _capture_output_states(work_dir_abs, expected_outputs or [])
            subprocess.run(command, shell=True, check=True)
            if expected_outputs:
                _validate_expected_outputs(work_dir_abs, expected_outputs, previous_states, command)
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(
            f"{label} 目录中的命令执行失败: {command}"
        ) from exc
    finally:
        os.chdir(original_dir)
        print(f"已返回目录: {original_dir}")


def submit_script(work_dir, script_name):
    original_dir = os.getcwd()
    try:
        os.chdir(work_dir)
        cmd = f"sbatch {script_name}"
        print(f"\n正在进入 {work_dir} 目录提交: {cmd}")
        subprocess.run(cmd, shell=True, check=True)
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(f"{work_dir} 目录中的 Slurm 提交失败: {cmd}") from exc
    finally:
        os.chdir(original_dir)
        print(f"已返回目录: {original_dir}")


def handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
    if run_calc:
        execute_fn()
    elif submit_job:
        submit_script(work_dir, script_name)


def add_run_submit_arguments(parser, run_help, submit_help):
    group = parser.add_mutually_exclusive_group()
    group.add_argument("-r", "--run", action="store_true", help=run_help)
    group.add_argument("-s", "--submit", action="store_true", help=submit_help)
    return group
