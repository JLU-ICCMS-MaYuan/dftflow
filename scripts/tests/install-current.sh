#!/usr/bin/env bash
# 退役入口必须失败且不创建用户安装；不修改真实 HOME。
repo_dir=$(cd "$(dirname "$0")/../.." && pwd) || exit $?
test_dir=$(mktemp -d) || exit $?
trap 'rm -rf -- "$test_dir"' EXIT
mkdir "$test_dir/home" || exit $?
if HOME="$test_dir/home" bash "$repo_dir/scripts/install-current.sh" >"$test_dir/message" 2>&1; then
    echo '退役安装入口不应成功' >&2; exit 1
fi
[[ ! -e "$test_dir/home/.local/share/dftflow" ]] || exit 1
grep -q '已退役' "$test_dir/message" || exit 1
echo '旧安装入口退役验证通过：明确失败且不创建用户安装。'
