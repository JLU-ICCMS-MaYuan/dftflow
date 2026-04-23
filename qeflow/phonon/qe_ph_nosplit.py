#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.workflow import run_standalone_ph


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO ph.x 单步任务")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    parser.add_argument("--dyn0", action="store_true", help="只运行到生成 dyn0 为止")
    add_run_submit_arguments(parser, "直接运行 ph.x", "提交 ph.x 脚本")
    args = parser.parse_args()
    run_standalone_ph(args.config, args.input, args.run, args.submit, args.dyn0)


if __name__ == "__main__":
    main()
