#!/usr/bin/env bash
# 从已核验的 Wannier 实际外窗重绘；不提交或执行科学计算。
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
epwflow_bin="${EPWFLOW_BIN:-$repo_dir/target/linux-x86_64/x86_64-unknown-linux-gnu/debug/epwflow}"
calculation_dir="${1:-$repo_dir/test_auto/MgB2/local_run/001-epw-validation/026-disentanglement-window-plots}"
if [[ $# -gt 1 ]]; then
    echo '用法：bash replot_window.sh [运行目录]；目录须含 comparison-sources.json。' >&2
    exit 2
fi
if [[ ! -f "$calculation_dir/interpolation/comparison/manifest.json" ]]; then
    mkdir -p "$calculation_dir/interpolation"
    "$epwflow_bin" interpolation export-comparison \
        --source "$calculation_dir/comparison-sources.json" \
        --output "$calculation_dir/interpolation/comparison"
fi
"$epwflow_bin" collect -w "$calculation_dir"
"$epwflow_bin" plot --results "$calculation_dir/results" --formats png,svg,pdf
"$epwflow_bin" results check --results "$calculation_dir/results" --json
