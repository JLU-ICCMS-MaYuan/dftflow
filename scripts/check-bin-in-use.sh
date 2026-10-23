#!/usr/bin/env bash
# 发布前检查选中产品的本机进程和 Slurm 脚本引用；不停止或修改作业。
bin_root="${1:?缺少 bin 根目录}"
shift
script_dir=$(cd "$(dirname "$0")" && pwd) || exit $?
source "$script_dir/product-layout.sh" || exit $?
targets=()
for product in "$@"; do
    module=$(dftflow_module "$product") || exit 1
    targets+=("$bin_root/$module/$product")
done
for proc in /proc/[0-9]*/exe; do
    [[ -O "${proc%/exe}" && -e "$proc" ]] || continue
    executable=$(readlink "$proc" 2>/dev/null) || continue
    for target in "${targets[@]}"; do
        if [[ "$executable" == "$target" || "$executable" == "$target (deleted)" ]]; then
            echo "产品正在运行，保留现有程序，稍后重新构建发布: $proc -> $executable" >&2
            exit 1
        fi
    done
done
if command -v squeue >/dev/null; then
    jobs=$(squeue -u "$(id -un)" -h -o '%i|%o') || { echo '无法核验 Slurm 活动作业，停止发布。' >&2; exit 1; }
    while IFS='|' read -r job command; do
        [[ -n "$job" && -f "$command" ]] || continue
        for target in "${targets[@]}"; do
            if grep -Fq -- "$target" "$command"; then
                echo "Slurm 作业 $job 仍引用待更新产品: $target（$command）" >&2
                exit 1
            fi
        done
    done <<< "$jobs"
fi
