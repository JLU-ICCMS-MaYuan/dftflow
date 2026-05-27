#!/usr/bin/env python3
import os
import re
from typing import Dict, List, Optional, Tuple

import numpy as np

from qeflow.io_reader.common import is_float


def parse_tdos(path: str) -> Tuple[np.ndarray, np.ndarray, np.ndarray, Optional[float]]:
    energies = []
    tdos = []
    itdos = []
    fermi = None
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            if line.strip().startswith("#"):
                match = re.search(r"EFermi\s*=\s*([-\d.]+)", line)
                if match:
                    fermi = float(match.group(1))
                continue
            parts = line.split()
            if len(parts) >= 3 and is_float(parts[0]):
                energies.append(float(parts[0]))
                tdos.append(float(parts[1]))
                itdos.append(float(parts[2]))
    if not energies:
        raise ValueError(f"未从 {path} 解析到 TDOS 数据。")
    return np.array(energies), np.array(tdos), np.array(itdos), fermi


def parse_projwfc_shells(path: str) -> Dict[int, List[str]]:
    if not path or not os.path.exists(path):
        return {}
    mapping: Dict[int, List[str]] = {}
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            match = re.match(
                r"^\s*\d+\s+(\d+)\s+([A-Za-z]{1,2})\s+(\d+)([SPDF])\s+",
                line,
            )
            if not match:
                continue
            atom_index = int(match.group(1))
            shell_label = f"{match.group(3)}{match.group(4).lower()}"
            mapping.setdefault(atom_index, [])
            if shell_label not in mapping[atom_index]:
                mapping[atom_index].append(shell_label)
    return mapping


def parse_pdos_filename(path: str):
    name = os.path.basename(path)
    match = re.search(
        r"pdos_atm#(\d+)\(([A-Za-z]{1,2})\)_wfc#(\d+)\(([spdf])\)",
        name,
    )
    if not match:
        return None
    atom_index = int(match.group(1))
    element = match.group(2)
    wfc_index = int(match.group(3))
    l_letter = match.group(4).lower()
    return atom_index, element, wfc_index, l_letter


def orbital_components(l_letter: str) -> List[str]:
    if l_letter == "s":
        return ["s"]
    if l_letter == "p":
        return ["pz", "px", "py"]
    if l_letter == "d":
        return ["dz2", "dxz", "dyz", "dx2-y2", "dxy"]
    if l_letter == "f":
        return ["fz3", "fxz2", "fyz2", "fxyz", "fx2y2", "fy3x2-y2", "fzx2-y2"]
    return [l_letter]

