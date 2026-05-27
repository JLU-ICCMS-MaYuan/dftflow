#!/usr/bin/env python3
import os
import re
from dataclasses import dataclass
from typing import Dict, List, Optional, Sequence

from qeflow.io_reader.common import float_values, has_error_marker, read_text_if_exists


@dataclass
class Q2rOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return read_text_if_exists(self.path)

    def is_finished(self) -> bool:
        return "JOB DONE" in self.text()

    def has_error(self) -> bool:
        text = self.text()
        return bool(text) and has_error_marker(text)


@dataclass
class MatdynOut(Q2rOut):
    pass


def read_labelinfo(path: str) -> List[Dict[str, object]]:
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


def read_band_frequencies(path: str, expected_modes: int) -> List[List[float]]:
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
        values = float_values(line)
        if len(values) >= expected_modes:
            numeric_rows.append(values[-expected_modes:])
    if grouped:
        return grouped
    if current:
        raise ValueError("声子谱 freq 行数量不是模式数的整数倍。")
    if numeric_rows:
        return numeric_rows
    raise ValueError(f"无法从声子谱文件解析频率: {path}")


def read_numeric_rows(path: str) -> List[List[float]]:
    rows: List[List[float]] = []
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            values = float_values(stripped)
            if values:
                rows.append(values)
    return rows


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
        coord_values = float_values(lines[line_index])
        if len(coord_values) < 3:
            raise ValueError(f"声子谱文件第 {point_index} 个 q 点坐标无法解析: {lines[line_index].strip()}")
        line_index += 1

        modes: List[float] = []
        while len(modes) < nbnd:
            line_index = _next_nonempty_line(lines, line_index)
            if line_index >= len(lines):
                raise ValueError(f"声子谱文件第 {point_index} 个 q 点频率数量不足: 期望 {nbnd}，实际 {len(modes)}")
            modes.extend(float_values(lines[line_index]))
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

