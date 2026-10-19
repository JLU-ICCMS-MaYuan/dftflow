#!/usr/bin/env bash
# 为共享文件系统上的旧 glibc 计算节点打包本机构建制品；不改变全局环境。
# 用法：SOURCE_BIN_DIR DEST_BIN_DIR NATIVE_PREFIX
set -euo pipefail
source_bin=$(realpath "${1:?缺少源码构建的 bin 目录}")
mkdir -p "${2:?缺少版本 bin 目录}" "${3:?缺少原生依赖前缀}/lib"
dest_bin=$(realpath "$2")
native_prefix=$(realpath "$3")
[[ "$source_bin" != "$dest_bin" ]] || { echo '源与目标目录必须不同' >&2; exit 1; }
command -v patchelf >/dev/null
# 所有 glibc 文件必须来自同一主机版本，加载器与 libc 不可混用。
for name in ld-linux-x86-64.so.2 libc.so.6 libm.so.6 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1 libresolv.so.2 libnss_files.so.2 libnss_dns.so.2; do
    cp -L "/lib64/$name" "$native_prefix/lib/$name"
done
cp -L "$(gcc -print-file-name=libgcc_s.so.1)" "$native_prefix/lib/libgcc_s.so.1"
for name in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
    cp "$source_bin/$name" "$dest_bin/$name"
    patchelf --set-interpreter "$native_prefix/lib/ld-linux-x86-64.so.2" \
        --add-rpath "$native_prefix/lib" "$dest_bin/$name"
    "$dest_bin/$name" --help >/dev/null
done
# 保留构建原件和打包后的指纹，便于核对二进制来源。
(cd "$source_bin"; sha256sum qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity) > "$dest_bin/SOURCE-SHA256SUMS"
(cd "$dest_bin"; sha256sum qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity) > "$dest_bin/SHA256SUMS"
echo "打包完成：$dest_bin；依赖前缀：$native_prefix。共享路径需要在计算节点可读。"
