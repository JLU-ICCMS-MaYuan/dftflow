#!/usr/bin/env bash
# 验证升级旧别名时不会删除独立工具；参数为五二进制目录。
set -euo pipefail
bin_dir=$(realpath "${1:?缺少 bin 目录}")
repo_dir=$(cd "$(dirname "$0")/../.." && pwd)
test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT
mkdir -p "$test_dir/legacy" "$test_dir/independent" "$test_dir/linked"
ln -s "$bin_dir/qeflow" "$test_dir/legacy/qeflow"
ln -s "$bin_dir/qeflow" "$test_dir/legacy/qf-gen-dyn0"
DFTFLOW_BACKUP_ENTRIES=1 bash "$repo_dir/scripts/install-compat.sh" "$bin_dir" "$test_dir/legacy" >/dev/null
[[ ! -e "$test_dir/legacy/qf-gen-dyn0" && ! -L "$test_dir/legacy/qf-gen-dyn0" ]]
backup=("$test_dir"/legacy/.dftflow-backup-*/qf-gen-dyn0)
[[ ${#backup[@]} == 1 && -L "${backup[0]}" ]]
printf '#!/bin/bash\necho standalone\n' > "$test_dir/independent/qf-gen-dyn0"
chmod +x "$test_dir/independent/qf-gen-dyn0"
bash "$repo_dir/scripts/install-compat.sh" "$bin_dir" "$test_dir/independent" >/dev/null
[[ $("$test_dir/independent/qf-gen-dyn0") == standalone ]]
ln -s "$test_dir/independent/qf-gen-dyn0" "$test_dir/linked/qf-gen-dyn0"
ln -s "$bin_dir/qeflow" "$test_dir/linked/qeflow"
bash "$repo_dir/scripts/install-compat.sh" "$bin_dir" "$test_dir/linked" >/dev/null
[[ $(readlink "$test_dir/linked/qf-gen-dyn0") == "$test_dir/independent/qf-gen-dyn0" ]]
echo '安装迁移通过：旧别名备份移除，独立程序与独立链接保留。'
