#!/usr/bin/env bash
# 验证固定安装重复更新及无效输入不会破坏已安装程序。
set -euo pipefail
source_bin=$(cd "${1:?缺少五程序目录}" && pwd)
repo_dir=$(cd "$(dirname "$0")/../.." && pwd)
test_dir=$(mktemp -d)
trap 'rm -rf -- "$test_dir"' EXIT
for iteration in 1 2; do
    bash "$repo_dir/scripts/install-current.sh" "$source_bin" "$test_dir/install" "$test_dir/entries" >/dev/null
    [[ ! -e "$test_dir/install/versions" ]]
    [[ $(find "$test_dir/entries" -name '.dftflow-backup-*' | wc -l) == 0 ]]
done
for program in qeflow vaspflow scflow wannier90 substitution epwflow; do
    cmp "$source_bin/$program" "$test_dir/install/current/bin/$program"
done
mkdir "$test_dir/invalid"
if bash "$repo_dir/scripts/install-current.sh" "$test_dir/invalid" "$test_dir/install" "$test_dir/entries" >"$test_dir/error" 2>&1; then
    echo '无效源目录应失败' >&2
    exit 1
fi
[[ ! -e "$test_dir/install/.install-current.lock" ]]
[[ $(find "$test_dir/install" -name '.install-current.*' | wc -l) == 0 ]]
for program in qeflow vaspflow scflow wannier90 substitution epwflow; do
    cmp "$source_bin/$program" "$test_dir/install/current/bin/$program"
done
for entry in "$test_dir/entries/"*; do
    [[ $(readlink "$entry") == "$test_dir/install/current/bin/"* ]]
    "$entry" --help >/dev/null
done
echo '固定安装通过：重复更新、无历史副本、错误输入保护、52 入口。'
