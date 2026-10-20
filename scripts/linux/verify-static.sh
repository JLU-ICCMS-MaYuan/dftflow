#!/usr/bin/env bash
bin_dir="${1:?用法: verify-static.sh <bin_dir>}"
work_dir="$(mktemp -d)" || exit $?
trap 'rm -rf "$work_dir"' EXIT
for name in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
    binary="$bin_dir/$name"
    file "$binary" || exit $?
    readelf -h "$binary" > "$work_dir/header" || exit $?
    grep -q 'Advanced Micro Devices X86-64' "$work_dir/header" || exit $?
    readelf -l "$binary" > "$work_dir/program" || exit $?
    readelf -d "$binary" > "$work_dir/dynamic" || exit $?
    if grep -q INTERP "$work_dir/program" || grep -q NEEDED "$work_dir/dynamic"; then
        echo "交付文件依赖动态加载器或共享库: $binary" >&2
        exit 1
    fi
done
