#!/usr/bin/env bash
# 只从已有比较包收集、重绘，不提交或执行 QE/EPW。
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit $?
repo_dir="$(cd "$script_dir/../../.." && pwd)" || exit $?
epw_wannier_bin="${EPW_WANNIER_BIN:-$repo_dir/target/linux-x86_64/x86_64-unknown-linux-gnu/debug/epw_wannier}"
calculation_dir="${1:-$repo_dir/test_auto/MgB2/local_run/001-epw-validation/026-disentanglement-window-plots}"
if [[ $# -gt 1 ]]; then
    echo '用法：bash replot.sh [运行目录]；EPW_WANNIER_BIN 可指定受测程序。' >&2
    exit 2
fi
"$epw_wannier_bin" --collect -w "$calculation_dir" || exit $?
"$epw_wannier_bin" --plot --results "$calculation_dir/results" --formats png,svg,pdf || exit $?
"$epw_wannier_bin" --results-check --results "$calculation_dir/results" --json || exit $?
