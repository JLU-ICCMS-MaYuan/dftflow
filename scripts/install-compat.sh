#!/usr/bin/env bash
# install-compat.sh —— 为五个家族二进制创建 48 个旧命令名兼容入口（0004 T027）。
#
# 用法：./install-compat.sh <bin_dir> [target_dir]
#   <bin_dir>   五个二进制所在目录（如 target/release）
#   target_dir  兼容入口安装目录（默认 ~/.local/bin）
#
# 实现方式：为每个入口创建指向稳定版本目录中家族二进制的软链接，
# 二进制按 argv[0] 分派到对应子命令；cargo 只交付 5 个二进制，不增加第六个。
set -euo pipefail

BIN_DIR="${1:?用法: install-compat.sh <bin_dir> [target_dir]}"
TARGET_DIR="${2:-$HOME/.local/bin}"

for binary in qeflow vaspflow scflow wannier90 substitution; do
    test -x "$BIN_DIR/$binary" || { echo "缺少可执行文件: $BIN_DIR/$binary" >&2; exit 1; }
done
BIN_DIR="$(cd "$BIN_DIR" && pwd)"
mkdir -p "$TARGET_DIR"
TARGET_DIR="$(cd "$TARGET_DIR" && pwd)"
if [[ "$BIN_DIR" == "$TARGET_DIR" ]]; then
    echo "源目录与安装目录不能相同" >&2
    exit 1
fi
BACKUP_DIR="$TARGET_DIR/.dftflow-backup-$(date +%Y%m%d-%H%M%S)-$$"

link_entry() {
    local name="$1" target_binary="$2"
    local dest="$TARGET_DIR/$name"
    if [[ -d "$dest" && ! -L "$dest" ]]; then
        echo "安装目标是目录，拒绝覆盖: $dest" >&2
        exit 1
    fi
    if [[ -e "$dest" || -L "$dest" ]]; then
        mkdir -p "$BACKUP_DIR"
        cp -P "$dest" "$BACKUP_DIR/$name"
    fi
    local temp="$TARGET_DIR/.dftflow-$name-$$"
    ln -s "$BIN_DIR/$target_binary" "$temp"
    mv -f "$temp" "$dest"

}

# 同时安装家族入口，供生成的跨 flow 脚本使用。
for family in qeflow vaspflow wannier90; do link_entry "$family" "$family"; done

# qeflow 家族（17 个 qe_* + qf-gen-dyn0 别名；T013 合并，不交付第六个二进制）
for entry in \
    qe_scf qe_nscf qe_eband qe_eledos qe_process_eband qe_process_eledos \
    qe_ph_nosplit qe_merge qe_q2r qe_matdyn_phband qe_matdyn_phdos \
    qe_process_phonon qe_split_submit qe_check \
    qe_phonon_nosplit qe_phonon_split_index qe_phonon_split_coords \
    qf-gen-dyn0; do
    link_entry "$entry" qeflow
done

# vaspflow 家族（19 个 vasp_*）
for entry in \
    vasp_scf vasp_scf_converge vasp_eband vasp_process_eband \
    vasp_eledos vasp_process_eledos vasp_idos_computed vasp_fermi \
    vasp_cohp_uniform vasp_cohp_fatband vasp_lobsterin \
    vasp_plot_cohp vasp_plot_kcohp vasp_locpot vasp_process_locpot \
    vasp_charge vasp_charge_diff vasp_opt vasp_elf; do
    link_entry "$entry" vaspflow
done

# wannier90flow 家族（4 个 wannier_*）
for entry in wannier_init wannier_post wannier_window wannier_plot; do
    link_entry "$entry" wannier90
done

# substitutionflow 家族（7 个 substitution*）
for entry in \
    substitution substitution_structure substitution_continue \
    substitution_prepare_potcar substitution_prepare_qe_pseudo \
    substitution_plot_eledos substitution_check_phonon_stability; do
    link_entry "$entry" substitution
done

# scflow 家族（1 个）
link_entry scflow scflow

echo "已安装 五个家族入口、48 个旧命令与额外 qf-gen-dyn0 别名到 $TARGET_DIR"
echo "请确认 $TARGET_DIR 在 PATH 中：export PATH=\"$TARGET_DIR:\$PATH\""
