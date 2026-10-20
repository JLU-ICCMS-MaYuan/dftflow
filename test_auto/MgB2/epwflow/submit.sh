#!/usr/bin/env bash
# 主节点测试入口；每次只提交一个明确的阶段。
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit $?
repo_dir="$(cd "$script_dir/../../.." && pwd)" || exit $?
if [[ $# -lt 2 ]]; then
    echo '用法：bash submit.sh phonon|wannier|superconductivity 配置.toml [阶段选项]' >&2
    exit 2
fi
case "$1" in
    phonon) program=epw_phonon ;;
    wannier) program=epw_wannier ;;
    superconductivity) program=epw_superconductivity ;;
    *) echo '阶段必须为 phonon、wannier 或 superconductivity' >&2; exit 2 ;;
esac
config_path="$2"
shift 2
bin_dir="${EPW_BIN_DIR:-$repo_dir/bin/epwflow}"
exec "$bin_dir/$program" -s -c "$config_path" "$@"
