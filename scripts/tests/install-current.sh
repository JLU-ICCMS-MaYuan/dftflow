#!/usr/bin/env bash
# 验证固定安装重复更新及无效输入不会破坏已安装程序。
source_bin=$(cd "${1:?缺少八程序目录}" && pwd) || exit $?
repo_dir=$(cd "$(dirname "$0")/../.." && pwd) || exit $?
test_dir=$(mktemp -d) || exit $?
trap 'rm -rf -- "$test_dir"' EXIT
for iteration in 1 2; do
    if [[ "$iteration" == 2 ]]; then
        cp "$source_bin/epw_phonon" "$test_dir/install/current/bin/epwflow" || exit $?
        ln -s "$test_dir/install/current/bin/epwflow" "$test_dir/entries/epwflow" || exit $?
    fi
    bash "$repo_dir/scripts/install-current.sh" "$source_bin" "$test_dir/install" "$test_dir/entries" >/dev/null || exit $?
    [[ ! -e "$test_dir/install/current/bin/epwflow" && ! -L "$test_dir/entries/epwflow" ]] || exit $?
    [[ ! -e "$test_dir/install/versions" ]] || exit $?
    [[ $(find "$test_dir/entries" -name '.dftflow-backup-*' | wc -l) == 0 ]] || exit $?
done
for program in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
    cmp "$source_bin/$program" "$test_dir/install/current/bin/$program" || exit $?
done
mkdir "$test_dir/invalid" || exit $?
if bash "$repo_dir/scripts/install-current.sh" "$test_dir/invalid" "$test_dir/install" "$test_dir/entries" >"$test_dir/error" 2>&1; then
    echo '无效源目录应失败' >&2
    exit 1
fi
[[ ! -e "$test_dir/install/.install-current.lock" ]] || exit $?
[[ $(find "$test_dir/install" -name '.install-current.*' | wc -l) == 0 ]] || exit $?
for program in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
    cmp "$source_bin/$program" "$test_dir/install/current/bin/$program" || exit $?
done
# 升级时删除本安装器管理的旧 EPW 入口，保留不归本安装器管理的文件。
ln -s "$test_dir/install/current/bin/epwflow" "$test_dir/entries/epwflow" || exit $?
bash "$repo_dir/scripts/install-compat.sh" "$test_dir/install/current/bin" "$test_dir/entries" >/dev/null || exit $?
[[ ! -e "$test_dir/entries/epwflow" && ! -L "$test_dir/entries/epwflow" ]] || exit $?
printf 'unrelated standalone installation\n' > "$test_dir/entries/epwflow" || exit $?
bash "$repo_dir/scripts/install-compat.sh" "$test_dir/install/current/bin" "$test_dir/entries" >/dev/null || exit $?
[[ $(cat "$test_dir/entries/epwflow") == 'unrelated standalone installation' ]] || exit $?
rm "$test_dir/entries/epwflow" || exit $?
for entry in "$test_dir/entries/"*; do
    [[ $(readlink "$entry") == "$test_dir/install/current/bin/"* ]] || exit $?
    "$entry" --help >/dev/null || exit $?
done
echo '固定安装通过：重复更新、无历史副本、错误输入保护、54 入口。'
