#!/usr/bin/env python3
import os
import re
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np

from qeflow.phonon.archive import PHBAND_DIR, PHDOS_DIR
from qeflow.phonon.context import PhononStepContext
from qeflow.qe_input.phonon_input import derive_filenames


def build_phonon_band_tables(context: PhononStepContext) -> Tuple[List[Dict[str, object]], List[Dict[str, object]]]:
    label_path = os.path.join(context.workspace, PHBAND_DIR, f"{context.prefix}_band.labelinfo.dat")
    freq_path = os.path.join(context.workspace, PHBAND_DIR, derive_filenames(context.prefix)["flfrq"])
    labels = _read_labelinfo(label_path)
    if len(labels) < 1:
        raise ValueError("高对称点 labelinfo 为空。")
    frequencies = _read_band_frequencies(freq_path, int(np.sum(context.struct_info["counts"])) * 3)
    points = _interpolate_band_points(labels, len(frequencies))
    if len(points) != len(frequencies):
        raise ValueError(f"声子谱路径点数量与频率数量不一致: {len(points)} != {len(frequencies)}")

    band_rows: List[Dict[str, object]] = []
    for point, modes in zip(points, frequencies):
        row: Dict[str, object] = {
            "point_index": point["point_index"],
            "distance": point["distance"],
            "kx": point["kx"],
            "ky": point["ky"],
            "kz": point["kz"],
        }
        for mode_index, frequency in enumerate(modes, start=1):
            row[f"mode_{mode_index}_cm-1"] = frequency
        band_rows.append(row)

    label_rows = [
        {
            "label": item["label"],
            "point_index": item["point_index"],
            "distance": item["distance"],
            "kx": item["kx"],
            "ky": item["ky"],
            "kz": item["kz"],
        }
        for item in labels
    ]
    return band_rows, label_rows


def build_phonon_dos_tables(context: PhononStepContext) -> Tuple[List[Dict[str, object]], List[Dict[str, object]], List[str]]:
    dos_path = os.path.join(context.workspace, PHDOS_DIR, derive_filenames(context.prefix)["fldos"])
    numeric_rows = _read_numeric_rows(dos_path)
    if not numeric_rows or len(numeric_rows[0]) < 2:
        raise ValueError("声子 DOS 文件至少需要 frequency 与 TDOS 两列。")
    atom_names = _atom_names(context.struct_info["elements"], context.struct_info["counts"])
    max_atom_cols = max(0, len(numeric_rows[0]) - 2)
    selected_atom_names = atom_names[:max_atom_cols]
    atoms_header = ["frequency_cm-1", "TDOS", *selected_atom_names]

    dos_atoms_rows: List[Dict[str, object]] = []
    dos_elements_rows: List[Dict[str, object]] = []
    element_names = list(dict.fromkeys(str(element) for element in context.struct_info["elements"]))

    for values in numeric_rows:
        atoms_row: Dict[str, object] = {"frequency_cm-1": values[0], "TDOS": values[1]}
        atom_values = values[2 : 2 + len(selected_atom_names)]
        for name, value in zip(selected_atom_names, atom_values):
            atoms_row[name] = value
        dos_atoms_rows.append({key: atoms_row.get(key, "") for key in atoms_header})

        element_row: Dict[str, object] = {"frequency_cm-1": values[0], "TDOS": values[1]}
        for element in element_names:
            element_row[element] = 0.0
        for name, value in zip(selected_atom_names, atom_values):
            element = name.rsplit("_", 1)[0]
            element_row[element] = float(element_row.get(element, 0.0)) + float(value)
        dos_elements_rows.append(element_row)
    return dos_atoms_rows, dos_elements_rows, element_names


def _atom_names(elements: Sequence[str], counts: Sequence[int]) -> List[str]:
    names: List[str] = []
    for element, count in zip(elements, counts):
        for index in range(1, int(count) + 1):
            names.append(f"{element}_{index}")
    return names


def _read_labelinfo(path: str) -> List[Dict[str, object]]:
    labels: List[Dict[str, object]] = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            parts = line.split()
            if len(parts) < 6:
                continue
            labels.append(
                {
                    "label": parts[0].replace("GAMMA", "Γ"),
                    "point_index": int(parts[1]),
                    "distance": float(parts[2]),
                    "kx": float(parts[3]),
                    "ky": float(parts[4]),
                    "kz": float(parts[5]),
                }
            )
    return labels


def _interpolate_band_points(labels: List[Dict[str, object]], count: int) -> List[Dict[str, object]]:
    points: List[Dict[str, object]] = []
    for point_index in range(1, count + 1):
        left = labels[0]
        right = labels[-1]
        for index in range(len(labels) - 1):
            if int(labels[index]["point_index"]) <= point_index <= int(labels[index + 1]["point_index"]):
                left = labels[index]
                right = labels[index + 1]
                break
        left_index = int(left["point_index"])
        right_index = int(right["point_index"])
        fraction = 0.0 if right_index == left_index else (point_index - left_index) / (right_index - left_index)
        points.append(
            {
                "point_index": point_index,
                "distance": _lerp(float(left["distance"]), float(right["distance"]), fraction),
                "kx": _lerp(float(left["kx"]), float(right["kx"]), fraction),
                "ky": _lerp(float(left["ky"]), float(right["ky"]), fraction),
                "kz": _lerp(float(left["kz"]), float(right["kz"]), fraction),
            }
        )
    return points


def _lerp(left: float, right: float, fraction: float) -> float:
    return left + (right - left) * fraction


def _read_band_frequencies(path: str, expected_modes: int) -> List[List[float]]:
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        lines = handle.readlines()

    qe_plot_rows = _read_qe_plot_frequencies(lines, expected_modes, path)
    if qe_plot_rows is not None:
        return qe_plot_rows

    freq_pattern = re.compile(
        r"freq\s*\(\s*\d+\s*\)\s*=\s*[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?\s*(?:\[?[A-Za-z]+\]?)?\s*=\s*([-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?)",
        flags=re.IGNORECASE,
    )
    grouped: List[List[float]] = []
    current: List[float] = []
    numeric_rows: List[List[float]] = []
    for line in lines:
        match = freq_pattern.search(line)
        if match is not None:
            current.append(float(match.group(1)))
            if len(current) == expected_modes:
                grouped.append(current)
                current = []
            continue
        values = _float_values(line)
        if len(values) >= expected_modes:
            numeric_rows.append(values[-expected_modes:])
    if grouped:
        return grouped
    if current:
        raise ValueError("声子谱 freq 行数量不是模式数的整数倍。")
    if numeric_rows:
        return numeric_rows
    raise ValueError(f"无法从声子谱文件解析频率: {path}")


def _read_qe_plot_frequencies(lines: Sequence[str], expected_modes: int, path: str) -> Optional[List[List[float]]]:
    header_pattern = re.compile(r"&plot\s+nbnd\s*=\s*(\d+)\s*,\s*nks\s*=\s*(\d+)", flags=re.IGNORECASE)
    header_index = -1
    nbnd = 0
    nks = 0
    for index, line in enumerate(lines):
        match = header_pattern.search(line)
        if match is None:
            continue
        header_index = index
        nbnd = int(match.group(1))
        nks = int(match.group(2))
        break
    if header_index < 0:
        return None
    if nbnd != expected_modes:
        raise ValueError(f"声子谱模式数与结构不一致: 文件 nbnd={nbnd}，结构期望 {expected_modes}")

    rows: List[List[float]] = []
    line_index = header_index + 1
    for point_index in range(1, nks + 1):
        line_index = _next_nonempty_line(lines, line_index)
        if line_index >= len(lines):
            raise ValueError(f"声子谱文件缺少第 {point_index} 个 q 点坐标: {path}")
        coord_values = _float_values(lines[line_index])
        if len(coord_values) < 3:
            raise ValueError(f"声子谱文件第 {point_index} 个 q 点坐标无法解析: {lines[line_index].strip()}")
        line_index += 1

        modes: List[float] = []
        while len(modes) < nbnd:
            line_index = _next_nonempty_line(lines, line_index)
            if line_index >= len(lines):
                raise ValueError(f"声子谱文件第 {point_index} 个 q 点频率数量不足: 期望 {nbnd}，实际 {len(modes)}")
            modes.extend(_float_values(lines[line_index]))
            line_index += 1
        if len(modes) != nbnd:
            raise ValueError(f"声子谱文件第 {point_index} 个 q 点频率数量错误: 期望 {nbnd}，实际 {len(modes)}")
        rows.append(modes)

    if len(rows) != nks:
        raise ValueError(f"声子谱 q 点数量与 header 不一致: {len(rows)} != {nks}")
    return rows


def _next_nonempty_line(lines: Sequence[str], start_index: int) -> int:
    index = start_index
    while index < len(lines) and not lines[index].strip():
        index += 1
    return index


def _float_values(line: str) -> List[float]:
    return [float(item) for item in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?", line)]


def _read_numeric_rows(path: str) -> List[List[float]]:
    rows: List[List[float]] = []
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            values = _float_values(stripped)
            if values:
                rows.append(values)
    return rows
