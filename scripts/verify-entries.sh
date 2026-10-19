#!/usr/bin/env bash
# 在隔离安装目录验证所有兼容入口，避免只测少数代表命令。
set -euo pipefail
bin_dir="${1:?用法: verify-entries.sh <bin_dir>}"
script_dir="$(cd "$(dirname "$0")" && pwd)"
work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT
bash "$script_dir/install-compat.sh" "$bin_dir" "$work_dir/bin"
count=0
for entry in "$work_dir"/bin/*; do
    "$entry" --help > "$work_dir/help" 2>&1 || { cat "$work_dir/help"; exit 1; }
    count=$((count + 1))
done
[[ "$count" == 54 ]]
printf '全部 %s 个入口通过（48 旧名 + 3 个家族名 + 3 个 EPW 阶段程序）。\n' "$count"
