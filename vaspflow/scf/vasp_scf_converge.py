#!/usr/bin/env python3
import argparse
import copy
import math
import os
import shutil
from typing import Any, Dict, List, Optional, Tuple

from public.config_templates import load_config_file
from public.job import add_run_submit_arguments, handle_job_action
from vaspflow.scf.vasp_scf import VaspSetup
from vaspflow.templates import (
    apply_vasp_config_overrides,
    get_vasp_scf_converge_allowed_keys,
    get_vasp_scf_converge_parameters_template,
)

try:
    import tomllib  # Python 3.11+
except ImportError:
    try:
        import toml
    except ImportError:
        print("Error: 'toml' or 'tomllib' (Python 3.11+) is required.")
        raise SystemExit(1)

def _is_numeric(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _walk_lists(node: Any, path: Tuple[str, ...] = ()) -> List[Tuple[Tuple[str, ...], List[Any]]]:
    results: List[Tuple[Tuple[str, ...], List[Any]]] = []
    if isinstance(node, dict):
        for key, value in node.items():
            results.extend(_walk_lists(value, path + (str(key),)))
    elif isinstance(node, list):
        results.append((path, node))
    return results


def find_unique_scan_parameter(config: Dict[str, Any]) -> Tuple[Tuple[str, ...], List[Any]]:
    list_entries = _walk_lists(config)
    if not list_entries:
        raise ValueError("收敛测试需要且仅需要一个列表参数。当前配置中未找到列表参数。")
    if len(list_entries) > 1:
        labels = ["/".join(path) for path, _ in list_entries]
        raise ValueError(
            "收敛测试仅允许一个列表参数，当前检测到多个列表参数: " + ", ".join(labels)
        )
    return list_entries[0]


def expand_scan_values(raw_values: List[Any]) -> List[Any]:
    if len(raw_values) == 3 and all(_is_numeric(value) for value in raw_values):
        start, stop, step = raw_values
        if step != 0:
            distance = stop - start
            if math.isclose(distance, 0.0, abs_tol=1e-12):
                return [start]
            if distance * step > 0:
                if abs(float(step)) - abs(float(distance)) > 1.0e-12:
                    return raw_values
                values: List[Any] = []
                current = float(start)
                step = float(step)
                stop = float(stop)
                tol = abs(step) * 1.0e-9 + 1.0e-12
                if step > 0:
                    while current <= stop + tol:
                        values.append(_normalize_numeric(current))
                        current += step
                else:
                    while current >= stop - tol:
                        values.append(_normalize_numeric(current))
                        current += step
                if values:
                    return values
    return raw_values


def _normalize_numeric(value: float) -> Any:
    rounded = round(value, 10)
    if math.isclose(rounded, round(rounded), abs_tol=1e-10):
        return int(round(rounded))
    return rounded


def _set_nested_value(config: Dict[str, Any], path: Tuple[str, ...], value: Any) -> None:
    target: Any = config
    for key in path[:-1]:
        target = target[key]
    target[path[-1]] = value


def _format_value_for_dir(value: Any) -> str:
    if isinstance(value, float):
        normalized = f"{value:.10g}"
    else:
        normalized = str(value)
    normalized = normalized.replace(os.sep, "_").replace(" ", "_")
    normalized = normalized.replace("/", "_")
    return normalized


def _read_total_atoms_from_poscar(poscar_path: str) -> int:
    if not os.path.exists(poscar_path):
        raise FileNotFoundError(f"未找到 POSCAR 文件: {poscar_path}")

    with open(poscar_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    if len(lines) < 7:
        raise ValueError(f"POSCAR 内容过短，无法解析原子数: {poscar_path}")

    tokens = lines[5].split()
    try:
        counts = [int(value) for value in tokens]
    except ValueError:
        if len(lines) < 8:
            raise ValueError(f"POSCAR 缺少原子数行: {poscar_path}")
        try:
            counts = [int(value) for value in lines[6].split()]
        except ValueError as exc:
            raise ValueError(f"无法解析 POSCAR 原子数行: {poscar_path}") from exc

    total_atoms = sum(counts)
    if total_atoms <= 0:
        raise ValueError(f"POSCAR 总原子数无效: {poscar_path}")
    return total_atoms


class VaspScfConvergeSetup:
    def __init__(self, config_file: str = "input.toml", struct_file: str = None):
        self.config_file = config_file
        self.struct_file = struct_file
        self.parameter_template = get_vasp_scf_converge_parameters_template()
        self.input_config = load_config_file(config_file)
        self.base_config = apply_vasp_config_overrides(
            self.parameter_template, self.input_config, get_vasp_scf_converge_allowed_keys()
        )
        self.scan_path, self.scan_raw_values = find_unique_scan_parameter(self.base_config["vasp_scf_converge"])
        self.scan_values = expand_scan_values(self.scan_raw_values)
        self.param_name = self.scan_path[-1]
        self.work_root = self.param_name

    def _build_case_runtime_config(self, value: Any) -> Dict[str, Any]:
        case_config = copy.deepcopy(self.base_config)
        _set_nested_value(case_config["vasp_scf_converge"], self.scan_path, value)
        case_config["vasp_scf"] = case_config.pop("vasp_scf_converge")
        return case_config

    def _build_case_dir(self, value: Any) -> str:
        return os.path.join(self.work_root, _format_value_for_dir(value))

    def _find_reference_poscar(self) -> str:
        for value in self.scan_values:
            poscar_path = os.path.join(self._build_case_dir(value), "POSCAR")
            if os.path.exists(poscar_path):
                return poscar_path

        if self.struct_file and os.path.exists(self.struct_file):
            fallback_path = os.path.join(self.work_root, "_plot_reference_POSCAR")
            os.makedirs(self.work_root, exist_ok=True)
            shutil.copy2(self.struct_file, fallback_path)
            return fallback_path

        raise FileNotFoundError("未找到可用于统计总原子数的 POSCAR。请先生成输入文件或提供有效结构文件。")

    def _extract_total_energy(self, outcar_path: str) -> float:
        if not os.path.exists(outcar_path):
            raise FileNotFoundError(f"未找到 OUTCAR 文件: {outcar_path}")

        command = f'grep -s "energy  without entropy" "{outcar_path}" | awk \'{{print $4}}\''
        output = os.popen(command).read().strip()
        if not output:
            raise ValueError(f"未能从 {outcar_path} 提取到 energy  without entropy")

        lines = [line.strip() for line in output.splitlines() if line.strip()]
        if not lines:
            raise ValueError(f"未能从 {outcar_path} 提取到有效能量数值")

        try:
            return float(lines[-1])
        except ValueError as exc:
            raise ValueError(f"无法将 {outcar_path} 中提取的能量转换为浮点数: {lines[-1]}") from exc

    def _collect_plot_data(self) -> Dict[str, Any]:
        total_atoms = _read_total_atoms_from_poscar(self._find_reference_poscar())
        points: List[Dict[str, Any]] = []

        for value in self.scan_values:
            outcar_path = os.path.join(self._build_case_dir(value), "OUTCAR")
            try:
                total_energy = self._extract_total_energy(outcar_path)
            except Exception as exc:
                print(f"警告: 跳过 {outcar_path}，原因: {exc}")
                continue

            points.append(
                {
                    "x": value,
                    "energy_per_atom": total_energy / total_atoms,
                    "outcar_path": outcar_path,
                }
            )

        if not points:
            raise RuntimeError("没有可用于绘图的有效 OUTCAR 能量数据。")

        reference_point = max(points, key=lambda item: item["x"])
        reference_energy = reference_point["energy_per_atom"]
        first_converged_point: Optional[Dict[str, Any]] = None

        for point in points:
            delta_mev = (point["energy_per_atom"] - reference_energy) * 1000.0
            point["delta_mev_per_atom"] = delta_mev
            point["within_1mev"] = abs(delta_mev) <= 1.0
            if first_converged_point is None and point["within_1mev"]:
                first_converged_point = point

        return {
            "points": points,
            "reference_point": reference_point,
            "reference_energy": reference_energy,
            "first_converged_point": first_converged_point,
        }

    def plot(self) -> str:
        try:
            import matplotlib.pyplot as plt
        except ImportError as exc:
            raise ImportError("缺少 matplotlib，无法绘制收敛曲线。") from exc

        plot_data = self._collect_plot_data()
        points = plot_data["points"]
        reference_point = plot_data["reference_point"]
        first_converged_point = plot_data["first_converged_point"]

        x_values = [point["x"] for point in points]
        total_energy_values = [point["energy_per_atom"] for point in points]
        delta_values = [point["delta_mev_per_atom"] for point in points]

        fig, (ax_energy, ax_delta) = plt.subplots(
            2,
            1,
            figsize=(8, 7),
            sharex=True,
            gridspec_kw={"height_ratios": [2, 1]},
        )

        ax_energy.plot(x_values, total_energy_values, marker="o", linewidth=1.5)
        ax_energy.set_ylabel("Energy per atom (eV/atom)")
        ax_energy.set_title(f"{self.param_name} convergence")
        ax_energy.grid(True, linestyle=":", alpha=0.5)
        ax_energy.text(
            0.98,
            0.98,
            f"reference = {self.param_name} {reference_point['x']}",
            transform=ax_energy.transAxes,
            ha="right",
            va="top",
            fontsize=9,
            bbox={"boxstyle": "round", "facecolor": "white", "alpha": 0.8, "edgecolor": "0.8"},
        )

        ax_delta.plot(x_values, delta_values, marker="o", linewidth=1.5, color="tab:red")
        ax_delta.axhline(1.0, color="tab:green", linestyle="--", linewidth=1.0)
        ax_delta.axhline(-1.0, color="tab:green", linestyle="--", linewidth=1.0)
        ax_delta.fill_between(x_values, -1.0, 1.0, color="tab:green", alpha=0.08)
        ax_delta.set_xlabel(self.param_name)
        ax_delta.set_ylabel("ΔE vs highest\n(meV/atom)")
        ax_delta.grid(True, linestyle=":", alpha=0.5)

        if first_converged_point is not None:
            ax_delta.scatter(
                [first_converged_point["x"]],
                [first_converged_point["delta_mev_per_atom"]],
                color="tab:blue",
                s=50,
                zorder=3,
            )
            ax_delta.annotate(
                f"{self.param_name} = {first_converged_point['x']}",
                xy=(first_converged_point["x"], first_converged_point["delta_mev_per_atom"]),
                xytext=(10, 12),
                textcoords="offset points",
                fontsize=9,
                color="tab:blue",
                arrowprops={"arrowstyle": "->", "color": "tab:blue", "lw": 0.8},
            )

        fig.tight_layout()

        output_path = os.path.abspath(f"converge_{self.param_name}.png")
        fig.savefig(output_path, dpi=300)
        plt.close(fig)
        print(f"已生成收敛曲线图: {output_path}")
        return output_path

    def setup(self) -> None:
        for value in self.scan_values:
            case_dir = self._build_case_dir(value)
            setup = VaspSetup(config_file=self.config_file, struct_file=self.struct_file)
            setup.config = self._build_case_runtime_config(value)
            setup.work_dir = case_dir
            setup.setup()
            print(f"已生成收敛测试目录: {case_dir}")

    def execute(self) -> None:
        for value in self.scan_values:
            case_dir = self._build_case_dir(value)
            setup = VaspSetup(config_file=self.config_file, struct_file=self.struct_file)
            setup.config = self._build_case_runtime_config(value)
            setup.work_dir = case_dir
            setup.execute()


def main():
    parser = argparse.ArgumentParser(description="VASP SCF 参数收敛测试脚本")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径 (默认为 input.toml)")
    parser.add_argument("-p", "--plot", action="store_true", help="读取已有 OUTCAR 并绘制收敛曲线")
    add_run_submit_arguments(parser, "生成文件后直接执行所有收敛测试", "生成文件后提交所有收敛测试")
    args = parser.parse_args()

    setup = VaspScfConvergeSetup(config_file=args.config, struct_file=args.input)
    if args.plot and not args.run and not args.submit:
        setup.plot()
        return

    setup.setup()
    handle_job_action(args.run, args.submit, setup.work_root, "", setup.execute)
    if args.plot:
        setup.plot()


if __name__ == "__main__":
    main()
