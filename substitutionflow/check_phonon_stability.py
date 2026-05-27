#!/usr/bin/env python3
import argparse
import csv
import os
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, List, Optional, Sequence, Tuple

from substitutionflow.plot_eledos import (
    ROW_DISPLAY_ORDER,
    _print_compact_progress,
    discover_case_entries,
    discover_pressures,
    resolve_target_xa_dirs,
)
from substitutionflow.status import get_case_phonon_workdir


CM_INV_PER_THZ = 33.35641
FIGURE_DPI = 300
STATUS_COLORS = {
    "stable": "#4C9F70",
    "unstable": "#D95F59",
    "unfinished": "#ECECEC",
}
STATUS_TEXT_COLORS = {
    "stable": "white",
    "unstable": "white",
    "unfinished": "#555555",
}


def parse_arguments():
    parser = argparse.ArgumentParser(description="检查 substitutionflow 批量声子结果是否稳定，并生成周期表式总览图。")
    parser.add_argument(
        "-d",
        "--directory",
        nargs="*",
        default=[],
        help="统计目录；省略时默认扫描当前目录下全部 [Z].XA_* 目录。",
    )
    parser.add_argument(
        "-j",
        "--jobs",
        type=int,
        default=None,
        help="并行读取 case 结果的线程数；默认按 CPU 数自动选择。",
    )
    return parser.parse_args()


def _phonon_dos_elements_path(case_dir: str, pressure: str) -> str:
    workdir = get_case_phonon_workdir(case_dir)
    return os.path.join(case_dir, pressure, workdir, "phdos", "phonon_dos_elements.csv")


def load_case_stability_record(case_entry: Dict, pressure: str) -> Dict:
    csv_path = _phonon_dos_elements_path(case_entry["case_dir"], pressure)
    try:
        min_frequency_cm = _read_min_frequency_cm(csv_path)
    except Exception:
        return {
            "status": "unfinished",
            "min_frequency_THz": None,
        }

    min_frequency_thz = min_frequency_cm / CM_INV_PER_THZ
    return {
        "status": "unstable" if min_frequency_thz < -1.0 else "stable",
        "min_frequency_THz": min_frequency_thz,
    }


def _read_min_frequency_cm(path: str) -> float:
    with open(path, "r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames is None or "frequency_cm-1" not in reader.fieldnames:
            raise ValueError(f"{path} 缺少 frequency_cm-1 列。")
        values = []
        for row in reader:
            value = row.get("frequency_cm-1")
            if value is None or value == "":
                continue
            values.append(float(value))
    if not values:
        raise ValueError(f"{path} 不包含有效 frequency_cm-1 数据。")
    return min(values)


def collect_pressure_stability_data(xa_dir: str, pressure: str, jobs: Optional[int] = None) -> Dict:
    entries = discover_case_entries(xa_dir)
    row_labels = {}
    cells: Dict[Tuple[str, int], Dict] = {}

    for entry, record in _load_pressure_records(entries, pressure, jobs):
        for row_label, column in entry["positions"]:
            row_labels[row_label] = None
            cells[(row_label, column)] = {
                "xb_symbol": entry["xb_symbol"],
                "formula": entry["formula"],
                "status": record["status"],
                "min_frequency_THz": record["min_frequency_THz"],
            }

    ordered_rows = [row_label for row_label in ROW_DISPLAY_ORDER if row_label in row_labels]
    return {
        "xa_dir": os.path.abspath(xa_dir),
        "pressure": pressure,
        "rows": ordered_rows,
        "cells": cells,
    }


def _load_pressure_records(entries: Sequence[Dict], pressure: str, jobs: Optional[int]) -> List[Tuple[Dict, Dict]]:
    if not entries:
        return []
    max_workers = _resolve_worker_count(jobs, len(entries))
    if max_workers <= 1:
        return [(entry, load_case_stability_record(entry, pressure)) for entry in entries]

    indexed_results: Dict[int, Tuple[Dict, Dict]] = {}
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_map = {
            executor.submit(load_case_stability_record, entry, pressure): (index, entry)
            for index, entry in enumerate(entries)
        }
        for future in as_completed(future_map):
            index, entry = future_map[future]
            indexed_results[index] = (entry, future.result())
    return [indexed_results[index] for index in range(len(entries))]


def _resolve_worker_count(jobs: Optional[int], task_count: int) -> int:
    if jobs is not None:
        return max(1, min(int(jobs), task_count))
    cpu_count = os.cpu_count() or 1
    return max(1, min(task_count, cpu_count + 4, 32))


def plot_pressure_stability(plot_data: Dict) -> str:
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        from matplotlib.patches import Patch
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 phonon stability 图。") from exc

    rows = plot_data["rows"]
    if not rows:
        raise ValueError(f"{plot_data['xa_dir']} 在 pressure={plot_data['pressure']} 下没有可绘制行。")

    fig_width = 24
    fig_height = max(2.5 * len(rows), 4.0)
    fig, axes = plt.subplots(len(rows), 18, figsize=(fig_width, fig_height), squeeze=False)
    fig.subplots_adjust(left=0.08, right=0.98, top=0.90, bottom=0.10, wspace=0.35, hspace=0.90)

    legend_handles = [
        Patch(facecolor=STATUS_COLORS[status], edgecolor="0.7", label=status)
        for status in ("stable", "unstable", "unfinished")
    ]
    for row_index, row_label in enumerate(rows):
        fig.text(0.02, 1.0 - (row_index + 0.5) / len(rows), row_label, va="center", ha="left", fontsize=10)
        for column_index in range(18):
            ax = axes[row_index][column_index]
            payload = plot_data["cells"].get((row_label, column_index + 1))
            ax.set_xticks([])
            ax.set_yticks([])
            for spine in ax.spines.values():
                spine.set_visible(False)
            if payload is None:
                ax.axis("off")
                continue

            status = payload["status"]
            ax.set_facecolor(STATUS_COLORS[status])
            text_color = STATUS_TEXT_COLORS[status]
            ax.text(0.5, 0.70, payload["xb_symbol"], ha="center", va="center", fontsize=10, fontweight="bold", color=text_color)
            ax.text(0.5, 0.48, status, ha="center", va="center", fontsize=7, color=text_color)
            ax.text(0.5, 0.28, _format_min_frequency(payload["min_frequency_THz"]), ha="center", va="center", fontsize=6.5, color=text_color)

    fig.legend(handles=legend_handles, loc="upper center", ncol=3, frameon=False)
    fig.suptitle(f"{os.path.basename(plot_data['xa_dir'])} | pressure = {plot_data['pressure']} GPa", fontsize=14)

    output_path = os.path.join(plot_data["xa_dir"], f"phonon_stability_summary_{plot_data['pressure']}.png")
    fig.savefig(output_path, dpi=FIGURE_DPI)
    plt.close(fig)
    return output_path


def _format_min_frequency(value: Optional[float]) -> str:
    if value is None:
        return "min = n/a"
    return f"min = {value:.2f} THz"


def generate_xa_dir_stability_plots(
    xa_dir: str,
    show_progress: bool = False,
    progress_offset: int = 0,
    progress_total: Optional[int] = None,
    jobs: Optional[int] = None,
) -> List[str]:
    entries = discover_case_entries(xa_dir)
    pressures = discover_pressures(entries)
    outputs = []
    total = progress_total or len(pressures)
    for index, pressure in enumerate(pressures, start=1):
        if show_progress:
            current = progress_offset + index
            label = f"{os.path.basename(os.path.abspath(xa_dir))} pressure={pressure}"
            _print_compact_progress("检查 phonon stability", current, total, label)
        plot_data = collect_pressure_stability_data(xa_dir, pressure, jobs=jobs)
        outputs.append(plot_pressure_stability(plot_data))
    return outputs


def run(directory_args: Sequence[str], jobs: Optional[int] = None) -> List[str]:
    outputs = []
    xa_dirs = resolve_target_xa_dirs(directory_args)
    pressure_counts = [len(discover_pressures(discover_case_entries(xa_dir))) for xa_dir in xa_dirs]
    progress_total = sum(pressure_counts)
    progress_offset = 0
    for xa_dir, pressure_count in zip(xa_dirs, pressure_counts):
        outputs.extend(
            generate_xa_dir_stability_plots(
                xa_dir,
                show_progress=True,
                progress_offset=progress_offset,
                progress_total=progress_total,
                jobs=jobs,
            )
        )
        progress_offset += pressure_count
    return outputs


def main():
    args = parse_arguments()
    outputs = run(args.directory, jobs=args.jobs)
    for path in outputs:
        print(path)
