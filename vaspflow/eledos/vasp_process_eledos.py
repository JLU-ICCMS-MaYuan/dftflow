#!/usr/bin/env python3
import argparse
import csv
import os
import re
import subprocess
from collections import OrderedDict

import numpy as np


SUPPORTED_ORBITALS = ("s", "p", "d", "f")
P_COMPONENTS = ("py", "pz", "px")
D_COMPONENTS = ("dxy", "dyz", "dz2", "dxz", "dx2")
F_COMPONENTS = ("fy3x2", "fxyz", "fyz2", "fz3", "fxz2", "fzx2", "fx3")


def parse_arguments():
    parser = argparse.ArgumentParser(description="处理 VASP eledos 目录中的 TDOS/PDOS 数据并绘图。")
    parser.add_argument(
        "-d",
        "--directory",
        default=".",
        help="vasp_eledos 工作目录，默认当前目录。",
    )
    parser.add_argument(
        "-e",
        "--elements",
        nargs="+",
        required=True,
        help='元素与轨道列表，例如: "La:s,p,d" "H:s"',
    )
    parser.add_argument(
        "--scf-outcar",
        default="../vasp_scf/OUTCAR",
        help="参考 SCF 的 OUTCAR 路径，用于同步费米能级。",
    )
    parser.add_argument(
        "--doscar",
        default="DOSCAR",
        help="DOSCAR 文件名，默认 DOSCAR。",
    )
    parser.add_argument(
        "--per-volume",
        action="store_true",
        help="若指定，则将 DOS 除以晶胞体积（单位 Å^3）。",
    )
    parser.add_argument(
        "--skip-vaspkit",
        action="store_true",
        help="跳过 vaspkit 调用，直接读取现有的 TDOS.dat 和 PDOS_*.dat。",
    )
    parser.add_argument(
        "--out-prefix",
        default="pdos_merged",
        help="输出文件名前缀，默认 pdos_merged。",
    )
    return parser.parse_args()


def parse_element_specs(items):
    specs = OrderedDict()
    for item in items:
        try:
            element, orbitals_text = item.split(":")
        except ValueError as exc:
            raise ValueError(f"无效输入格式: {item}，应为 元素:轨道,轨道") from exc
        orbitals = []
        for orbital in orbitals_text.split(","):
            orbital = orbital.strip().lower()
            if orbital not in SUPPORTED_ORBITALS:
                raise ValueError(f"不支持的轨道类型: {orbital}，仅支持 s/p/d/f")
            if orbital not in orbitals:
                orbitals.append(orbital)
        specs[element.strip()] = orbitals
    if not specs:
        raise ValueError("至少需要指定一个元素轨道组合。")
    return specs


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


def replace_doscar_fermi(doscar_path, new_fermi):
    if not os.path.exists(doscar_path):
        raise FileNotFoundError(f"未找到 DOSCAR: {doscar_path}")
    with open(doscar_path, "r", encoding="utf-8", errors="ignore") as f:
        lines = f.readlines()
    if len(lines) < 6:
        raise ValueError("DOSCAR 行数不足，无法修改费米能级。")
    parts = lines[5].split()
    if len(parts) < 4:
        raise ValueError("DOSCAR 第 6 行格式不正确，无法修改费米能级。")
    parts[3] = f"{new_fermi:.10f}"
    lines[5] = "  " + "  ".join(parts) + "\n"
    with open(doscar_path, "w", encoding="utf-8") as f:
        f.writelines(lines)
    return float(parts[3])


def run_vaspkit(work_dir, command_text):
    process = subprocess.Popen(
        ["vaspkit"],
        cwd=work_dir,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    stdout, stderr = process.communicate(input=command_text)
    if process.returncode != 0:
        raise RuntimeError(f"vaspkit 执行失败。\nstdout:\n{stdout}\nstderr:\n{stderr}")
    return stdout, stderr


def generate_vaspkit_dos_files(work_dir):
    run_vaspkit(work_dir, "11\n113\n")
    run_vaspkit(work_dir, "11\n111\n")


def read_poscar_volume(poscar_path):
    with open(poscar_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
    if len(lines) < 5:
        raise ValueError("POSCAR 内容不完整，无法计算体积。")
    scale = float(lines[1].split()[0])
    a1 = np.array([float(x) for x in lines[2].split()[:3]], dtype=float) * scale
    a2 = np.array([float(x) for x in lines[3].split()[:3]], dtype=float) * scale
    a3 = np.array([float(x) for x in lines[4].split()[:3]], dtype=float) * scale
    return float(abs(np.dot(a1, np.cross(a2, a3))))


def is_float(text):
    try:
        float(text)
        return True
    except ValueError:
        return False


def _normalize_header(name):
    return name.strip().lstrip("#").replace("(", "").replace(")", "").replace("-", "").lower()


def read_dat_table(path):
    if not os.path.exists(path):
        raise FileNotFoundError(f"未找到数据文件: {path}")
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        lines = [line.strip() for line in f if line.strip()]
    if not lines:
        raise ValueError(f"{path} 为空。")

    header = None
    data_lines = []
    for line in lines:
        first = line.split()[0]
        if is_float(first):
            data_lines.append(line)
        elif header is None:
            header = line.split()

    if not data_lines:
        raise ValueError(f"未从 {path} 解析到数值数据。")

    data = np.array([[float(x) for x in line.split()] for line in data_lines], dtype=float)
    if header is None:
        header = [f"col{i}" for i in range(data.shape[1])]
    if len(header) != data.shape[1]:
        if len(header) > data.shape[1]:
            header = header[: data.shape[1]]
        else:
            header = header + [f"col{i}" for i in range(len(header), data.shape[1])]

    columns = OrderedDict()
    for idx, name in enumerate(header):
        columns[_normalize_header(name)] = data[:, idx]
    return columns


def find_column(columns, *candidates):
    for candidate in candidates:
        if candidate in columns:
            return columns[candidate]
    raise KeyError(f"缺少列: {', '.join(candidates)}")


def sum_optional_columns(columns, names):
    arrays = []
    for name in names:
        if name in columns:
            arrays.append(columns[name])
    if len(arrays) != len(names):
        missing = [name for name in names if name not in columns]
        raise KeyError(f"缺少轨道分量列: {', '.join(missing)}")
    total = np.zeros_like(arrays[0])
    for array in arrays:
        total += array
    return total


def load_pdos_file(path, orbitals):
    columns = read_dat_table(path)
    energy = find_column(columns, "energy", "energyev", "energyev")
    total = find_column(columns, "tot", "total")
    data = {"energy": energy, "tot": total}
    if "s" in orbitals:
        data["s"] = find_column(columns, "s")
    if "p" in orbitals:
        data["p"] = sum_optional_columns(columns, P_COMPONENTS)
    if "d" in orbitals:
        data["d"] = sum_optional_columns(columns, D_COMPONENTS)
    if "f" in orbitals:
        data["f"] = sum_optional_columns(columns, F_COMPONENTS)
    return data


def load_tdos_file(path):
    columns = read_dat_table(path)
    energy = find_column(columns, "energy", "energyev", "energyev")
    tdos = None
    for key in ("tot", "dos", "tdos"):
        if key in columns:
            tdos = columns[key]
            break
    if tdos is None:
        for key, value in columns.items():
            if key not in ("energy", "energyev", "energyev", "int", "idos", "itdos"):
                tdos = value
                break
    if tdos is None:
        raise ValueError(f"未能从 {path} 识别 TDOS 列。")
    return energy, tdos


def ensure_same_grid(reference, current, path):
    if len(reference) != len(current) or np.max(np.abs(reference - current)) > 1e-8:
        raise ValueError(f"能量网格不一致: {path}")


def interpolate_at_zero(energy, values):
    if energy[0] > 0 or energy[-1] < 0:
        idx = int(np.abs(energy).argmin())
        return float(values[idx])
    return float(np.interp(0.0, energy, values))


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
                data[name].append(float(row[name]))
    return {name: np.array(values, dtype=float) for name, values in data.items()}


def plot_dos_from_csv(csv_path, png_path):
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 DOS 图。") from exc

    data = load_csv_columns(csv_path)
    energy = data.pop("Energy(eV)")

    fig, ax = plt.subplots(figsize=(8, 5))
    for label, values in data.items():
        ax.plot(energy, values, linewidth=1.5, label=label)
    ax.axvline(0.0, color="k", linestyle="--", linewidth=1.0, label="E_F")
    ax.set_xlabel("Energy - E_F (eV)")
    ax.set_ylabel("DOS")
    ax.legend()
    ax.grid(True, linestyle=":", alpha=0.4)
    fig.tight_layout()
    fig.savefig(png_path, dpi=300)
    plt.close(fig)


class VaspProcessDosSetup:
    def __init__(
        self,
        directory=".",
        element_specs=None,
        scf_outcar="../vasp_scf/OUTCAR",
        doscar="DOSCAR",
        per_volume=False,
        skip_vaspkit=False,
        out_prefix="pdos_merged",
    ):
        self.work_dir = os.path.abspath(directory)
        self.element_specs = parse_element_specs(element_specs or [])
        self.scf_outcar = os.path.abspath(os.path.join(self.work_dir, scf_outcar))
        self.doscar_path = os.path.abspath(os.path.join(self.work_dir, doscar))
        self.per_volume = per_volume
        self.skip_vaspkit = skip_vaspkit
        self.out_prefix = out_prefix

    def prepare_vaspkit_inputs(self):
        fermi = extract_fermi_from_outcar(self.scf_outcar)
        final_fermi = replace_doscar_fermi(self.doscar_path, fermi)
        print(f"已将 DOSCAR 的费米能级更新为 {final_fermi:.10f} eV")
        if not self.skip_vaspkit:
            print("正在调用 vaspkit 生成 TDOS/PDOS 数据...")
            generate_vaspkit_dos_files(self.work_dir)

    def build_merged_data(self):
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        volume = read_poscar_volume(poscar_path)
        energy_ref = None
        merged = OrderedDict()

        for element, orbitals in self.element_specs.items():
            pdos_path = os.path.join(self.work_dir, f"PDOS_{element}.dat")
            pdos_data = load_pdos_file(pdos_path, orbitals)
            if energy_ref is None:
                energy_ref = pdos_data["energy"]
                merged["Energy(eV)"] = energy_ref
            else:
                ensure_same_grid(energy_ref, pdos_data["energy"], pdos_path)

            for orbital in orbitals:
                values = pdos_data[orbital]
                if self.per_volume:
                    values = values / volume
                merged[f"{element}_{orbital}"] = values

            total_values = pdos_data["tot"]
            if self.per_volume:
                total_values = total_values / volume
            merged[element] = total_values

        tdos_path = os.path.join(self.work_dir, "TDOS.dat")
        tdos_energy, tdos_values = load_tdos_file(tdos_path)
        if energy_ref is None:
            energy_ref = tdos_energy
            merged["Energy(eV)"] = energy_ref
        else:
            ensure_same_grid(energy_ref, tdos_energy, tdos_path)
        if self.per_volume:
            tdos_values = tdos_values / volume
        merged["TDOS"] = tdos_values

        return merged, volume

    def write_outputs(self, merged):
        suffix = "_per_volume" if self.per_volume else ""
        merged_csv = os.path.join(self.work_dir, f"{self.out_prefix}{suffix}.csv")
        at_ef_csv = os.path.join(self.work_dir, f"{self.out_prefix}_at_ef{suffix}.csv")
        plot_png = os.path.join(self.work_dir, f"{self.out_prefix}{suffix}.png")

        headers = list(merged.keys())
        energy = merged["Energy(eV)"]
        rows = []
        for idx in range(len(energy)):
            rows.append([merged[name][idx] for name in headers])
        write_csv(merged_csv, headers, rows)

        ef_headers = ["Energy(eV)"] + [name for name in headers if name != "Energy(eV)"]
        ef_row = [0.0]
        for name in ef_headers[1:]:
            ef_row.append(interpolate_at_zero(energy, merged[name]))
        write_csv(at_ef_csv, ef_headers, [ef_row])

        plot_dos_from_csv(merged_csv, plot_png)
        return merged_csv, at_ef_csv, plot_png

    def run(self):
        self.prepare_vaspkit_inputs()
        merged, volume = self.build_merged_data()
        merged_csv, at_ef_csv, plot_png = self.write_outputs(merged)

        tdos_at_ef = interpolate_at_zero(merged["Energy(eV)"], merged["TDOS"])
        pdos_cols = [name for name in merged if name not in ("Energy(eV)", "TDOS") and "_" in name]
        pdos_sum = sum(interpolate_at_zero(merged["Energy(eV)"], merged[name]) for name in pdos_cols)
        print(f"晶胞体积: {volume:.6f} Å^3")
        print(f"TDOS at E_F: {tdos_at_ef:.6f}")
        print(f"Sum of selected PDOS at E_F: {pdos_sum:.6f}")
        print(f"Difference (TDOS - sum of selected PDOS): {tdos_at_ef - pdos_sum:.6f}")
        print(f"已生成: {merged_csv}")
        print(f"已生成: {at_ef_csv}")
        print(f"已生成: {plot_png}")
        return merged_csv, at_ef_csv, plot_png


def main():
    args = parse_arguments()
    setup = VaspProcessDosSetup(
        directory=args.directory,
        element_specs=args.elements,
        scf_outcar=args.scf_outcar,
        doscar=args.doscar,
        per_volume=args.per_volume,
        skip_vaspkit=args.skip_vaspkit,
        out_prefix=args.out_prefix,
    )
    setup.run()


if __name__ == "__main__":
    main()
