#!/usr/bin/env python3
import os
import re
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple


ATOMIC_MASSES = {
    "H": 1.008, "He": 4.0026, "Li": 6.94, "Be": 9.0122, "B": 10.81, "C": 12.011, "N": 14.007, "O": 15.999, "F": 18.998, "Ne": 20.180,
    "Na": 22.990, "Mg": 24.305, "Al": 26.982, "Si": 28.085, "P": 30.974, "S": 32.06, "Cl": 35.45, "Ar": 39.948,
    "K": 39.098, "Ca": 40.078, "Sc": 44.956, "Ti": 47.867, "V": 50.942, "Cr": 51.996, "Mn": 54.938, "Fe": 55.845, "Co": 58.933, "Ni": 58.693, "Cu": 63.546, "Zn": 65.38, "Ga": 69.723, "Ge": 72.63, "As": 74.922, "Se": 78.971, "Br": 79.904, "Kr": 83.798,
    "Rb": 85.468, "Sr": 87.62, "Y": 88.906, "Zr": 91.224, "Nb": 92.906, "Mo": 95.95, "Tc": 98, "Ru": 101.07, "Rh": 102.91, "Pd": 106.42, "Ag": 107.87, "Cd": 112.41, "In": 114.82, "Sn": 118.71, "Sb": 121.76, "Te": 127.60, "I": 126.90, "Xe": 131.29,
    "Cs": 132.91, "Ba": 137.33, "La": 138.91, "Ce": 140.12, "Pr": 140.91, "Nd": 144.24, "Pm": 145, "Sm": 150.36, "Eu": 151.96, "Gd": 157.25, "Tb": 158.93, "Dy": 162.50, "Ho": 164.93, "Er": 167.26, "Tm": 168.93, "Yb": 173.05, "Lu": 174.97, "Hf": 178.49, "Ta": 180.95, "W": 183.84, "Re": 186.21, "Os": 190.23, "Ir": 192.22, "Pt": 195.08, "Au": 196.97, "Hg": 200.59, "Tl": 204.38, "Pb": 207.2, "Bi": 208.98, "Th": 232.04, "Pa": 231.04, "U": 238.03
}


def get_atomic_mass(element: str, explicit_mass: Optional[float] = None) -> float:
    if explicit_mass is not None:
        return float(explicit_mass)
    return float(ATOMIC_MASSES.get(element, 1.0))


def extract_pseudo_element_token(filename: str) -> str:
    basename = os.path.basename(filename)
    match = re.match(r"([A-Za-z]+)", basename)
    return match.group(1) if match else ""


def find_pseudo_file(pp_dir: Optional[str], element: str) -> str:
    if not pp_dir:
        raise ValueError("缺少 pp_dir 配置，无法自动匹配赝势文件")
    expanded_pp_dir = os.path.expanduser(str(pp_dir))
    if not os.path.isdir(expanded_pp_dir):
        raise FileNotFoundError(f"找不到赝势目录: {expanded_pp_dir}")

    candidates: List[str] = []
    for entry in os.scandir(expanded_pp_dir):
        if not entry.is_file():
            continue
        token = extract_pseudo_element_token(entry.name)
        if token and token.lower() == element.lower():
            candidates.append(entry.name)

    if not candidates:
        raise FileNotFoundError(
            f"在 {expanded_pp_dir} 中找不到元素 {element} 对应的赝势文件"
        )
    if len(candidates) > 1:
        candidate_list = ", ".join(sorted(candidates))
        raise ValueError(
            f"元素 {element} 在 {expanded_pp_dir} 中匹配到多个赝势文件: {candidate_list}"
        )
    return candidates[0]


def resolve_pseudo_files(pp_dir: Optional[str], elements: Iterable[str]) -> Dict[str, str]:
    resolved: Dict[str, str] = {}
    for element in elements:
        if element in resolved:
            continue
        resolved[element] = find_pseudo_file(pp_dir, element)
    return resolved


def resolve_atomic_species_entries(
    config: Dict[str, Any],
    elements: Sequence[str],
) -> List[Tuple[str, float, str]]:
    pseudo_files = resolve_pseudo_files(config.get("pp_dir"), elements)
    return [
        (element, get_atomic_mass(element), pseudo_files[element])
        for element in elements
    ]
