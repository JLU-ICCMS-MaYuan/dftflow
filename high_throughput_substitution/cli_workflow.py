#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from high_throughput_substitution.workflow import run_workflow


def main():
    parser = argparse.ArgumentParser(description="XA/XB 占位结构高通量替换工作流。")
    parser.add_argument("-i", "--input", required=True, help="输入占位结构文件。")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径。")
    parser.add_argument("-o", "--output-dir", required=True, help="输出根目录。")
    add_run_submit_arguments(parser, "生成后顺序执行全部任务", "生成后提交总脚本")
    args = parser.parse_args()

    result = run_workflow(args.input, args.config, args.output_dir, run_calc=args.run, submit=args.submit)
    print(result)


if __name__ == "__main__":
    main()

