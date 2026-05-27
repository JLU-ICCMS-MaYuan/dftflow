#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments, handle_job_action
from qeflow.qe_input.calculation_setup import create_qe_calculation_setup


def run_qe_calculation_cli(kind: str, description: str, action_handler=handle_job_action) -> None:
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")

    args = parser.parse_args()
    setup = create_qe_calculation_setup(kind, config_file=args.config, struct_file=args.input)
    setup.prepare()
    action_handler(args.run, args.submit, setup.work_dir, "run_qe.sh", setup.execute)
