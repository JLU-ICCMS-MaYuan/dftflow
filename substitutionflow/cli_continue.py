#!/usr/bin/env python3
import argparse
import sys

from public.job import add_run_submit_arguments
from substitutionflow.workflow import continue_workflow, rewrite_workflow_states


def main():
    parser = argparse.ArgumentParser(description="检查并续算 substitutionflow 工作流。")
    parser.add_argument("-o", "--output-dir", required=True, help="工作流输出根目录。")
    parser.add_argument("-c", "--config", required=True, help="配置文件路径；所有模式都以该文件作为 case/input.toml 的唯一真源。")
    parser.add_argument(
        "-rws",
        "--rewrite-states",
        action="store_true",
        help="先批量重写已有 case/input.toml，再刷新状态、同步 qe_phonon_* 的 POSCAR、重生 submit_group_*.sh，并把全部 error 重置为 unstarted。",
    )
    add_run_submit_arguments(parser, "从现有状态文件的第一个非 finished 步骤开始顺序续算", "从现有状态文件的第一个非 finished 步骤开始按分组提交缺失任务脚本")
    args = parser.parse_args()

    if args.rewrite_states and (args.run or args.submit):
        parser.error("--rewrite-states/-rws 不能与 -r/--run 或 -s/--submit 同时使用。")

    if args.rewrite_states:
        result = rewrite_workflow_states(args.output_dir, config_file=args.config)
    else:
        result = continue_workflow(args.output_dir, run_calc=args.run, submit=args.submit, config_file=args.config)
    print(result)


if __name__ == "__main__":
    sys.exit(main())
