#!/usr/bin/env python3
import argparse
from typing import Optional

from qeflow.phonon import console
from qeflow.phonon.context import create_context
from qeflow.phonon.split_merge import (
    MergeResult,
    SplitMergeStrategy,
    merge_split_results,
    merge_split_results_for_coords,
    merge_split_results_for_index,
)
from qeflow.phonon.status_machine import refresh_workflow_status


def _print_merge_result(prefix: str, result: MergeResult) -> None:
    if result.success:
        console.info("merge 检查: dyn 文件最低前 5 个频率")
        for q_index, values in result.dyn_summaries:
            formatted = ", ".join(f"{value:.6f}" for value in values) if values else "无可解析频率"
            console.info(f"  {prefix}.dyn{q_index}: {formatted}")
        return
    console.failure(
        "4/11 merge",
        result.errors,
        "修复上述 split q 点结果后，重新执行 qe_phonon_split_index -s 或 qe_phonon_split_coords -s",
    )


def run_merge_step(
    config_file: str,
    struct_file: Optional[str],
    workflow_name: Optional[str] = None,
) -> int:
    context = create_context(config_file, struct_file, workflow_name=workflow_name, prepare_inputs=False)
    result = merge_split_results(context.workspace, context.prefix)
    _print_merge_result(context.prefix, result)
    refresh_workflow_status(context.workspace, context.prefix)
    return 0 if result.success else 1


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO split 声子 merge 回收")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    args = parser.parse_args()
    raise SystemExit(run_merge_step(args.config, args.input))


if __name__ == "__main__":
    main()
