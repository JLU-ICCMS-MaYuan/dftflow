#!/usr/bin/env bash
# 从已核验的 Wannier 实际外窗重绘；不提交或执行科学计算。
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
epw_wannier_bin="${EPW_WANNIER_BIN:-$repo_dir/target/linux-x86_64/x86_64-unknown-linux-gnu/debug/epw_wannier}"
calculation_dir="${1:-$repo_dir/test_auto/MgB2/local_run/001-epw-validation/026-disentanglement-window-plots}"
if [[ $# -gt 1 ]]; then
    echo '用法：bash replot_window.sh [运行目录]；目录须含 comparison-sources.json。' >&2
    exit 2
fi
if [[ ! -f "$calculation_dir/interpolation/comparison/manifest.json" ]]; then
    mkdir -p "$calculation_dir/interpolation"
    "$epw_wannier_bin" --export-comparison "$calculation_dir/comparison-sources.json" \
        --output "$calculation_dir/interpolation/comparison"
fi
"$epw_wannier_bin" --collect -w "$calculation_dir"
"$epw_wannier_bin" --plot --results "$calculation_dir/results" --formats png,svg,pdf
"$epw_wannier_bin" --results-check --results "$calculation_dir/results" --json
