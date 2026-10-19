#!/usr/bin/env bash
# 将八个已构建程序更新到固定 current/bin；不保留历史安装副本。
# 用法：install-current.sh SOURCE_BIN [INSTALL_ROOT] [ENTRY_DIR]
# SOURCE_BIN 必须已经完成目标机器需要的运行库打包。
set -euo pipefail
source_bin=$(cd "${1:?用法: install-current.sh SOURCE_BIN [INSTALL_ROOT] [ENTRY_DIR]}" && pwd)
install_root="${2:-$HOME/.local/share/dftflow}"
entry_dir="${3:-$HOME/.local/bin}"
script_dir=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$install_root/current/bin" "$entry_dir"
install_root=$(cd "$install_root" && pwd)
entry_dir=$(cd "$entry_dir" && pwd)
[[ "$entry_dir" != "$install_root/current/bin" ]] || { echo '入口目录不能等于程序目录' >&2; exit 1; }
lock_dir="$install_root/.install-current.lock"
mkdir "$lock_dir" || { echo '另一个安装正在进行；请检查安装锁' >&2; exit 1; }
stage=""
cleanup() {
    [[ -z "$stage" ]] || rm -rf -- "$stage"
    rmdir "$lock_dir"
}
trap cleanup EXIT
stage=$(mktemp -d "$install_root/.install-current.XXXXXX")
for program in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
    [[ -x "$source_bin/$program" ]] || { echo "缺少可执行文件: $source_bin/$program" >&2; exit 1; }
    cp "$source_bin/$program" "$stage/$program"
done
# 在覆盖任何已安装程序之前验证全部命令的启动能力。
bash "$script_dir/verify-entries.sh" "$stage" >/dev/null
for program in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
    mv -f "$stage/$program" "$install_root/current/bin/$program"
done
# 迁移本安装器拥有的旧 EPW 程序；独立安装的同名文件不动。
if [[ -L "$entry_dir/epwflow" ]] && [[ "$(readlink "$entry_dir/epwflow")" == "$install_root/current/bin/epwflow" ]]; then
    if [[ -f "$install_root/current/bin/epwflow" && ! -L "$install_root/current/bin/epwflow" ]]; then
        rm -- "$install_root/current/bin/epwflow"
    fi
fi
bash "$script_dir/install-compat.sh" "$install_root/current/bin" "$entry_dir"
echo "已更新固定安装目录：$install_root/current/bin"
