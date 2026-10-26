#!/usr/bin/env bash
# 为共享文件系统上的旧 glibc 计算节点打包本机构建制品；不改变全局环境。
# 用法：[--module epwflow] SOURCE_BIN_DIR DEST_BIN_DIR NATIVE_PREFIX
script_dir=$(cd "$(dirname "$0")/.." && pwd) || exit $?
source "$script_dir/product-layout.sh" || exit $?
programs=("${dftflow_main_products[@]}")
if [[ "${1:-}" == --module ]]; then
    [[ "${2:-}" == epwflow ]] || { echo '按模块打包当前支持 epwflow' >&2; exit 1; }
    programs=(epw_phonon epw_wannier epw_superconductivity epw_clear)
    shift 2
elif [[ "${1:-}" == --products ]]; then
    dftflow_select_products "${2:-}" || exit 1
    programs=("${dftflow_products[@]}")
    shift 2
fi
source_bin=$(realpath "${1:?缺少源码构建的 bin 目录}") || exit $?
for name in "${programs[@]}"; do
    [[ -f "$source_bin/$name" && -x "$source_bin/$name" ]] || { echo "缺少可执行文件: $source_bin/$name" >&2; exit 1; }
done
for destination in "${2:?缺少版本 bin 目录}" "${3:?缺少原生依赖前缀}"; do
    case "$destination/" in "$HOME/.local/share/dftflow/"*) echo '旧用户依赖／安装目录已退役。' >&2; exit 2 ;; esac
done
mkdir -p "$2" "$3/lib" || exit $?
dest_bin=$(realpath "$2") || exit $?
native_prefix=$(realpath "$3") || exit $?
[[ "$source_bin" != "$dest_bin" ]] || { echo '源与目标目录必须不同' >&2; exit 1; }
command -v patchelf >/dev/null || exit $?
# 所有 glibc 文件必须来自同一主机版本，加载器与 libc 不可混用。
copy_runtime() {
    local source="$1" dest="$2"
    if [[ -e "$dest" ]]; then
        cmp -s "$source" "$dest" || { echo "运行库版本冲突，请使用新的私有前缀: $dest" >&2; return 1; }
    else
        cp -L "$source" "$dest" || return $?
    fi
}
for name in ld-linux-x86-64.so.2 libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1 libresolv.so.2 libnss_files.so.2 libnss_dns.so.2; do
    copy_runtime "/lib64/$name" "$native_prefix/lib/$name" || exit $?
done
copy_runtime "$(gcc -print-file-name=libgcc_s.so.1)" "$native_prefix/lib/libgcc_s.so.1" || exit $?
for name in "${programs[@]}"; do
    cp "$source_bin/$name" "$dest_bin/$name" || exit $?
    patchelf --set-interpreter "$native_prefix/lib/ld-linux-x86-64.so.2" \
        --set-rpath "$native_prefix/lib:$native_prefix/lib64" "$dest_bin/$name" || exit $?
    "$dest_bin/$name" --help >/dev/null || exit $?
done
# 保留构建原件和打包后的指纹，便于核对二进制来源。
(cd "$source_bin" && sha256sum "${programs[@]}") > "$dest_bin/SOURCE-SHA256SUMS" || exit $?
(cd "$dest_bin" && sha256sum "${programs[@]}") > "$dest_bin/SHA256SUMS" || exit $?
echo "打包完成：$dest_bin；依赖前缀：$native_prefix。共享路径需要在计算节点可读。"
