#!/usr/bin/env bash
# 使用真实已构建程序验证分组安装及失败保护；不运行科学计算。
source_bin=$(cd "${1:?缺少八程序目录}" && pwd) || exit $?
dyn0="${2:?缺少独立 dyn0 程序}"
repo_dir=$(cd "$(dirname "$0")/../.." && pwd) || exit $?
test_dir=$(mktemp -d) || exit $?
trap 'rm -rf -- "$test_dir"' EXIT
dest="$test_dir/bin"
install_grouped() { bash "$repo_dir/scripts/install-bin.sh" "$1" "$dest" "${2:-}" >/dev/null; }
verify() {
    local program module count=0 entry
    for program in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
        case "$program" in
            epw_*) module=epwflow ;;
            wannier90) module=wannierflow ;;
            substitution) module=substitutionflow ;;
            *) module="$program" ;;
        esac
        cmp "$source_bin/$program" "$dest/$module/$program" || exit $?
    done
    cmp "$dyn0" "$dest/tools/qf-gen-dyn0" || exit $?
    for entry in "$dest/"*/*; do
        "$entry" --help >/dev/null || exit $?
        count=$((count + 1))
    done
    [[ "$count" == 55 ]] || exit $?
    [[ ! -e "$dest/.install-bin.lock" ]] || exit $?
    [[ $(find "$dest" -name '.install-bin.*' | wc -l) -eq 0 ]] || exit $?
}
install_grouped "$source_bin" "$dyn0" || exit $?
verify || exit $?
mkdir -p "$test_dir/module-bin/qeflow" || exit $?
printf '保留其他模块\n' > "$test_dir/module-bin/qeflow/unchanged" || exit $?
bash "$repo_dir/scripts/install-bin.sh" --module epwflow "$source_bin" "$test_dir/module-bin" >/dev/null || exit $?
for program in epw_phonon epw_wannier epw_superconductivity; do
    cmp "$source_bin/$program" "$test_dir/module-bin/epwflow/$program" || exit $?
done
[[ $(cat "$test_dir/module-bin/qeflow/unchanged") == '保留其他模块' ]] || exit $?
[[ $(find "$test_dir/module-bin/epwflow" -type f | wc -l) -eq 3 ]] || exit $?
install_grouped "$source_bin" || exit $?
verify || exit $?
mkdir "$test_dir/invalid" || exit $?
if install_grouped "$test_dir/invalid" 2>"$test_dir/error"; then
    echo '缺失源应安装失败' >&2; exit 1
fi
verify || exit $?
mkdir "$test_dir/broken" || exit $?
for entry in "$source_bin/"*; do ln -s "$entry" "$test_dir/broken/$(basename "$entry")" || exit $?; done
rm "$test_dir/broken/epw_wannier" || exit $?
printf '#!/bin/sh\nexit 1\n' > "$test_dir/broken/epw_wannier" || exit $?
chmod +x "$test_dir/broken/epw_wannier" || exit $?
if install_grouped "$test_dir/broken" 2>"$test_dir/error"; then
    echo '不可启动的程序应安装失败' >&2; exit 1
fi
verify || exit $?
rm "$dest/qeflow/qe_scf" || exit $?
mkdir "$dest/qeflow/qe_scf" || exit $?
if install_grouped "$source_bin" 2>"$test_dir/error"; then
    echo '目标目录冲突应安装失败' >&2; exit 1
fi
rmdir "$dest/qeflow/qe_scf" || exit $?
ln -s qeflow "$dest/qeflow/qe_scf" || exit $?
verify || exit $?
mv "$dest/epwflow" "$test_dir/epwflow" || exit $?
ln -s "$test_dir/epwflow" "$dest/epwflow" || exit $?
if install_grouped "$source_bin" 2>"$test_dir/error"; then
    echo '模块目录链接应安装失败' >&2; exit 1
fi
rm "$dest/epwflow" || exit $?
mv "$test_dir/epwflow" "$dest/epwflow" || exit $?
verify || exit $?
if install_grouped "$dest/epwflow" 2>"$test_dir/error"; then
    echo '来源位于目标内应安装失败' >&2; exit 1
fi
verify || exit $?
echo '分组安装通过：重复更新、55 入口、缺失／损坏源、目标冲突及源目标重叠保护。'
