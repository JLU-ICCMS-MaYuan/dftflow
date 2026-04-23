#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.workflow import run_phonon_workflow


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO 声子总工作流，不拆分 q 点")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接推进工作流", "提交当前阶段脚本")
    args = parser.parse_args()
    run_phonon_workflow(args.config, args.input, args.run, args.submit, "nosplit")


if __name__ == "__main__":
    main()
