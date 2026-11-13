#!/usr/bin/env bash
# 从唯一产品清单生成显式目标目录中的兼容入口；不再默认写入用户安装。
script_dir=$(cd "$(dirname "$0")" && pwd) || exit $?
source "$script_dir/product-layout.sh" || exit $?
dftflow_products=("${dftflow_main_products[@]}")
if [[ "${1:-}" == --products ]]; then
    dftflow_select_products "${2:-}" || exit 1
    shift 2
fi
source_bin=$(cd "${1:?用法: install-compat.sh [--products CSV] SOURCE_BIN ENTRY_DIR}" && pwd -P) || exit $?
target_dir="${2:?必须显式提供兼容入口目录；日常使用请 source scripts/env.sh}"
case "$target_dir/" in "$HOME/.local/share/dftflow/"*) echo '旧用户安装目录已退役。' >&2; exit 2 ;; esac
for product in "${dftflow_products[@]}"; do
    [[ -x "$source_bin/$product" ]] || { echo "缺少可执行文件: $source_bin/$product" >&2; exit 1; }
done
mkdir -p "$target_dir" || exit $?
target_dir=$(cd "$target_dir" && pwd -P) || exit $?
[[ "$source_bin" != "$target_dir" ]] || { echo '源与目标目录不能相同' >&2; exit 1; }
# 在写入前核对全部入口，避免遇到目录冲突时只安装一半。
for product in "${dftflow_products[@]}"; do
    while IFS= read -r entry; do
        [[ ! -d "$target_dir/$entry" || -L "$target_dir/$entry" ]] || { echo "安装目标是目录: $target_dir/$entry" >&2; exit 1; }
    done < <(dftflow_entries "$product")
done
# 兼容旧 qeflow dyn0 别名；独立工具及其独立链接保持原样。
legacy_dyn0="$target_dir/qf-gen-dyn0"
if [[ " ${dftflow_products[*]} " == *' qeflow '* && " ${dftflow_products[*]} " != *' qf-gen-dyn0 '* &&
    -L "$legacy_dyn0" && -L "$target_dir/qeflow" ]] &&
    [[ $(readlink "$legacy_dyn0") == $(readlink "$target_dir/qeflow") ]]; then
    rm -- "$legacy_dyn0" || exit $?
fi
for product in "${dftflow_products[@]}"; do
    while IFS= read -r entry; do
        dest="$target_dir/$entry"
        temp="$target_dir/.dftflow-$entry-$$"
        ln -s "$source_bin/$product" "$temp" || exit $?
        mv -f "$temp" "$dest" || exit $?
    done < <(dftflow_entries "$product")
done
echo "兼容入口已生成: $target_dir"
