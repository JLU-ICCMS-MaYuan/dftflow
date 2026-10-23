#!/usr/bin/env bash
# 将显式成功制品发布到仓库模块目录；共享产品映射，不创建第二套用户安装。
script_dir=$(cd "$(dirname "$0")" && pwd) || exit $?
repo_dir=$(cd "$script_dir/.." && pwd) || exit $?
source "$script_dir/product-layout.sh" || exit $?
dftflow_products=("${dftflow_main_products[@]}")
if [[ "${1:-}" == --module ]]; then
    selected=()
    for product in "${dftflow_all_products[@]}"; do
        [[ $(dftflow_module "$product") != "${2:-}" ]] || selected+=("$product")
    done
    [[ ${#selected[@]} -gt 0 ]] || { echo "未知模块: ${2:-}" >&2; exit 1; }
    dftflow_products=("${selected[@]}")
    shift 2
elif [[ "${1:-}" == --products ]]; then
    dftflow_select_products "${2:-}" || exit 1
    shift 2
fi
source_bin=$(cd "${1:?用法: install-bin.sh [--products CSV|--module FAMILY] SOURCE_BIN [BIN_ROOT] [DYN0]}" && pwd -P) || exit $?
bin_root="${2:-$repo_dir/bin}"
dyn0="${3:-}"
case "$bin_root/" in "$HOME/.local/share/dftflow/"*) echo '旧用户安装目录已退役。' >&2; exit 2 ;; esac
mkdir -p "$bin_root" || exit $?
bin_root=$(cd "$bin_root" && pwd -P) || exit $?
case "$source_bin/" in
    "$bin_root/.build/"*) ;;
    "$bin_root/"*) echo '源目录不能位于公开安装模块内' >&2; exit 1 ;;
esac
lock_dir="$bin_root/.install-bin.lock"
mkdir "$lock_dir" || { echo '另一个发布正在进行；请检查安装锁' >&2; exit 1; }
stage=""
cleanup() {
    [[ -z "$stage" ]] || rm -rf -- "$stage"
    rmdir "$lock_dir"
}
trap cleanup EXIT
stage=$(mktemp -d "$bin_root/.install-bin.XXXXXX") || exit $?
mkdir "$stage/source" || exit $?
for product in "${dftflow_products[@]}"; do
    [[ -f "$source_bin/$product" && -x "$source_bin/$product" ]] || { echo "缺少可执行文件: $source_bin/$product" >&2; exit 1; }
    cp -p "$source_bin/$product" "$stage/source/$product" || exit $?
done
if [[ -n "$dyn0" ]]; then
    [[ -f "$dyn0" && -x "$dyn0" ]] || { echo "缺少独立工具: $dyn0" >&2; exit 1; }
    [[ " ${dftflow_products[*]} " != *' qf-gen-dyn0 '* ]] || { echo '独立工具来源重复' >&2; exit 1; }
    cp -p "$dyn0" "$stage/source/qf-gen-dyn0" || exit $?
    dftflow_products+=(qf-gen-dyn0)
fi
csv=$(IFS=,; echo "${dftflow_products[*]}")
bash "$script_dir/install-compat.sh" --products "$csv" "$stage/source" "$stage/entries" >/dev/null || exit $?
# 全部入口真正启动后才准备替换目标；错误的兼容入口也会在此拒绝。
for entry in "$stage/entries/"*; do
    "$entry" --help >/dev/null || { echo "入口无法启动: $entry" >&2; exit 1; }
done
for product in "${dftflow_products[@]}"; do
    module=$(dftflow_module "$product") || exit 1
    mkdir -p "$stage/grouped/$module" || exit $?
    cp -p "$stage/source/$product" "$stage/grouped/$module/$product" || exit $?
    while IFS= read -r entry; do
        [[ "$entry" == "$product" ]] || ln -s "$product" "$stage/grouped/$module/$entry" || exit $?
    done < <(dftflow_entries "$product")
done
for module_dir in "$stage/grouped/"*; do
    dest_dir="$bin_root/$(basename "$module_dir")"
    [[ ! -L "$dest_dir" && ( ! -e "$dest_dir" || -d "$dest_dir" ) ]] || { echo "模块目标不是普通目录: $dest_dir" >&2; exit 1; }
    for entry in "$module_dir/"*; do
        dest="$dest_dir/$(basename "$entry")"
        [[ ! -d "$dest" ]] || { echo "安装目标是目录: $dest" >&2; exit 1; }
    done
done
bash "$script_dir/check-bin-in-use.sh" "$bin_root" "${dftflow_products[@]}" || exit $?
for module_dir in "$stage/grouped/"*; do
    dest_dir="$bin_root/$(basename "$module_dir")"
    mkdir -p "$dest_dir" || exit $?
    for entry in "$module_dir/"*; do
        mv -f "$entry" "$dest_dir/$(basename "$entry")" || exit $?
    done
done
echo "已发布产品到 $bin_root：${dftflow_products[*]}"
