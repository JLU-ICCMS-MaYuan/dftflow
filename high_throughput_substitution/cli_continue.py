#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from high_throughput_substitution.workflow import continue_workflow


def main():
    parser = argparse.ArgumentParser(description="检查并续算 high_throughput_substitution 工作流。")
    parser.add_argument("-o", "--output-dir", required=True, help="工作流输出根目录。")
    add_run_submit_arguments(parser, "检查后顺序续算缺失任务", "检查后提交缺失任务脚本")
    args = parser.parse_args()

    result = continue_workflow(args.output_dir, run_calc=args.run, submit=args.submit)
    print(result)


if __name__ == "__main__":
    main()
