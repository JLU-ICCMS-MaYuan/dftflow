#!/usr/bin/env python3
import argparse
import glob
import math
import os
import re
import sys
from typing import Dict, Optional, Sequence, Tuple

import numpy as np
from vaspflow.cohp.orbital_interactions import (
    InteractionSide,
    InteractionSpec,
    match_basis_indices as match_indices,
    parse_interaction_side,
    parse_interaction_spec,
)


def get_reciprocal_lattice_from_poscar(poscar_file: str) -> np.ndarray:
    with open(poscar_file, "r", encoding="utf-8") as f:
        lines = f.readlines()
    scale = float(lines[1].strip())
    a1 = np.array(list(map(float, lines[2].split())), dtype=float) * scale
    a2 = np.array(list(map(float, lines[3].split())), dtype=float) * scale
    a3 = np.array(list(map(float, lines[4].split())), dtype=float) * scale
    volume = np.dot(a1, np.cross(a2, a3))
    if abs(volume) < 1e-12:
        raise ValueError("POSCAR 晶胞体积接近 0，无法计算倒易晶格。")
    b1 = 2 * np.pi * np.cross(a2, a3) / volume
    b2 = 2 * np.pi * np.cross(a3, a1) / volume
    b3 = 2 * np.pi * np.cross(a1, a2) / volume
    return np.array([b1, b2, b3])


def parse_kpoints_and_kpath(kpoints_file: str, kpath_file: str, poscar_file: str):
    reciprocal_lattice = get_reciprocal_lattice_from_poscar(poscar_file)

    high_sym_points_dict = {}
    with open(kpath_file, "r", encoding="utf-8") as f:
        for raw_line in f:
            line = raw_line.strip()
            if not line or "!" in line or "K-Path" in line or "Line-Mode" in line or "Reciprocal" in line:
                continue
            parts = line.split()
            if len(parts) < 4:
                continue
            try:
                x, y, z = float(parts[0]), float(parts[1]), float(parts[2])
            except ValueError:
                continue
            label = parts[-1]
            if "GAMMA" in label.upper():
                label = r"$\Gamma$"
            high_sym_points_dict[(x, y, z)] = label

    with open(kpoints_file, "r", encoding="utf-8") as f:
        lines = f.readlines()
    parts = lines[0].split()
    if len(parts) < 7:
        raise ValueError("KPOINTS 第一行格式不正确，无法解析 fatband 路径信息。")
    num_ibz_kpts = int(parts[4])
    num_path_kpts = int(parts[6])
    start_line = 3 + num_ibz_kpts
    end_line = min(start_line + num_path_kpts, len(lines))

    path_kpoints_coords = []
    for i in range(start_line, end_line):
        values = lines[i].split()
        if len(values) < 3:
            continue
        try:
            path_kpoints_coords.append(list(map(float, values[:3])))
        except ValueError:
            continue
    if not path_kpoints_coords:
        raise ValueError("KPOINTS 中未解析到高对称路径 k 点。")

    distances = [0.0]
    high_sym_points = []
    first_coord = tuple(path_kpoints_coords[0])
    for sym_coord, sym_label in high_sym_points_dict.items():
        if np.allclose(first_coord, sym_coord, atol=1e-4):
            high_sym_points.append({"label": sym_label, "distance": 0.0})
            break

    for i in range(1, len(path_kpoints_coords)):
        prev_k_frac = np.array(path_kpoints_coords[i - 1], dtype=float)
        curr_k_frac = np.array(path_kpoints_coords[i], dtype=float)
        prev_k_cart = np.dot(prev_k_frac, reciprocal_lattice)
        curr_k_cart = np.dot(curr_k_frac, reciprocal_lattice)
        distances.append(distances[-1] + np.linalg.norm(curr_k_cart - prev_k_cart))

        curr_coord = tuple(path_kpoints_coords[i])
        for sym_coord, sym_label in high_sym_points_dict.items():
            if np.allclose(curr_coord, sym_coord, atol=1e-4):
                high_sym_points.append({"label": sym_label, "distance": distances[-1]})
                break

    unique_points = []
    seen = []
    for point in sorted(high_sym_points, key=lambda item: item["distance"]):
        if not any(abs(point["distance"] - value) < 1e-6 for value in seen):
            unique_points.append(point)
            seen.append(point["distance"])
    return np.array(distances, dtype=float), unique_points


def parse_outcar_fermi(outcar_path: str) -> float:
    pattern = re.compile(r"E-fermi\s*:\s*([-+]?\d*\.?\d+(?:[Ee][-+]?\d+)?)")
    with open(outcar_path, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            match = pattern.search(line)
            if match:
                return float(match.group(1))
    raise ValueError(f"未能从 {outcar_path} 读取到 E-fermi。")


def parse_single_band_cohp_file(filename: str):
    with open(filename, "r", encoding="utf-8") as f:
        content = f.read()
    lines = content.splitlines()
    header_match = re.search(r"kpoints\s+(\d+)", lines[0])
    if not header_match:
        raise ValueError(f"无法从 {filename} 第一行解析 k 点数。")
    num_kpoints = int(header_match.group(1))
    data_blocks = content.split("COHP_munujk matrix for spin")[1:]
    if not data_blocks:
        raise ValueError(f"{filename} 中没有 COHP 矩阵数据。")
    num_kpoints = min(num_kpoints, len(data_blocks))
    first_block_lines = data_blocks[0].splitlines()
    if len(first_block_lines) < 3:
        raise ValueError(f"{filename} 中的第一个 COHP block 格式不完整。")
    labels = first_block_lines[1].split()[1:]
    num_orbitals = len(labels)
    energies = np.zeros(num_kpoints, dtype=float)
    cohps = np.zeros((num_kpoints, num_orbitals, num_orbitals), dtype=float)

    for ik, block in enumerate(data_blocks[:num_kpoints]):
        block_lines = [line for line in block.splitlines() if line.strip()]
        eigenval_match = re.search(r"eigenvalue\s+([-\d.]+)", block_lines[0])
        if not eigenval_match:
            raise ValueError(f"无法从 {filename} 的第 {ik + 1} 个 block 读取 eigenvalue。")
        energies[ik] = float(eigenval_match.group(1))
        matrix_lines = block_lines[2 : 2 + num_orbitals]
        if len(matrix_lines) != num_orbitals:
            raise ValueError(f"{filename} 的第 {ik + 1} 个 block 矩阵行数不完整。")
        for row_index, line in enumerate(matrix_lines):
            values = line.split()[1:]
            if len(values) != num_orbitals:
                raise ValueError(f"{filename} 的第 {ik + 1} 个 block 矩阵列数不完整。")
            cohps[ik, row_index, :] = np.array(values, dtype=float)
    return energies, cohps, labels


def calculate_pcohp(cohps: np.ndarray, labels: Sequence[str], interaction: InteractionSpec) -> np.ndarray:
    left_indices = match_indices(labels, interaction.left)
    right_indices = match_indices(labels, interaction.right)
    if not left_indices or not right_indices:
        raise ValueError(
            f"相互作用 {interaction.raw} 未能匹配到 basis labels。"
        )
    pcohp_values = np.zeros(cohps.shape[0], dtype=float)
    for i in left_indices:
        for j in right_indices:
            pcohp_values += cohps[:, i, j]
    return pcohp_values


def calculate_subplot_layout(n_plots: int) -> Tuple[int, int]:
    if n_plots <= 1:
        return 1, 1
    if n_plots == 2:
        return 1, 2
    if n_plots <= 4:
        return 2, 2
    if n_plots <= 6:
        return 2, 3
    cols = math.ceil(math.sqrt(n_plots))
    rows = math.ceil(n_plots / cols)
    return rows, cols


def build_output_prefix(interactions: Sequence[InteractionSpec], explicit_prefix: Optional[str]) -> str:
    if explicit_prefix:
        return explicit_prefix
    return "band_resolved_pcohp_" + "_and_".join(interaction.filename_label for interaction in interactions)


def save_data_to_dat(
    filename: str,
    k_distances: np.ndarray,
    shifted_energies: np.ndarray,
    pcohps_dict: Dict[str, np.ndarray],
):
    labels = list(pcohps_dict.keys())
    num_bands, num_kpoints = shifted_energies.shape
    with open(filename, "w", encoding="utf-8") as f:
        header_labels = ["# K-Distance", "Energy"] + labels
        f.write("   ".join(f"{label:<24}" for label in header_labels) + "\n")
        for ib in range(num_bands):
            for ik in range(num_kpoints):
                values = " ".join(f"{pcohps_dict[label][ib, ik]:<24.10f}" for label in labels)
                f.write(f"{k_distances[ik]:<24.10f} {shifted_energies[ib, ik]:<24.10f} {values}\n")
            f.write("\n")


def plot_band_resolved_pcohp(
    filename: str,
    k_distances: np.ndarray,
    shifted_energies: np.ndarray,
    pcohps_dict: Dict[str, np.ndarray],
    high_sym_points,
    energy_range,
    vmax: float,
):
    try:
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 band-resolved pCOHP。") from exc

    num_bands, _ = shifted_energies.shape
    interaction_labels = list(pcohps_dict.keys())
    rows, cols = calculate_subplot_layout(len(interaction_labels))
    fig_width = min(20, cols * 6)
    fig_height = min(16, rows * 4)
    fig, axes = plt.subplots(rows, cols, figsize=(fig_width, fig_height))
    if len(interaction_labels) == 1:
        axes = np.array([[axes]])
    elif rows == 1 or cols == 1:
        axes = np.array(axes).reshape(rows, cols)

    scatter = None
    for index, label in enumerate(interaction_labels):
        row = index // cols
        col = index % cols
        ax = axes[row, col]
        for ib in range(num_bands):
            ax.plot(k_distances, shifted_energies[ib, :], color="0.6", lw=0.5, alpha=0.7)
        current_pcohp = pcohps_dict[label]
        x_coords = np.tile(k_distances, num_bands)
        y_coords = shifted_energies.flatten()
        colors = current_pcohp.flatten()
        scatter = ax.scatter(
            x_coords,
            y_coords,
            c=colors,
            cmap="coolwarm_r",
            s=18,
            vmin=-vmax,
            vmax=vmax,
            rasterized=True,
        )
        ax.axhline(0.0, color="k", linestyle="--", linewidth=1.0, alpha=0.8)
        ax.set_xlim(k_distances[0], k_distances[-1])
        if energy_range:
            ax.set_ylim(energy_range[0], energy_range[1])
        ax.set_title(label, fontsize=11)
        tick_locs = [point["distance"] for point in high_sym_points]
        tick_lbls = [point["label"] for point in high_sym_points]
        if tick_locs:
            ax.set_xticks(tick_locs)
            if row == rows - 1:
                ax.set_xticklabels(tick_lbls, fontsize=10)
            else:
                ax.set_xticklabels([])
            for loc in tick_locs:
                ax.axvline(x=loc, color="k", linestyle="--", linewidth=0.6, alpha=0.5)
        if col == 0:
            ax.set_ylabel(r"Energy (E - E$_F$) [eV]", fontsize=11)

    for index in range(len(interaction_labels), rows * cols):
        axes[index // cols, index % cols].set_visible(False)

    fig.suptitle("Band-Resolved pCOHP Analysis", fontsize=15, fontweight="bold", y=0.98)
    plt.tight_layout(rect=[0, 0.08, 1, 0.95])
    if scatter is not None:
        cbar_ax = fig.add_axes([0.15, 0.02, 0.7, 0.03])
        cbar = fig.colorbar(scatter, cax=cbar_ax, orientation="horizontal")
        cbar.set_label("pCOHP (a.u.)", fontsize=12)
    plt.savefig(filename, dpi=300, bbox_inches="tight")
    plt.close(fig)


class VaspPlotKCohpSetup:
    def __init__(
        self,
        directory: str,
        interaction_texts: Sequence[str],
        fermi: Optional[float],
        emin,
        emax,
        vmax,
        output_prefix: Optional[str],
    ):
        self.directory = os.path.abspath(directory)
        self.interactions = [parse_interaction_spec(text) for text in interaction_texts]
        if not self.interactions:
            raise ValueError(
                "至少需要一个 --interaction，示例: --interaction \"Mg:3s,3p|B:2s,2p\""
            )
        self.fermi_energy = fermi
        self.energy_range = None if emin is None and emax is None else [emin, emax]
        self.vmax = vmax
        self.output_prefix = build_output_prefix(self.interactions, output_prefix)

    def _resolve_fermi_energy(self) -> float:
        if self.fermi_energy is not None:
            return self.fermi_energy
        outcar_path = os.path.join(self.directory, "OUTCAR")
        if not os.path.exists(outcar_path):
            raise FileNotFoundError("未找到 OUTCAR，无法自动读取费米能，请使用 --fermi。")
        return parse_outcar_fermi(outcar_path)

    def run(self):
        file_list = sorted(
            glob.glob(os.path.join(self.directory, "KspaceCOHPBand*.lobster")),
            key=lambda path: int(re.search(r"(\d+)", os.path.basename(path)).group(1)),
        )
        if not file_list:
            raise FileNotFoundError("当前目录下未找到 KspaceCOHPBand*.lobster 文件。")

        k_distances, high_sym_points = parse_kpoints_and_kpath(
            os.path.join(self.directory, "KPOINTS"),
            os.path.join(self.directory, "KPATH.in"),
            os.path.join(self.directory, "POSCAR"),
        )
        fermi_energy = self._resolve_fermi_energy()

        all_energies = []
        pcohps_by_label = {interaction.label: [] for interaction in self.interactions}
        labels = None
        for filename in file_list:
            energies_one_band, cohps_one_band, current_labels = parse_single_band_cohp_file(filename)
            if labels is None:
                labels = current_labels
            if len(energies_one_band) < len(k_distances):
                raise ValueError(f"{os.path.basename(filename)} 的 k 点数少于 KPOINTS 路径点数。")
            energies_one_band = energies_one_band[: len(k_distances)]
            cohps_one_band = cohps_one_band[: len(k_distances), :, :]
            all_energies.append(energies_one_band)
            for interaction in self.interactions:
                pcohp = calculate_pcohp(cohps_one_band, labels, interaction)
                pcohps_by_label[interaction.label].append(pcohp)

        energies = np.array(all_energies, dtype=float)
        shifted_energies = energies - fermi_energy
        pcohps_dict = {
            label: np.array(values, dtype=float)
            for label, values in pcohps_by_label.items()
        }

        dat_path = os.path.join(self.directory, f"{self.output_prefix}.dat")
        png_path = os.path.join(self.directory, f"{self.output_prefix}.png")
        save_data_to_dat(dat_path, k_distances, shifted_energies, pcohps_dict)
        plot_band_resolved_pcohp(
            png_path,
            k_distances,
            shifted_energies,
            pcohps_dict,
            high_sym_points,
            self.energy_range,
            self.vmax,
        )
        print(f"数据已保存至: {dat_path}")
        print(f"绘图已保存为: {png_path}")
        return dat_path, png_path


def parse_args():
    parser = argparse.ArgumentParser(description="绘制 band-resolved projected COHP 图")
    parser.add_argument("-d", "--directory", default=".", help="LOBSTER 输出目录，默认为当前目录")
    parser.add_argument(
        "--interaction",
        action="append",
        default=[],
        help='轨道相互作用，格式如 "Mg:3s,3p|B:2s,2p"，可重复传入',
    )
    parser.add_argument("--fermi", type=float, help="显式指定费米能，覆盖从 OUTCAR 自动读取的值")
    parser.add_argument("--emin", type=float, default=None, help="绘图最小能量 (E-Ef)")
    parser.add_argument("--emax", type=float, default=None, help="绘图最大能量 (E-Ef)")
    parser.add_argument("--vmax", type=float, default=1.0, help="颜色映射最大值")
    parser.add_argument("-o", "--output-prefix", help="输出文件名前缀")
    return parser.parse_args()


def main():
    args = parse_args()
    try:
        setup = VaspPlotKCohpSetup(
            directory=args.directory,
            interaction_texts=args.interaction,
            fermi=args.fermi,
            emin=args.emin,
            emax=args.emax,
            vmax=args.vmax,
            output_prefix=args.output_prefix,
        )
        setup.run()
    except Exception as exc:
        print(f"错误: {exc}")
        sys.exit(1)


if __name__ == "__main__":
    main()
