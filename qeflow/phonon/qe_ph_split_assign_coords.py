#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.workflow import run_standalone_split


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO split ph.x，按不可约 q 点坐标拆分")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接运行 split 任务", "提交 split 任务")
    args = parser.parse_args()
    run_standalone_split(args.config, args.input, args.run, args.submit, "split_assign_coords")


if __name__ == "__main__":
    main()
