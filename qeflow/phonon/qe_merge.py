#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.workflow import run_standalone_merge


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO split 声子 merge 回收")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    add_run_submit_arguments(parser, "直接执行 merge", "提交 merge 脚本")
    args = parser.parse_args()
    raise SystemExit(run_standalone_merge(args.config, args.input, args.run, args.submit))


if __name__ == "__main__":
    main()
