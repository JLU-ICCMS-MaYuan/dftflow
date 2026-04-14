#!/usr/bin/env python3
import argparse
import csv
import os

import numpy as np

from vaspflow.eledos.vasp_process_eledos import (
    ensure_same_grid,
    extract_fermi_from_outcar,
    generate_vaspkit_dos_files,
    load_pdos_file,
    load_tdos_file,
    parse_element_specs,
    read_poscar_volume,
    replace_doscar_fermi,
)


def parse_arguments():
    parser = argparse.ArgumentParser(description="计算指定能量区间内的 VASP TDOS/PDOS IDOS。")
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
        help='元素与轨道列表，例如: "Mg:s,p" "B:s"',
    )
    parser.add_argument(
        "--low",
        type=float,
        required=True,
        help="积分下限，单位 eV，能量坐标按 E - E_F。",
    )
    parser.add_argument(
        "--up",
        type=float,
        required=True,
        help="积分上限，单位 eV，能量坐标按 E - E_F。",
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
        help="若指定，则将 DOS 除以晶胞体积（单位 Å^3）后再积分。",
    )
    parser.add_argument(
        "--skip-vaspkit",
        action="store_true",
        help="跳过 vaspkit 调用，直接读取现有的 TDOS.dat 和 PDOS_*.dat。",
    )
    parser.add_argument(
        "--out-prefix",
        default="idos_computed",
        help="输出文件名前缀，默认 idos_computed。",
    )
    return parser.parse_args()


def integrate_range_with_interpolation(energy, values, low, up):
    if low >= up:
        raise ValueError("要求 low < up。")
    if len(energy) != len(values):
        raise ValueError("能量数组与数值数组长度不一致。")
    if len(energy) < 2:
        raise ValueError("至少需要两个能量点才能做积分。")
    if low < energy[0] or up > energy[-1]:
        raise ValueError(
            f"积分区间 [{low}, {up}] 超出能量网格范围 [{energy[0]}, {energy[-1]}]。"
        )

    selected_energy = [low]
    selected_values = [float(np.interp(low, energy, values))]

    for e, v in zip(energy, values):
        if low < e < up:
            selected_energy.append(float(e))
            selected_values.append(float(v))

    selected_energy.append(up)
    selected_values.append(float(np.interp(up, energy, values)))

    selected_energy = np.array(selected_energy, dtype=float)
    selected_values = np.array(selected_values, dtype=float)
    return float(np.trapz(selected_values, selected_energy))


class VaspIdosComputedSetup:
    def __init__(
        self,
        directory=".",
        element_specs=None,
        low=None,
        up=None,
        scf_outcar="../vasp_scf/OUTCAR",
        doscar="DOSCAR",
        per_volume=False,
        skip_vaspkit=False,
        out_prefix="idos_computed",
    ):
        self.work_dir = os.path.abspath(directory)
        self.element_specs = parse_element_specs(element_specs or [])
        self.low = float(low)
        self.up = float(up)
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

    def build_data(self):
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        volume = read_poscar_volume(poscar_path)
        energy_ref = None
        data = {}

        for element, orbitals in self.element_specs.items():
            pdos_path = os.path.join(self.work_dir, f"PDOS_{element}.dat")
            pdos_data = load_pdos_file(pdos_path, orbitals)
            if energy_ref is None:
                energy_ref = pdos_data["energy"]
            else:
                ensure_same_grid(energy_ref, pdos_data["energy"], pdos_path)

            for orbital in orbitals:
                values = pdos_data[orbital]
                if self.per_volume:
                    values = values / volume
                data[f"{element}_{orbital}"] = values

        tdos_path = os.path.join(self.work_dir, "TDOS.dat")
        tdos_energy, tdos_values = load_tdos_file(tdos_path)
        if energy_ref is None:
            energy_ref = tdos_energy
        else:
            ensure_same_grid(energy_ref, tdos_energy, tdos_path)
        if self.per_volume:
            tdos_values = tdos_values / volume
        data["TDOS"] = tdos_values
        return energy_ref, data, volume

    def compute_idos(self, energy, data):
        results = {
            "low_eV": self.low,
            "up_eV": self.up,
        }
        selected_sum = 0.0

        tdos_idos = integrate_range_with_interpolation(energy, data["TDOS"], self.low, self.up)
        results["TDOS_IDOS"] = tdos_idos

        for key, values in data.items():
            if key == "TDOS":
                continue
            idos = integrate_range_with_interpolation(energy, values, self.low, self.up)
            results[f"{key}_IDOS"] = idos
            selected_sum += idos

        results["sum_selected_PDOS_IDOS"] = selected_sum
        results["difference_TDOS_minus_sum_selected"] = tdos_idos - selected_sum
        return results

    def write_output(self, results):
        csv_path = os.path.join(self.work_dir, f"{self.out_prefix}.csv")
        headers = list(results.keys())
        with open(csv_path, "w", encoding="utf-8", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(headers)
            writer.writerow([results[key] for key in headers])
        return csv_path

    def run(self):
        self.prepare_vaspkit_inputs()
        energy, data, volume = self.build_data()
        results = self.compute_idos(energy, data)
        csv_path = self.write_output(results)

        print(f"晶胞体积: {volume:.6f} Å^3")
        print(f"积分区间: [{self.low:.6f}, {self.up:.6f}] eV")
        print(f"TDOS_IDOS: {results['TDOS_IDOS']:.6f}")
        for key, value in results.items():
            if key.endswith("_IDOS") and key != "TDOS_IDOS":
                print(f"{key}: {value:.6f}")
        print(f"sum_selected_PDOS_IDOS: {results['sum_selected_PDOS_IDOS']:.6f}")
        print(
            "difference_TDOS_minus_sum_selected: "
            f"{results['difference_TDOS_minus_sum_selected']:.6f}"
        )
        print(f"已生成: {csv_path}")
        return csv_path


def main():
    args = parse_arguments()
    setup = VaspIdosComputedSetup(
        directory=args.directory,
        element_specs=args.elements,
        low=args.low,
        up=args.up,
        scf_outcar=args.scf_outcar,
        doscar=args.doscar,
        per_volume=args.per_volume,
        skip_vaspkit=args.skip_vaspkit,
        out_prefix=args.out_prefix,
    )
    setup.run()


if __name__ == "__main__":
    main()
