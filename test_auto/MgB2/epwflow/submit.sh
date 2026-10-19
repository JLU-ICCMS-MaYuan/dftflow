#!/usr/bin/env bash
# 主节点入口；配置、固定输入和所有计算内容应位于 ../local_run/001-epw-validation。
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
epwflow_bin="${EPWFLOW_BIN:-$repo_dir/target/linux-x86_64/x86_64-unknown-linux-gnu/debug/epwflow}"
if [[ $# -lt 1 ]]; then
    echo '用法：bash submit.sh 配置.toml [--until qe-data|interpolation|superconductivity|all]' >&2
    exit 2
fi
config_path="$1"
shift
exec "$epwflow_bin" submit -c "$config_path" "$@"
