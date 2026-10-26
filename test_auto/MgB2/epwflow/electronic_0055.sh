#!/usr/bin/env bash
# 在已登记 Slurm 分配内执行；测试程序及缓存保持在仓库 bin。
repo_dir="$(cd "$(dirname "$0")/../../.." && pwd)" || exit $?
[[ -n "${SLURM_JOB_ID:-}" ]] || { echo "需要 Slurm 分配" >&2; exit 2; }
[[ $# == 2 ]] || { echo "用法: $0 已构建的验收程序 新验收目录" >&2; exit 2; }
source "/data/home/mayuan/intel-2024/oneapi/intel2024_setvars.sh"
export DFTFLOW_0055_SOURCE="$repo_dir/test_auto/MgB2/local_run/012-stage-results"
export DFTFLOW_0055_OUTPUT="$2"
export OMP_NUM_THREADS=1
test_loader="$repo_dir/bin/.native/linux-x86_64/lib/ld-linux-x86-64.so.2"
if [[ -x "$test_loader" ]]; then
    "$test_loader" --library-path "$repo_dir/bin/.native/linux-x86_64/lib" "$1" --ignored --exact real_electronic_dual_windows_and_pdos --nocapture
else
    "$1" --ignored --exact real_electronic_dual_windows_and_pdos --nocapture
fi
