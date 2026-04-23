#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.workflow import run_standalone_matdyn


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO matdyn.x 声子 DOS 单步任务")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接运行 matdyn.x 声子 DOS", "提交 matdyn.x 声子 DOS 脚本")
    args = parser.parse_args()
    run_standalone_matdyn(args.config, args.input, args.run, args.submit, "phdos")


if __name__ == "__main__":
    main()
