#!/usr/bin/env python3
import argparse
import os
import sys
from collections import OrderedDict

import numpy as np

from vaspflow.cohp.orbital_interactions import orbital_pair_matches, parse_interaction_spec


def load_cohpcar(filename):
    try:
        from pymatgen.io.lobster.outputs import Cohpcar
    except ImportError as exc:
        raise ImportError("缺少 pymatgen，无法解析 COHPCAR.lobster。请安装 `pip install .[analysis]`。") from exc
    return Cohpcar(filename=filename)


def _get_first_spin_array(data_dict, label):
    if not data_dict:
        raise ValueError(f"{label} 数据为空。")
    return next(iter(data_dict.values()))


def average_cohp_for_interaction(cohpcar, num_bonds, energy_grid, interaction):
    total_cohp = np.zeros_like(energy_grid, dtype=float)
    total_icohp = np.zeros_like(energy_grid, dtype=float)
    fermi_index = np.argmin(np.abs(energy_grid - 0.0))

    total_orbit_num = 0
    for orbitals_dict in cohpcar.orb_res_cohp.values():
        for orb_name, lobster_data in orbitals_dict.items():
            try:
                left_label, right_label = orb_name.split("-", 1)
            except ValueError:
                continue
            if orbital_pair_matches(interaction, left_label, right_label):
                total_orbit_num += 1
                total_cohp += _get_first_spin_array(lobster_data["COHP"], "COHP")
                total_icohp += _get_first_spin_array(lobster_data["ICOHP"], "ICOHP")

    if total_orbit_num > 0:
        total_cohp /= num_bonds
        total_icohp /= num_bonds
        icohp_at_ef = total_icohp[fermi_index]
    else:
        print(f"警告: 未找到相互作用 {interaction.label} 的贡献。")
        icohp_at_ef = 0.0

    return total_cohp, total_icohp, icohp_at_ef


def save_data(filename, header_labels, matrix):
    with open(filename, "w", encoding="utf-8") as f:
        f.write("  ".join(header_labels) + "\n")
        np.savetxt(f, matrix, fmt="%12.6f")


def _calculate_subplot_layout(n_plots):
    if n_plots <= 1:
        return 1, 1
    if n_plots == 2:
        return 1, 2
    if n_plots <= 4:
        return 2, 2
    cols = int(np.ceil(np.sqrt(n_plots)))
    rows = int(np.ceil(n_plots / cols))
    return rows, cols


def plot_projected_cohp(filename, energy_grid, projected_data, energy_range=None):
    try:
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 COHP 图。") from exc

    labels = list(projected_data.keys())
    rows, cols = _calculate_subplot_layout(len(labels))
    fig, axes = plt.subplots(rows, cols, figsize=(min(20, cols * 6), min(16, rows * 4)))
    axes = np.atleast_2d(axes)

    for idx, label in enumerate(labels):
        row = idx // cols
        col = idx % cols
        ax = axes[row, col]
        data = projected_data[label]
        ax.plot(energy_grid, data["COHP"], label="COHP", color="tab:blue", linewidth=1.5)
        ax.plot(energy_grid, data["ICOHP"], label="ICOHP", color="tab:red", linewidth=1.5)
        ax.axvline(0.0, color="k", linestyle="--", linewidth=1.0, alpha=0.8)
        if energy_range is not None:
            ax.set_xlim(energy_range[0], energy_range[1])
        ax.set_title(label)
        if col == 0:
            ax.set_ylabel("COHP / ICOHP")
        if row == rows - 1:
            ax.set_xlabel("Energy (eV)")
        ax.legend(fontsize=9)

    for idx in range(len(labels), rows * cols):
        axes[idx // cols, idx % cols].set_visible(False)

    fig.suptitle("Projected COHP / ICOHP", fontsize=15, fontweight="bold")
    plt.tight_layout(rect=[0, 0, 1, 0.96])
    plt.savefig(filename, dpi=300, bbox_inches="tight")
    plt.close(fig)


def plot_average_cohp(filename, energy_grid, average_cohp, average_icohp, energy_range=None):
    try:
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 average COHP 图。") from exc

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot(energy_grid, average_cohp, label="COHP", color="tab:blue", linewidth=1.5)
    ax.plot(energy_grid, average_icohp, label="ICOHP", color="tab:red", linewidth=1.5)
    ax.axvline(0.0, color="k", linestyle="--", linewidth=1.0, alpha=0.8)
    if energy_range is not None:
        ax.set_xlim(energy_range[0], energy_range[1])
    ax.set_xlabel("Energy (eV)")
    ax.set_ylabel("COHP / ICOHP")
    ax.set_title("Average COHP / ICOHP")
    ax.legend()
    plt.tight_layout()
    plt.savefig(filename, dpi=300, bbox_inches="tight")
    plt.close(fig)


class VaspPlotCohpSetup:
    def __init__(
        self,
        directory=".",
        file_name="COHPCAR.lobster",
        interaction_texts=None,
        lower_energy=None,
        upper_energy=None,
        emin=None,
        emax=None,
        out_prefix=None,
    ):
        self.directory = os.path.abspath(directory)
        self.file_name = file_name
        self.interactions = [parse_interaction_spec(text) for text in (interaction_texts or [])]
        self.lower_energy = lower_energy
        self.upper_energy = upper_energy
        self.energy_range = None if emin is None and emax is None else [emin, emax]
        self.out_prefix = out_prefix

    @property
    def cohpcar_path(self):
        return os.path.join(self.directory, self.file_name)

    def _resolve_output_name(self, base_name):
        if self.out_prefix:
            stem = os.path.splitext(base_name)[0]
            ext = os.path.splitext(base_name)[1]
            return os.path.join(self.directory, f"{self.out_prefix}_{stem}{ext}")
        return os.path.join(self.directory, base_name)

    def run(self):
        if not self.interactions:
            raise ValueError(
                "至少需要一个 --interaction，示例: --interaction \"Mg:3s,3p|B:2s,2p\""
            )

        cohpcar = load_cohpcar(self.cohpcar_path)
        if cohpcar.orb_res_cohp is None:
            raise ValueError(
                "当前 COHPCAR.lobster 不包含可解析的 orbital-resolved COHP 数据，"
                "无法按轨道对绘图。请确认该目录中的 LOBSTER 输出已被 pymatgen 识别为 orbitalWise 数据。"
            )
        num_bonds = len(cohpcar.cohp_data) - 1
        energy_grid = np.array(cohpcar.energies, dtype=float)

        print(f"Analysis based on {num_bonds} bonds found in {self.cohpcar_path}")

        proj_cohp_matrix = [energy_grid]
        proj_cohp_label = ["energy"]
        icohp_at_ef_list = []
        projected_plot_data = OrderedDict()

        for interaction in self.interactions:
            total_cohp, total_icohp, icohp_at_ef = average_cohp_for_interaction(
                cohpcar, num_bonds, energy_grid, interaction
            )
            proj_cohp_matrix.extend([total_cohp, total_icohp])
            proj_cohp_label.extend([interaction.label + "-COHP", interaction.label + "-ICOHP"])
            icohp_at_ef_list.append(icohp_at_ef)
            projected_plot_data[interaction.label] = {
                "COHP": total_cohp,
                "ICOHP": total_icohp,
            }

        proj_cohp_matrix = np.array(proj_cohp_matrix).T
        proj_cohp_path = self._resolve_output_name("proj_cohp.dat")
        save_data(proj_cohp_path, proj_cohp_label, proj_cohp_matrix)

        average_data = cohpcar.cohp_data["average"]
        average_cohp = _get_first_spin_array(average_data["COHP"], "average COHP")
        average_icohp = _get_first_spin_array(average_data["ICOHP"], "average ICOHP")
        fermi_index = np.argmin(np.abs(energy_grid - 0.0))
        average_icohp_at_ef = average_icohp[fermi_index]
        average_matrix = np.array([energy_grid, average_cohp, average_icohp]).T
        average_path = self._resolve_output_name("average_cohp.dat")
        save_data(average_path, ["energy", "COHP", "ICOHP"], average_matrix)

        proj_png_path = self._resolve_output_name("proj_cohp.png")
        avg_png_path = self._resolve_output_name("average_cohp.png")
        plot_projected_cohp(proj_png_path, energy_grid, projected_plot_data, self.energy_range)
        plot_average_cohp(avg_png_path, energy_grid, average_cohp, average_icohp, self.energy_range)

        print("\n--- ICOHP values at the Fermi level (E_Fermi = 0.000 eV) ---")
        for interaction, value in zip(self.interactions, icohp_at_ef_list):
            print(f"  {interaction.label + '-ICOHP':<20}: {value:12.6f}")
        print(f"  {'average-ICOHP':<20}: {average_icohp_at_ef:12.6f}")
        print("-" * 55)

        if self.lower_energy is not None and self.upper_energy is not None:
            low_index = np.argmin(np.abs(energy_grid - self.lower_energy))
            up_index = np.argmin(np.abs(energy_grid - self.upper_energy))
            actual_low = energy_grid[low_index]
            actual_up = energy_grid[up_index]

            print(f"\n--- ICOHP values in energy range [{actual_low:.3f} eV, {actual_up:.3f} eV] ---")
            print(f"{'Interaction':<20} | {'ICOHP @ EF':>12} | {'ICOHP @ low':>12} | {'ICOHP @ up':>12} | {'ICOHP(up-low)':>14}")
            print("-" * 78)
            for i, interaction in enumerate(self.interactions):
                label = interaction.label + "-ICOHP"
                col_index = proj_cohp_label.index(label)
                icohp_at_low = proj_cohp_matrix[low_index, col_index]
                icohp_at_up = proj_cohp_matrix[up_index, col_index]
                icohp_diff = icohp_at_up - icohp_at_low
                print(
                    f"{label:<20} | {icohp_at_ef_list[i]:12.6f} | {icohp_at_low:12.6f} | "
                    f"{icohp_at_up:12.6f} | {icohp_diff:14.6f}"
                )

            avg_icohp_at_low = average_icohp[low_index]
            avg_icohp_at_up = average_icohp[up_index]
            avg_icohp_diff = avg_icohp_at_up - avg_icohp_at_low
            print(
                f"{'average-ICOHP':<20} | {average_icohp_at_ef:12.6f} | {avg_icohp_at_low:12.6f} | "
                f"{avg_icohp_at_up:12.6f} | {avg_icohp_diff:14.6f}"
            )
            print("-" * 78)

        return {
            "proj_dat": proj_cohp_path,
            "average_dat": average_path,
            "proj_png": proj_png_path,
            "average_png": avg_png_path,
        }


def parse_args():
    parser = argparse.ArgumentParser(
        description="Run COHP analysis on a COHPCAR.lobster file.",
        formatter_class=argparse.RawTextHelpFormatter,
    )
    parser.add_argument(
        "-d",
        "--directory",
        default=".",
        help="LOBSTER 输出目录，默认为当前目录。",
    )
    parser.add_argument(
        "-f",
        "--file",
        type=str,
        default="COHPCAR.lobster",
        help="COHPCAR.lobster 文件名，默认为 COHPCAR.lobster。",
    )
    parser.add_argument(
        "--interaction",
        action="append",
        default=[],
        help='轨道相互作用，格式如 "Mg:3s,3p|B:2s,2p"，可重复传入',
    )
    parser.add_argument("-low", "--lower_energy", type=float, help="ICOHP 能窗分析的下限。")
    parser.add_argument("-up", "--upper_energy", type=float, help="ICOHP 能窗分析的上限。")
    parser.add_argument("--emin", type=float, help="绘图能量范围下限。")
    parser.add_argument("--emax", type=float, help="绘图能量范围上限。")
    parser.add_argument("--out-prefix", type=str, help="输出文件名前缀。")
    return parser.parse_args()


def main():
    args = parse_args()
    try:
        setup = VaspPlotCohpSetup(
            directory=args.directory,
            file_name=args.file,
            interaction_texts=args.interaction,
            lower_energy=args.lower_energy,
            upper_energy=args.upper_energy,
            emin=args.emin,
            emax=args.emax,
            out_prefix=args.out_prefix,
        )
        setup.run()
    except Exception as exc:
        print(f"错误: {exc}")
        sys.exit(1)


if __name__ == "__main__":
    main()
