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


def execute_commands(work_dir, commands, work_label=None, error_prefix="执行命令出错"):
    original_dir = os.getcwd()
    label = work_label or work_dir
    try:
        os.chdir(work_dir)
        for command in commands:
            print(f"\n正在进入 {label} 目录执行: {command}")
            subprocess.run(command, shell=True, check=True)
    except subprocess.CalledProcessError as e:
        print(f"{error_prefix}: {e}")
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
    except subprocess.CalledProcessError as e:
        print(f"提交 Slurm 任务出错: {e}")
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
