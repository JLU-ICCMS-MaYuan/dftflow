#!/usr/bin/env python3
import os
import re
from typing import List, Tuple

import numpy as np

from qeflow.io_reader.common import is_float


def orbital_from_lm(l_val: int, m_val: int) -> str:
    if l_val == 0:
        return "s"
    if l_val == 1:
        return {1: "pz", 2: "px", 3: "py"}.get(m_val, f"p{m_val}")
    if l_val == 2:
        return {1: "dz2", 2: "dxz", 3: "dyz", 4: "dx2-y2", 5: "dxy"}.get(m_val, f"d{m_val}")
    if l_val == 3:
        return {
            1: "fz3",
            2: "fxz2",
            3: "fyz2",
            4: "fxyz",
            5: "fx2y2",
            6: "fy3x2-y2",
            7: "fzx2-y2",
        }.get(m_val, f"f{m_val}")
    return f"l{l_val}m{m_val}"


def read_cell_from_pw_input(path: str) -> np.ndarray:
    if not os.path.exists(path):
        raise FileNotFoundError(f"找不到 eband.in: {path}")
    with open(path, "r", encoding="utf-8") as handle:
        lines = handle.readlines()
    for idx, line in enumerate(lines):
        if line.strip().upper().startswith("CELL_PARAMETERS"):
            cell = []
            for offset in range(1, 4):
                if idx + offset >= len(lines):
                    raise ValueError("CELL_PARAMETERS 段不完整。")
                parts = lines[idx + offset].split()
                if len(parts) < 3:
                    raise ValueError("CELL_PARAMETERS 行格式错误。")
                cell.append([float(parts[0]), float(parts[1]), float(parts[2])])
            return np.array(cell, dtype=float)
    poscar_path = os.path.join(os.path.dirname(path) or ".", "POSCAR")
    if os.path.exists(poscar_path):
        with open(poscar_path, "r", encoding="utf-8") as handle:
            poscar_lines = handle.readlines()
        scale = float(poscar_lines[1].strip())
        return np.array(
            [[float(value) * scale for value in poscar_lines[index].split()[:3]] for index in range(2, 5)],
            dtype=float,
        )
    raise ValueError("eband.in 中未找到 CELL_PARAMETERS 段，且同目录没有 POSCAR。")


def read_kpath_points_from_pw_input(path: str) -> Tuple[np.ndarray, List[str]]:
    lines, kpoint_line = _read_crystal_b_kpoints(path)
    idx, num_points = _read_kpoint_count(lines, kpoint_line + 1)
    points = []
    labels = []
    for _ in range(num_points):
        idx = _next_nonempty_line(lines, idx)
        if idx >= len(lines):
            raise ValueError("K_POINTS 段数据不足。")
        line = lines[idx]
        idx += 1
        if "!" in line:
            coord_part, label_part = line.split("!", 1)
            label = label_part.strip()
        else:
            coord_part = line
            label = ""
        parts = coord_part.split()
        if len(parts) < 3:
            raise ValueError(f"K_POINTS 行格式错误: {line}")
        points.append([float(parts[0]), float(parts[1]), float(parts[2])])
        labels.append(label)
    return np.array(points, dtype=float), labels


def read_kpoints_weights_from_pw_input(path: str) -> Tuple[np.ndarray, List[int]]:
    lines, kpoint_line = _read_crystal_b_kpoints(path)
    idx, num_points = _read_kpoint_count(lines, kpoint_line + 1)
    weights = []
    points = []
    for _ in range(num_points):
        idx = _next_nonempty_line(lines, idx)
        if idx >= len(lines):
            raise ValueError("K_POINTS 段数据不足。")
        line = lines[idx]
        idx += 1
        coord_part = line.split("!", 1)[0]
        parts = coord_part.split()
        if len(parts) < 4:
            raise ValueError(f"K_POINTS 行格式错误: {line}")
        try:
            weight = int(float(parts[3]))
        except ValueError as exc:
            raise ValueError(f"K_POINTS 权重解析失败: {line}") from exc
        points.append([float(parts[0]), float(parts[1]), float(parts[2])])
        weights.append(weight)
    return np.array(points, dtype=float), weights


def parse_elebanddata(path: str):
    if not os.path.exists(path):
        raise FileNotFoundError(f"找不到能带数据文件: {path}")

    with open(path, "r", encoding="utf-8") as handle:
        lines = [line.rstrip() for line in handle if line.strip() != ""]

    header = lines[0]
    match = re.search(r"nbnd\s*=\s*(\d+).+nks\s*=\s*(\d+)", header)
    if not match:
        raise ValueError("无法从 elebanddata.dat 解析 nbnd/nks。")
    nbnd = int(match.group(1))
    nks = int(match.group(2))

    coords = []
    energies = []
    idx = 1
    while idx < len(lines) and len(coords) < nks:
        parts = lines[idx].split()
        if len(parts) >= 3 and all(is_float(part) for part in parts[:3]):
            coords.append([float(parts[0]), float(parts[1]), float(parts[2])])
            idx += 1
            band_vals = []
            while idx < len(lines) and len(band_vals) < nbnd:
                vals = [float(value) for value in lines[idx].split() if is_float(value)]
                band_vals.extend(vals)
                idx += 1
            energies.append(band_vals[:nbnd])
        else:
            idx += 1

    if len(coords) != nks or len(energies) != nks:
        raise ValueError("elebanddata.dat 解析的 k 点数量与头部不一致。")

    return np.array(coords), np.array(energies), nbnd, nks


def parse_projwfc_file(path: str):
    with open(path, "r", encoding="utf-8") as handle:
        lines = [line.rstrip() for line in handle]

    header_idx = None
    nproj = nk = nbnd = None
    for index, line in enumerate(lines):
        parts = line.split()
        if len(parts) == 3 and all(part.isdigit() for part in parts):
            if index + 1 < len(lines) and re.match(r"^[FT](\s+[FT])*$", lines[index + 1].strip(), re.IGNORECASE):
                nproj, nk, nbnd = map(int, parts)
                header_idx = index + 2
                break
    if header_idx is None:
        return None

    labels = []
    proj_weights = np.zeros((nproj, nk, nbnd))

    idx = header_idx
    for proj_idx in range(nproj):
        idx = _next_nonempty_line(lines, idx)
        if idx >= len(lines):
            raise ValueError("projwfc 文件结构不完整，缺少投影头部。")
        tokens = lines[idx].split()
        idx += 1
        if len(tokens) < 6:
            raise ValueError(f"投影头部解析失败: {lines[idx - 1]}")

        atom_index = tokens[1]
        element = tokens[2]
        shell_token = None
        for token in tokens:
            if re.search(r"[SPDFspdf]$", token) and re.search(r"\d", token):
                shell_token = token
                break
        trailing_ints = [int(token) for token in tokens if token.isdigit()]
        if len(trailing_ints) < 2:
            raise ValueError(f"无法解析 l/m: {lines[idx - 1]}")
        l_val = trailing_ints[-2]
        m_val = trailing_ints[-1]

        orbital = orbital_from_lm(l_val, m_val)
        if shell_token:
            shell_label = shell_token.strip().lower()
            base_letter = shell_label[-1]
            suffix = orbital[1:] if orbital.startswith(base_letter) else orbital
            label = f"{element}{atom_index}-{shell_label}{suffix}"
        else:
            label = f"{element}{atom_index}-{orbital}"
        labels.append(label)

        for _ in range(nk * nbnd):
            if idx >= len(lines):
                raise ValueError("projwfc 文件数据长度不足。")
            parts = lines[idx].split()
            idx += 1
            if len(parts) < 3:
                continue
            k_idx = int(parts[0]) - 1
            b_idx = int(parts[1]) - 1
            proj_weights[proj_idx, k_idx, b_idx] = float(parts[2])

    return labels, proj_weights, nk, nbnd


def _read_crystal_b_kpoints(path: str):
    if not os.path.exists(path):
        raise FileNotFoundError(f"找不到 eband.in: {path}")
    with open(path, "r", encoding="utf-8") as handle:
        lines = handle.readlines()
    for idx, line in enumerate(lines):
        if line.strip().upper().startswith("K_POINTS"):
            if "CRYSTAL_B" not in line.upper():
                raise ValueError("K_POINTS 段不是 crystal_b 格式。")
            return lines, idx
    raise ValueError("eband.in 中未找到 K_POINTS 段。")


def _read_kpoint_count(lines: List[str], start_index: int):
    idx = _next_nonempty_line(lines, start_index)
    if idx >= len(lines):
        raise ValueError("K_POINTS 段缺少点数。")
    try:
        num_points = int(lines[idx].split()[0])
    except ValueError as exc:
        raise ValueError("K_POINTS 点数解析失败。") from exc
    return idx + 1, num_points


def _next_nonempty_line(lines: List[str], start_index: int) -> int:
    idx = start_index
    while idx < len(lines) and not lines[idx].strip():
        idx += 1
    return idx

