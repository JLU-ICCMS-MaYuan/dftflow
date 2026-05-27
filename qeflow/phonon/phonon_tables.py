#!/usr/bin/env python3
import os
from typing import Dict, List, Sequence, Tuple

import numpy as np

from qeflow.phonon.archive import PHBAND_DIR, PHDOS_DIR
from qeflow.phonon.context import PhononStepContext
from qeflow.io_reader.matdyn import read_band_frequencies, read_labelinfo, read_numeric_rows
from qeflow.input_deck.filenames import derive_filenames


def build_phonon_band_tables(context: PhononStepContext) -> Tuple[List[Dict[str, object]], List[Dict[str, object]]]:
    label_path = os.path.join(context.workspace, PHBAND_DIR, f"{context.prefix}_band.labelinfo.dat")
    freq_path = os.path.join(context.workspace, PHBAND_DIR, derive_filenames(context.prefix)["flfrq"])
    labels = read_labelinfo(label_path)
    if len(labels) < 1:
        raise ValueError("高对称点 labelinfo 为空。")
    frequencies = read_band_frequencies(freq_path, int(np.sum(context.struct_info["counts"])) * 3)
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
    numeric_rows = read_numeric_rows(dos_path)
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
