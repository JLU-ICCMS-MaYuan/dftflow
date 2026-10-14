#!/usr/bin/env bash
set -euo pipefail
bin_dir="${1:?用法: verify-static.sh <bin_dir>}"
work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT
for name in qeflow vaspflow scflow wannier90 substitution; do
    binary="$bin_dir/$name"
    file "$binary"
    readelf -h "$binary" > "$work_dir/header"
    grep -q 'Advanced Micro Devices X86-64' "$work_dir/header"
    readelf -l "$binary" > "$work_dir/program"
    readelf -d "$binary" > "$work_dir/dynamic"
    if grep -q INTERP "$work_dir/program" || grep -q NEEDED "$work_dir/dynamic"; then
        echo "交付文件依赖动态加载器或共享库: $binary" >&2
        exit 1
    fi
done
