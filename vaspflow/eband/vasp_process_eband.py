#!/usr/bin/env python3
import argparse
import csv
import os
import re
import subprocess
from collections import OrderedDict

import numpy as np

from vaspflow.eledos.vasp_process_eledos import (
    D_COMPONENTS,
    F_COMPONENTS,
    P_COMPONENTS,
    get_orbital_components,
    parse_element_specs,
    replace_doscar_fermi,
)


def parse_arguments():
    parser = argparse.ArgumentParser(description="处理 VASP eband 目录中的 BAND/PBAND 数据并绘图。")
    parser.add_argument(
        "-d",
        "--directory",
        default=".",
        help="vasp_eband 工作目录，默认当前目录。",
    )
    parser.add_argument(
        "-e",
        "--elements",
        nargs="+",
        required=True,
        help='元素与轨道列表，例如: "Mg:s,p" "B:p" "Mg:px,dz2"',
    )
    parser.add_argument(
        "--fermi",
        default=None,
        help="显式指定费米能（eV）。默认优先读 FERMI_ENERGY，再退回 OUTCAR。",
    )
    parser.add_argument(
        "--scf-outcar",
        default="../vasp_scf/OUTCAR",
        help="参考 SCF 的 OUTCAR 路径，用于同步 DOSCAR 中的费米能级。",
    )
    parser.add_argument(
        "--doscar",
        default="DOSCAR",
        help="DOSCAR 文件名，默认 DOSCAR。",
    )
    parser.add_argument(
        "--skip-vaspkit",
        action="store_true",
        help="跳过 vaspkit 调用，直接复用现有 BAND.dat/PBAND_*.dat/KLABELS。",
    )
    parser.add_argument(
        "--out-prefix",
        default="pband_merged",
        help="输出文件名前缀，默认 pband_merged。",
    )
    parser.add_argument(
        "--emin",
        type=float,
        default=None,
        help="绘图最低能量（相对费米能，eV）。",
    )
    parser.add_argument(
        "--emax",
        type=float,
        default=None,
        help="绘图最高能量（相对费米能，eV）。",
    )
    parser.add_argument(
        "--weight-scale",
        type=float,
        default=200.0,
        help="fatband 散点尺寸缩放因子，默认 200。",
    )
    return parser.parse_args()
def is_float(text):
    try:
        float(text)
        return True
    except ValueError:
        return False


def extract_fermi_from_outcar(path):
    if not os.path.exists(path):
        raise FileNotFoundError(f"未找到 OUTCAR: {path}")
    pattern = re.compile(r"E-fermi\s*:\s*([-\d.]+)")
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            match = pattern.search(line)
            if match:
                return float(match.group(1))
    raise ValueError(f"未能从 {path} 解析 E-fermi。")


def read_fermi_energy(work_dir, explicit_fermi=None):
    if explicit_fermi is not None:
        return float(explicit_fermi)
    fermi_file = os.path.join(work_dir, "FERMI_ENERGY")
    if os.path.exists(fermi_file):
        with open(fermi_file, "r", encoding="utf-8", errors="ignore") as f:
            for line in f:
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                value = stripped.split("#", 1)[0].strip().split()
                if value and is_float(value[0]):
                    return float(value[0])
    return extract_fermi_from_outcar(os.path.join(work_dir, "OUTCAR"))


def run_vaspkit(work_dir, command_text):
    process = subprocess.Popen(
        ["vaspkit"],
        cwd=work_dir,
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    process.communicate(input=command_text)
    if process.returncode != 0:
        raise RuntimeError(f"vaspkit 执行失败，退出码: {process.returncode}")
    return "", ""


def generate_vaspkit_band_files(work_dir):
    run_vaspkit(work_dir, "21\n211\n")
    run_vaspkit(work_dir, "21\n213\n")
    run_vaspkit(work_dir, "21\n215\n")


def parse_numeric_band_blocks(path):
    if not os.path.exists(path):
        raise FileNotFoundError(f"未找到文件: {path}")
    blocks = []
    current = []
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        for raw_line in f:
            line = raw_line.strip()
            if not line:
                continue
            if line.startswith("#"):
                if line.lower().startswith("# band-index") and current:
                    blocks.append(np.array(current, dtype=float))
                    current = []
                continue
            parts = line.split()
            if len(parts) >= 2 and is_float(parts[0]) and is_float(parts[1]):
                current.append([float(x) for x in parts])
    if current:
        blocks.append(np.array(current, dtype=float))
    if not blocks:
        raise ValueError(f"未从 {path} 解析到 band 数据。")
    return blocks


def sum_optional_columns(data_block, indices):
    total = np.zeros(data_block.shape[0], dtype=float)
    for idx in indices:
        if idx >= data_block.shape[1]:
            raise IndexError("PBAND 文件列数不足，无法聚合指定轨道。")
        total += data_block[:, idx]
    return total


PBAND_COMPONENT_INDEX = {
    "s": 2,
    "py": 3,
    "pz": 4,
    "px": 5,
    "dxy": 6,
    "dyz": 7,
    "dz2": 8,
    "dxz": 9,
    "dx2": 10,
    "fy3x2": 11,
    "fxyz": 12,
    "fyz2": 13,
    "fz3": 14,
    "fxz2": 15,
    "fzx2": 16,
    "fx3": 17,
}


def load_element_pband(path, orbitals):
    blocks = parse_numeric_band_blocks(path)
    result = []
    for block in blocks:
        kpath = block[:, 0]
        energy = block[:, 1]
        values = {}
        for orbital in orbitals:
            components = get_orbital_components(orbital)
            indices = [PBAND_COMPONENT_INDEX[component] for component in components]
            if max(indices) >= block.shape[1]:
                raise ValueError(f"{os.path.basename(path)} 不包含轨道 {orbital} 所需的完整分量。")
            values[orbital] = sum_optional_columns(block, indices)
        result.append({"kpath": kpath, "energy": energy, "orbitals": values})
    return result


def load_band_data(path):
    blocks = parse_numeric_band_blocks(path)
    return [{"kpath": block[:, 0], "energy": block[:, 1]} for block in blocks]


def ensure_matching_blocks(ref_blocks, test_blocks, path):
    if len(ref_blocks) != len(test_blocks):
        raise ValueError(f"{path} 的 band 数与 BAND.dat 不一致。")
    for idx, (ref_block, test_block) in enumerate(zip(ref_blocks, test_blocks), start=1):
        if len(ref_block["kpath"]) != len(test_block["kpath"]):
            raise ValueError(f"{path} 第 {idx} 条 band 的 k 点数不一致。")
        if np.max(np.abs(ref_block["kpath"] - test_block["kpath"])) > 1e-8:
            raise ValueError(f"{path} 第 {idx} 条 band 的 kpath 不一致。")
        # VASPKIT 导出的 BAND.dat 与 PBAND_*.dat 在文本写出时可能出现 1e-6 量级舍入差异。
        if np.max(np.abs(ref_block["energy"] - test_block["energy"])) > 1e-5:
            raise ValueError(f"{path} 第 {idx} 条 band 的能量不一致。")


def parse_klabels(path):
    if not os.path.exists(path):
        return []
    labels = []
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            parts = line.split()
            if len(parts) >= 2 and is_float(parts[-1]):
                label = " ".join(parts[:-1])
                if label.lower().startswith("k-label"):
                    continue
                labels.append((label, float(parts[-1])))
    return labels


def write_csv(path, headers, rows):
    with open(path, "w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(headers)
        writer.writerows(rows)


def load_csv_columns(path):
    with open(path, "r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames or []
        data = {name: [] for name in fieldnames}
        for row in reader:
            for name in fieldnames:
                value = row[name]
                if name == "label":
                    data[name].append(value)
                else:
                    data[name].append(float(value))
    return data


def _format_klabel(label):
    return label.replace("GAMMA", r"$\Gamma$")


def plot_band_from_csv(csv_path, png_path, klabels=None, emin=None, emax=None, weight_scale=200.0):
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 band 图。") from exc

    data = load_csv_columns(csv_path)
    band_index = np.array(data["band_index"], dtype=int)
    kpath = np.array(data["kpath_dist"], dtype=float)
    energy = np.array(data["energy_ev"], dtype=float)
    weight_keys = [key for key in data if key.startswith("weight_")]

    fig, ax = plt.subplots(figsize=(8, 5))

    for idx in sorted(set(band_index)):
        mask = band_index == idx
        ax.plot(kpath[mask], energy[mask], color="0.75", linewidth=0.8, zorder=1)

    for key in weight_keys:
        weights = np.array(data[key], dtype=float)
        label = key.replace("weight_", "")
        sizes = np.maximum(weights, 0.0) * weight_scale
        ax.scatter(kpath, energy, s=sizes, alpha=0.55, label=label, zorder=2)

    ax.axhline(0.0, color="k", linestyle="--", linewidth=1.0)
    if klabels:
        positions = []
        labels = []
        for label, x in klabels:
            x = float(x)
            if positions and abs(x - positions[-1]) <= 1e-8:
                labels[-1] = _format_klabel(label)
                continue
            positions.append(x)
            labels.append(_format_klabel(label))
            ax.axvline(x, color="0.4", linestyle=":", linewidth=0.8)
        ax.set_xticks(positions)
        ax.set_xticklabels(labels)
        ax.set_xlim(positions[0], positions[-1])
    else:
        ax.set_xlim(float(np.min(kpath)), float(np.max(kpath)))

    ax.set_xlabel("K-path")
    ax.set_ylabel("Energy - E_F (eV)")
    if emin is not None or emax is not None:
        ax.set_ylim(emin, emax)
    ax.margins(x=0.0)
    ax.grid(True, linestyle=":", alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(png_path, dpi=300)
    plt.close(fig)


class VaspProcessBandSetup:
    def __init__(
        self,
        directory=".",
        element_specs=None,
        fermi=None,
        scf_outcar="../vasp_scf/OUTCAR",
        doscar="DOSCAR",
        skip_vaspkit=False,
        out_prefix="pband_merged",
        emin=None,
        emax=None,
        weight_scale=200.0,
    ):
        self.work_dir = os.path.abspath(directory)
        self.element_specs = parse_element_specs(element_specs or [])
        self.fermi = fermi
        self.scf_outcar = os.path.abspath(os.path.join(self.work_dir, scf_outcar))
        self.doscar_path = os.path.abspath(os.path.join(self.work_dir, doscar))
        self.skip_vaspkit = skip_vaspkit
        self.out_prefix = out_prefix
        self.emin = emin
        self.emax = emax
        self.weight_scale = weight_scale

    def prepare_band_files(self):
        if self.skip_vaspkit:
            return
        fermi = extract_fermi_from_outcar(self.scf_outcar)
        final_fermi = replace_doscar_fermi(self.doscar_path, fermi)
        print(f"已将 DOSCAR 的费米能级更新为 {final_fermi} eV")
        print("正在调用 vaspkit 生成 BAND/PBAND 数据...")
        generate_vaspkit_band_files(self.work_dir)

    def build_merged_data(self):
        band_path = os.path.join(self.work_dir, "BAND.dat")
        band_blocks = load_band_data(band_path)
        fermi = read_fermi_energy(self.work_dir, self.fermi)
        labels = parse_klabels(os.path.join(self.work_dir, "KLABELS"))

        merged_rows = []
        for element, orbitals in self.element_specs.items():
            pband_path = os.path.join(self.work_dir, f"PBAND_{element}.dat")
            pband_blocks = load_element_pband(pband_path, orbitals)
            ensure_matching_blocks(band_blocks, pband_blocks, pband_path)

            for band_idx, pband_block in enumerate(pband_blocks, start=1):
                while len(merged_rows) < band_idx:
                    merged_rows.append([])
                merged_rows[band_idx - 1].append((element, pband_block["orbitals"]))

        csv_rows = []
        for band_idx, band_block in enumerate(band_blocks, start=1):
            for point_idx in range(len(band_block["kpath"])):
                row = OrderedDict()
                row["band_index"] = band_idx
                row["kpoint_index"] = point_idx + 1
                row["kpath_dist"] = float(band_block["kpath"][point_idx])
                row["energy_ev"] = float(band_block["energy"][point_idx] - fermi)
                for element, orbitals in self.element_specs.items():
                    entry = merged_rows[band_idx - 1]
                    orbital_map = None
                    for entry_element, entry_orbitals in entry:
                        if entry_element == element:
                            orbital_map = entry_orbitals
                            break
                    for orbital in orbitals:
                        row[f"weight_{element}_{orbital}"] = float(orbital_map[orbital][point_idx])
                csv_rows.append(row)

        return csv_rows, labels

    def write_outputs(self, rows, labels):
        csv_path = os.path.join(self.work_dir, f"{self.out_prefix}.csv")
        png_path = os.path.join(self.work_dir, f"{self.out_prefix}.png")
        label_map = OrderedDict(labels)
        headers = list(rows[0].keys()) + ["label", "tick_x"]
        csv_data = []
        for row in rows:
            tick_x = row["kpath_dist"]
            label = ""
            for label_name, x in label_map.items():
                if abs(x - tick_x) < 1e-8:
                    label = label_name
                    break
            csv_data.append([row[key] for key in rows[0].keys()] + [label, tick_x])
        write_csv(csv_path, headers, csv_data)
        plot_band_from_csv(csv_path, png_path, labels, self.emin, self.emax, self.weight_scale)
        return csv_path, png_path

    def run(self):
        self.prepare_band_files()
        rows, labels = self.build_merged_data()
        csv_path, png_path = self.write_outputs(rows, labels)
        print(f"已生成: {csv_path}")
        print(f"已生成: {png_path}")
        return csv_path, png_path


def main():
    args = parse_arguments()
    setup = VaspProcessBandSetup(
        directory=args.directory,
        element_specs=args.elements,
        fermi=args.fermi,
        scf_outcar=args.scf_outcar,
        doscar=args.doscar,
        skip_vaspkit=args.skip_vaspkit,
        out_prefix=args.out_prefix,
        emin=args.emin,
        emax=args.emax,
        weight_scale=args.weight_scale,
    )
    setup.run()


if __name__ == "__main__":
    main()
