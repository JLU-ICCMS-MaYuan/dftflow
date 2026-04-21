#!/usr/bin/env python3
import argparse
import csv
import glob
import json
import os
import re
from collections import OrderedDict
from typing import Dict, Iterable, List, Optional, Sequence, Tuple

from substitutionflow.selection import PERIOD_TABLE_LAYOUT, SERIES_TABLE_LAYOUT


XA_DIR_PATTERN = re.compile(r"^\d+\.XA_[A-Z][a-z]?$")
ROW_DISPLAY_ORDER = tuple(PERIOD_TABLE_LAYOUT.keys()) + tuple(SERIES_TABLE_LAYOUT.keys())
BAR_LABELS = ("XA", "XB", "H", "TDOS")
BAR_COLORS = {
    "XA": "#4C78A8",
    "XB": "#F58518",
    "H": "#54A24B",
    "TDOS": "#E45756",
}
FIGURE_DPI = 300
MISSING_FACE_COLOR = "#ECECEC"


def parse_arguments():
    parser = argparse.ArgumentParser(description="为 substitutionflow 批量 DOS 结果生成周期表式总览图。")
    parser.add_argument(
        "-d",
        "--directory",
        nargs="*",
        default=[],
        help="统计目录；省略时默认扫描当前目录下全部 [Z].XA_* 目录。",
    )
    return parser.parse_args()


def _load_json(path: str) -> Dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _load_csv_row(path: str) -> Dict[str, float]:
    with open(path, "r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        row = next(reader, None)
    if row is None:
        raise ValueError(f"{path} 不包含数据行。")
    return {key: float(value) for key, value in row.items() if key}


def _is_xa_dir(path: str) -> bool:
    return os.path.isdir(path) and XA_DIR_PATTERN.match(os.path.basename(os.path.abspath(path))) is not None


def discover_xa_dirs(root_dir: str) -> List[str]:
    root_dir = os.path.abspath(root_dir)
    result = []
    for entry in sorted(os.listdir(root_dir)):
        path = os.path.join(root_dir, entry)
        if _is_xa_dir(path):
            result.append(path)
    if not result:
        raise ValueError(f"在 {root_dir} 下未找到任何 [Z].XA_[元素] 目录。")
    return result


def resolve_target_xa_dirs(directory_args: Sequence[str]) -> List[str]:
    if not directory_args:
        return discover_xa_dirs(os.getcwd())

    abs_paths = [os.path.abspath(path) for path in directory_args]
    xa_flags = [_is_xa_dir(path) for path in abs_paths]
    if len(abs_paths) == 1 and not xa_flags[0]:
        return discover_xa_dirs(abs_paths[0])
    if all(xa_flags):
        return abs_paths
    raise ValueError("`-d/--directory` 只能传一个批量根目录，或一个/多个 [Z].XA_[元素] 目录，不能混传。")


def _matching_positions(symbol: str) -> List[Tuple[str, int]]:
    matches = []
    for row_label, layout in PERIOD_TABLE_LAYOUT.items():
        for column, current in layout.items():
            if current == symbol:
                matches.append((row_label, column))
    for row_label, layout in SERIES_TABLE_LAYOUT.items():
        for column, current in layout.items():
            if current == symbol:
                matches.append((row_label, column))
    if not matches:
        raise ValueError(f"无法在周期表布局中定位元素 {symbol}。")
    return matches


def discover_case_entries(xa_dir: str) -> List[Dict]:
    entries: List[Dict] = []
    manifest_paths = sorted(glob.glob(os.path.join(os.path.abspath(xa_dir), "period_*", "workflow_manifest.json")))
    for manifest_path in manifest_paths:
        manifest = _load_json(manifest_path)
        period_dir = os.path.dirname(manifest_path)
        for case_entry in manifest.get("cases", []):
            case_dir = os.path.join(period_dir, case_entry["case_dir"])
            position_list = _matching_positions(case_entry["xb_symbol"])
            entries.append(
                {
                    "case_dir": case_dir,
                    "formula": case_entry["formula"],
                    "xa_symbol": manifest["xa_symbol"],
                    "xb_symbol": case_entry["xb_symbol"],
                    "pressures": [f"{pressure:g}" if isinstance(pressure, (int, float)) else str(pressure) for pressure in manifest["pressures"]],
                    "positions": position_list,
                }
            )
    if not entries:
        raise ValueError(f"在 {xa_dir} 下未找到任何 workflow_manifest.json。")
    return entries


def discover_pressures(case_entries: Iterable[Dict]) -> List[str]:
    pressures = OrderedDict()
    for entry in case_entries:
        for pressure in entry["pressures"]:
            pressures[pressure] = None
    return list(pressures.keys())


def _read_case_step_status(case_dir: str, pressure: str) -> Optional[str]:
    path = os.path.join(case_dir, "case_status.json")
    if not os.path.exists(path):
        return None
    try:
        payload = _load_json(path)
    except Exception:
        return None
    return payload.get("pressures", {}).get(pressure, {}).get("vasp_process_eledos")


def _find_at_ef_csv(case_dir: str, pressure: str) -> Optional[str]:
    pattern = os.path.join(case_dir, pressure, "vasp_eledos", "*_at_ef.csv")
    candidates = [path for path in sorted(glob.glob(pattern)) if os.path.exists(path) and os.path.getsize(path) > 0]
    if not candidates:
        return None
    if len(candidates) > 1:
        raise ValueError(f"{os.path.dirname(pattern)} 下找到多个 *_at_ef.csv，无法确定使用哪个。")
    return candidates[0]


def load_case_plot_record(case_entry: Dict, pressure: str) -> Dict:
    status = _read_case_step_status(case_entry["case_dir"], pressure)
    csv_path = _find_at_ef_csv(case_entry["case_dir"], pressure)
    if csv_path is None:
        return {
            "status": status or "missing",
            "values": None,
        }

    row = _load_csv_row(csv_path)
    xa_symbol = case_entry["xa_symbol"]
    xb_symbol = case_entry["xb_symbol"]
    required = [xa_symbol, xb_symbol, "H", "TDOS"]
    if any(name not in row for name in required):
        return {
            "status": "missing_columns",
            "values": None,
        }
    return {
        "status": "finished",
        "values": OrderedDict(
            [
                ("XA", row[xa_symbol]),
                ("XB", row[xb_symbol]),
                ("H", row["H"]),
                ("TDOS", row["TDOS"]),
            ]
        ),
    }


def collect_pressure_plot_data(xa_dir: str, pressure: str) -> Dict:
    entries = discover_case_entries(xa_dir)
    row_labels = OrderedDict()
    cells: Dict[Tuple[str, int], Dict] = {}

    for entry in entries:
        record = load_case_plot_record(entry, pressure)
        for row_label, column in entry["positions"]:
            row_labels[row_label] = None
            cells[(row_label, column)] = {
                "xb_symbol": entry["xb_symbol"],
                "formula": entry["formula"],
                "status": record["status"],
                "values": record["values"],
            }

    ordered_rows = [row_label for row_label in ROW_DISPLAY_ORDER if row_label in row_labels]
    ymax = 1.0
    for payload in cells.values():
        if payload["values"]:
            ymax = max(ymax, max(payload["values"].values()) * 1.1)

    return {
        "xa_dir": os.path.abspath(xa_dir),
        "pressure": pressure,
        "rows": ordered_rows,
        "cells": cells,
        "ymax": ymax,
    }


def plot_pressure_summary(plot_data: Dict) -> str:
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        from matplotlib.patches import Patch
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制 substitution_plot_eledos 图。") from exc

    rows = plot_data["rows"]
    if not rows:
        raise ValueError(f"{plot_data['xa_dir']} 在 pressure={plot_data['pressure']} 下没有可绘制行。")

    fig_width = 24
    fig_height = max(2.5 * len(rows), 4.0)
    fig, axes = plt.subplots(len(rows), 18, figsize=(fig_width, fig_height), squeeze=False)
    fig.subplots_adjust(left=0.08, right=0.98, top=0.90, bottom=0.10, wspace=0.35, hspace=0.90)

    legend_handles = [Patch(facecolor=BAR_COLORS[label], edgecolor="none", label=label) for label in BAR_LABELS]
    plotted_any = False
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

            ax.set_facecolor("white")
            ax.set_ylim(0.0, plot_data["ymax"])
            ax.set_xlim(-0.6, 3.6)
            ax.set_title(payload["xb_symbol"], fontsize=8, pad=2)

            if payload["values"] is None:
                ax.set_facecolor(MISSING_FACE_COLOR)
                ax.text(0.5, 0.5, "unfinished", ha="center", va="center", fontsize=7, transform=ax.transAxes)
                continue

            values = [payload["values"][label] for label in BAR_LABELS]
            ax.bar(range(len(BAR_LABELS)), values, color=[BAR_COLORS[label] for label in BAR_LABELS], width=0.72)
            plotted_any = True

    fig.legend(handles=legend_handles, loc="upper center", ncol=4, frameon=False)
    fig.suptitle(f"{os.path.basename(plot_data['xa_dir'])} | pressure = {plot_data['pressure']} GPa", fontsize=14)

    output_path = os.path.join(plot_data["xa_dir"], f"eledos_summary_{plot_data['pressure']}.png")
    fig.savefig(output_path, dpi=FIGURE_DPI)
    plt.close(fig)
    if not plotted_any:
        # 仍输出图，但给调用方一个可见信号用于日志。
        print(f"警告: {plot_data['xa_dir']} 在 pressure={plot_data['pressure']} 下没有有效 DOS 数据，已生成空框架图。")
    return output_path


def generate_xa_dir_plots(xa_dir: str) -> List[str]:
    entries = discover_case_entries(xa_dir)
    outputs = []
    for pressure in discover_pressures(entries):
        outputs.append(plot_pressure_summary(collect_pressure_plot_data(xa_dir, pressure)))
    return outputs


def run(directory_args: Sequence[str]) -> List[str]:
    outputs = []
    for xa_dir in resolve_target_xa_dirs(directory_args):
        outputs.extend(generate_xa_dir_plots(xa_dir))
    return outputs


def main():
    args = parse_arguments()
    outputs = run(args.directory)
    for path in outputs:
        print(path)


if __name__ == "__main__":
    main()
