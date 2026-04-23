#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.workflow import run_standalone_q2r


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO q2r.x 单步任务")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接运行 q2r.x", "提交 q2r.x 脚本")
    args = parser.parse_args()
    run_standalone_q2r(args.config, args.input, args.run, args.submit)


if __name__ == "__main__":
    main()
