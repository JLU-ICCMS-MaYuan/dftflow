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
        "-rfs",
        "--refresh-status",
        action="store_true",
        help="按实际完成情况刷新 status 文件，并生成对应的 submit_group_*.sh；这是默认标准流程的显式入口。",
    )
    parser.add_argument(
        "-rws",
        "--rewrite-states",
        action="store_true",
        help="把全部 error 重写为 unstarted，再生成对应的 submit_group_*.sh；可与 -r/--run 或 -s/--submit 组合使用。",
    )
    add_run_submit_arguments(parser, "从现有状态文件的第一个非 finished 步骤开始顺序续算", "从现有状态文件的第一个非 finished 步骤开始按分组提交缺失任务脚本")
    args = parser.parse_args()

    if args.refresh_status and args.rewrite_states:
        parser.error("-rfs/--refresh-status 不能与 -rws/--rewrite-states 同时使用。")

    if args.rewrite_states and not (args.run or args.submit):
        result = rewrite_workflow_states(args.output_dir, config_file=args.config)
    else:
        result = continue_workflow(
            args.output_dir,
            run_calc=args.run,
            submit=args.submit,
            config_file=args.config,
            rewrite_states=args.rewrite_states,
        )
    print(result)


if __name__ == "__main__":
    sys.exit(main())
