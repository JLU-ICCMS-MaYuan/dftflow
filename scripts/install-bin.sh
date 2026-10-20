#!/usr/bin/env bash
# 将已有兼容制品按模块安装到仓库 bin；不改变 PATH 或历史运行绑定。
# 用法：install-bin.sh [--module epwflow] SOURCE_BIN [BIN_ROOT] [DYN0_BINARY]
selected_module=""
if [[ "${1:-}" == --module ]]; then
    [[ "${2:-}" == epwflow ]] || { echo '按模块安装当前支持 epwflow' >&2; exit 1; }
    selected_module="$2"
    shift 2
fi
script_dir=$(cd "$(dirname "$0")" && pwd) || exit $?
repo_dir=$(cd "$script_dir/.." && pwd) || exit $?
source_bin=$(cd "${1:?用法: install-bin.sh SOURCE_BIN [BIN_ROOT] [DYN0_BINARY]}" && pwd -P) || exit $?
bin_root="${2:-$repo_dir/bin}"
dyn0="${3:-}"
mkdir -p "$bin_root" || exit $?
bin_root=$(cd "$bin_root" && pwd -P) || exit $?
case "$source_bin/" in "$bin_root/"*) echo '源目录不能位于安装目录内' >&2; exit 1 ;; esac
lock_dir="$bin_root/.install-bin.lock"
mkdir "$lock_dir" || { echo '另一个分组安装正在进行；请检查安装锁' >&2; exit 1; }
stage=""
cleanup() {
    [[ -z "$stage" ]] || rm -rf -- "$stage"
    rmdir "$lock_dir" || exit $?
}
trap cleanup EXIT
stage=$(mktemp -d "$bin_root/.install-bin.XXXXXX") || exit $?
mkdir "$stage/source" || exit $?
programs=(qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity)
if [[ -n "$selected_module" ]]; then programs=(epw_phonon epw_wannier epw_superconductivity); fi
for program in "${programs[@]}"; do
    [[ -f "$source_bin/$program" && -x "$source_bin/$program" ]] || { echo "缺少可执行文件: $source_bin/$program" >&2; exit 1; }
    cp -p "$source_bin/$program" "$stage/source/$program" || exit $?
done
# 入口映射和启动检查复用现有实现，避免维护第二套命令名清单。
if [[ -z "$selected_module" ]]; then
    bash "$script_dir/verify-entries.sh" "$stage/source" >/dev/null || exit $?
    bash "$script_dir/install-compat.sh" "$stage/source" "$stage/entries" >/dev/null || exit $?
else
    mkdir "$stage/entries" || exit $?
    for program in "${programs[@]}"; do
        "$stage/source/$program" --help >/dev/null || exit $?
        ln -s "../source/$program" "$stage/entries/$program" || exit $?
    done
fi
for entry in "$stage/entries/"*; do
    program=$(basename "$(readlink "$entry")") || exit $?
    case "$program" in
        epw_phonon|epw_wannier|epw_superconductivity) module=epwflow ;;
        wannier90) module=wannierflow ;;
        substitution) module=substitutionflow ;;
        qeflow|vaspflow|scflow) module="$program" ;;
        *) echo "未知程序: $program" >&2; exit 1 ;;
    esac
    mkdir -p "$stage/grouped/$module" || exit $?
    name=$(basename "$entry") || exit $?
    if [[ "$name" == "$program" ]]; then
        mv "$stage/source/$program" "$stage/grouped/$module/$program" || exit $?
    else
        ln -s "$program" "$stage/grouped/$module/$name" || exit $?
    fi
done
if [[ -n "$dyn0" ]]; then
    [[ -f "$dyn0" && -x "$dyn0" ]] || { echo "缺少独立可执行文件: $dyn0" >&2; exit 1; }
    mkdir "$stage/grouped/tools" || exit $?
    cp -p "$dyn0" "$stage/grouped/tools/qf-gen-dyn0" || exit $?
    "$stage/grouped/tools/qf-gen-dyn0" --help >/dev/null || exit $?
fi
# 全部检查完成后才替换安装文件，拒绝模块目录链接和同名目录。
for module_dir in "$stage/grouped/"*; do
    module=$(basename "$module_dir") || exit $?
    dest_dir="$bin_root/$module"
    if [[ -L "$dest_dir" || ( -e "$dest_dir" && ! -d "$dest_dir" ) ]]; then
        echo "模块目标不是普通目录: $dest_dir" >&2; exit 1
    fi
    for entry in "$module_dir/"*; do
        dest="$dest_dir/$(basename "$entry")" || exit $?
        [[ ! -d "$dest" ]] || { echo "安装目标是目录: $dest" >&2; exit 1; }
    done
done
for module_dir in "$stage/grouped/"*; do
    module=$(basename "$module_dir") || exit $?
    mkdir -p "$bin_root/$module" || exit $?
    for entry in "$module_dir/"*; do
        mv -f "$entry" "$bin_root/$module/$(basename "$entry")" || exit $?
    done
done
echo "已更新分组安装目录：$bin_root（${selected_module:-全部模块}；独立 dyn0 按显式参数安装）"
