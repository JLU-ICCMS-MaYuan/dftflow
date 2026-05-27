#!/usr/bin/env python3
import os
from typing import Any, Dict, List, Sequence

import numpy as np


def get_formula(elements: Sequence[str], counts: Sequence[int]) -> str:
    formula = ""
    for element, count in zip(elements, counts):
        formula += str(element)
        if int(count) >= 1:
            formula += str(int(count))
    return formula


def parse_poscar(poscar_path: str) -> Dict[str, Any]:
    if not os.path.exists(poscar_path):
        raise FileNotFoundError(f"找不到 {poscar_path} 文件")
    with open(poscar_path, "r", encoding="utf-8") as handle:
        lines = handle.readlines()
    scale = float(lines[1].strip())
    lattice = []
    for index in range(2, 5):
        lattice.append([float(item) * scale for item in lines[index].split()])
    elements = lines[5].split()
    if elements[0].isdigit():
        elements = lines[0].split()
        if not elements:
            raise ValueError("无法在 POSCAR 中找到元素名称。")
    counts = [int(item) for item in lines[6].split()]
    coord_line = lines[7].strip()
    if coord_line.lower().startswith("s"):
        coord_line = lines[8].strip()
        start_line = 9
    else:
        start_line = 8
    total_atoms = sum(counts)
    positions = []
    for index in range(total_atoms):
        positions.append(lines[start_line + index].split()[:3])
    pos_array = np.array(positions, dtype=float)
    if not coord_line.lower().startswith("d"):
        pos_array *= scale
    return {
        "lattice": np.array(lattice),
        "elements": elements,
        "counts": counts,
        "coord_type": coord_line,
        "positions": pos_array,
    }


def get_kpoints_from_spacing(lattice: np.ndarray, kmesh: Any, default_spacing: float) -> List[int]:
    if isinstance(kmesh, list) and len(kmesh) == 3:
        return [int(kmesh[0]), int(kmesh[1]), int(kmesh[2])]
    reciprocal = 2 * np.pi * np.linalg.inv(lattice).T
    lengths = np.linalg.norm(reciprocal, axis=1)
    spacing = float(kmesh) if isinstance(kmesh, (int, float)) else default_spacing
    return [int(max(1, np.ceil(length / (spacing * 2 * np.pi)))) for length in lengths]
