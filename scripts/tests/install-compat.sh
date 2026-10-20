#!/usr/bin/env bash
# 验证升级旧别名时不会删除独立工具；参数为五二进制目录。
bin_dir=$(realpath "${1:?缺少 bin 目录}") || exit $?
repo_dir=$(cd "$(dirname "$0")/../.." && pwd) || exit $?
test_dir=$(mktemp -d) || exit $?
trap 'rm -rf "$test_dir"' EXIT
mkdir -p "$test_dir/legacy" "$test_dir/independent" "$test_dir/linked" || exit $?
ln -s "$bin_dir/qeflow" "$test_dir/legacy/qeflow" || exit $?
ln -s "$bin_dir/qeflow" "$test_dir/legacy/qf-gen-dyn0" || exit $?
DFTFLOW_BACKUP_ENTRIES=1 bash "$repo_dir/scripts/install-compat.sh" "$bin_dir" "$test_dir/legacy" >/dev/null || exit $?
[[ ! -e "$test_dir/legacy/qf-gen-dyn0" && ! -L "$test_dir/legacy/qf-gen-dyn0" ]] || exit $?
backup=("$test_dir"/legacy/.dftflow-backup-*/qf-gen-dyn0)
[[ ${#backup[@]} == 1 && -L "${backup[0]}" ]] || exit $?
printf '#!/bin/bash\necho standalone\n' > "$test_dir/independent/qf-gen-dyn0" || exit $?
chmod +x "$test_dir/independent/qf-gen-dyn0" || exit $?
bash "$repo_dir/scripts/install-compat.sh" "$bin_dir" "$test_dir/independent" >/dev/null || exit $?
[[ $("$test_dir/independent/qf-gen-dyn0") == standalone ]] || exit $?
ln -s "$test_dir/independent/qf-gen-dyn0" "$test_dir/linked/qf-gen-dyn0" || exit $?
ln -s "$bin_dir/qeflow" "$test_dir/linked/qeflow" || exit $?
bash "$repo_dir/scripts/install-compat.sh" "$bin_dir" "$test_dir/linked" >/dev/null || exit $?
[[ $(readlink "$test_dir/linked/qf-gen-dyn0") == "$test_dir/independent/qf-gen-dyn0" ]] || exit $?
echo '安装迁移通过：旧别名备份移除，独立程序与独立链接保留。'
