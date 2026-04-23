#!/usr/bin/env python3
import json
import math
import os
import xml.etree.ElementTree as ET
from typing import Dict, List, Sequence


EPS = 1.0e-5


def _parse_vector_text(text: str) -> List[float]:
    values = [float(item) for item in text.replace(",", " ").split()]
    if len(values) != 3:
        raise ValueError(f"无法解析 3 维向量: {text!r}")
    return values


def _parse_matrix_text(text: str) -> List[List[int]]:
    values = [int(round(float(item))) for item in text.replace(",", " ").split()]
    if len(values) != 9:
        raise ValueError(f"无法解析 3x3 矩阵: {text!r}")
    return [values[0:3], values[3:6], values[6:9]]


def _find_first_vector(root: ET.Element, candidate_tags: Sequence[str]) -> List[float]:
    lower_targets = tuple(tag.lower() for tag in candidate_tags)
    for element in root.iter():
        tag_name = element.tag.split("}")[-1].lower()
        if tag_name in lower_targets and (element.text or "").strip():
            return _parse_vector_text(element.text or "")
    raise ValueError(f"XML 中找不到向量标签: {candidate_tags}")


def _find_rotation_matrices(root: ET.Element) -> List[List[List[int]]]:
    matrices: List[List[List[int]]] = []
    for element in root.iter():
        tag_name = element.tag.split("}")[-1].lower()
        if tag_name in {"rotation", "rotation_matrix", "symmetry_matrix"} and (element.text or "").strip():
            matrices.append(_parse_matrix_text(element.text or ""))
    if not matrices:
        raise ValueError("XML 中未找到任何 symmetry rotation matrix。")
    return matrices


def _find_boolean(root: ET.Element, candidate_tags: Sequence[str], default: bool) -> bool:
    lower_targets = tuple(tag.lower() for tag in candidate_tags)
    for element in root.iter():
        tag_name = element.tag.split("}")[-1].lower()
        if tag_name in lower_targets and (element.text or "").strip():
            value = (element.text or "").strip().lower()
            return value in {"true", "t", "1", "yes"}
    return default


def parse_qgrid_xml(xml_path: str) -> Dict:
    if not os.path.exists(xml_path):
        raise FileNotFoundError(f"找不到 XML: {xml_path}")
    root = ET.parse(xml_path).getroot()
    bg = [
        _find_first_vector(root, ("b1", "reciprocal_lattice_vector_1")),
        _find_first_vector(root, ("b2", "reciprocal_lattice_vector_2")),
        _find_first_vector(root, ("b3", "reciprocal_lattice_vector_3")),
    ]
    rotations = _find_rotation_matrices(root)
    time_reversal = _find_boolean(root, ("time_reversal", "use_time_reversal"), True)
    return {
        "bg": bg,
        "rotations": rotations,
        "time_reversal": time_reversal,
    }


def _matmul3(matrix: Sequence[Sequence[int]], vector: Sequence[float]) -> List[float]:
    return [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]


def _nint(value: float) -> int:
    return int(round(value))


def _normalize_to_first_bz(vector: Sequence[float]) -> List[float]:
    return [component - _nint(component) for component in vector]


def _vector_close_to_integer_triplet(values: Sequence[float]) -> bool:
    return all(abs(value - _nint(value)) <= EPS for value in values)


def crystal_to_cart(bg: Sequence[Sequence[float]], crystal: Sequence[float]) -> List[float]:
    return [
        bg[0][0] * crystal[0] + bg[1][0] * crystal[1] + bg[2][0] * crystal[2],
        bg[0][1] * crystal[0] + bg[1][1] * crystal[1] + bg[2][1] * crystal[2],
        bg[0][2] * crystal[0] + bg[1][2] * crystal[1] + bg[2][2] * crystal[2],
    ]


def generate_irreducible_qpoints(
    xml_path: str,
    nq1: int,
    nq2: int,
    nq3: int,
    shifts: Sequence[int] = (0, 0, 0),
) -> List[Dict]:
    metadata = parse_qgrid_xml(xml_path)
    bg = metadata["bg"]
    rotations = metadata["rotations"]
    time_reversal = metadata["time_reversal"]
    k1, k2, k3 = shifts
    nqr = nq1 * nq2 * nq3
    xkg: Dict[int, List[float]] = {}
    equiv = {index: index for index in range(1, nqr + 1)}
    weights = {index: 0.0 for index in range(1, nqr + 1)}

    for i in range(1, nq1 + 1):
        for j in range(1, nq2 + 1):
            for k in range(1, nq3 + 1):
                n = (k - 1) + (j - 1) * nq3 + (i - 1) * nq2 * nq3 + 1
                xkg[n] = [
                    float(i - 1) / float(nq1) + float(k1) / (2.0 * float(nq1)),
                    float(j - 1) / float(nq2) + float(k2) / (2.0 * float(nq2)),
                    float(k - 1) / float(nq3) + float(k3) / (2.0 * float(nq3)),
                ]

    for nk in range(1, nqr + 1):
        if equiv[nk] != nk:
            continue
        weights[nk] = 1.0
        for rotation in rotations:
            xkr = _normalize_to_first_bz(_matmul3(rotation, xkg[nk]))
            xx = xkr[0] * nq1 - 0.5 * k1
            yy = xkr[1] * nq2 - 0.5 * k2
            zz = xkr[2] * nq3 - 0.5 * k3
            if _vector_close_to_integer_triplet((xx, yy, zz)):
                i = math.floor(((_nint(xx + 2 * nq1)) % nq1) + 1)
                j = math.floor(((_nint(yy + 2 * nq2)) % nq2) + 1)
                k = math.floor(((_nint(zz + 2 * nq3)) % nq3) + 1)
                n = (k - 1) + (j - 1) * nq3 + (i - 1) * nq2 * nq3 + 1
                if n > nk and equiv[n] == n:
                    equiv[n] = nk
                    weights[nk] += 1.0
            if time_reversal:
                xx = -xkr[0] * nq1 - 0.5 * k1
                yy = -xkr[1] * nq2 - 0.5 * k2
                zz = -xkr[2] * nq3 - 0.5 * k3
                if _vector_close_to_integer_triplet((xx, yy, zz)):
                    i = math.floor(((_nint(xx + 2 * nq1)) % nq1) + 1)
                    j = math.floor(((_nint(yy + 2 * nq2)) % nq2) + 1)
                    k = math.floor(((_nint(zz + 2 * nq3)) % nq3) + 1)
                    n = (k - 1) + (j - 1) * nq3 + (i - 1) * nq2 * nq3 + 1
                    if n > nk and equiv[n] == n:
                        equiv[n] = nk
                        weights[nk] += 1.0

    irreducible = []
    total_weight = 0.0
    for nk in range(1, nqr + 1):
        if equiv[nk] != nk:
            continue
        crystal = _normalize_to_first_bz(xkg[nk])
        irreducible.append(
            {
                "index": len(irreducible) + 1,
                "q_crystal": crystal,
                "q_cart": crystal_to_cart(bg, crystal),
                "raw_weight": weights[nk],
            }
        )
        total_weight += weights[nk]

    for item in irreducible:
        item["weight"] = item.pop("raw_weight") / total_weight if total_weight else 0.0

    if all(shift == 0 for shift in shifts):
        gamma_index = None
        for idx, item in enumerate(irreducible):
            if all(abs(component) < 1.0e-12 for component in item["q_cart"]):
                gamma_index = idx
                break
        if gamma_index is not None and gamma_index != 0:
            gamma_point = irreducible.pop(gamma_index)
            irreducible.insert(0, gamma_point)

    for idx, item in enumerate(irreducible, start=1):
        item["index"] = idx
    return irreducible


def write_dyn0(path: str, nq1: int, nq2: int, nq3: int, qpoints: Sequence[Dict]) -> None:
    with open(path, "w", encoding="utf-8") as f:
        f.write(f"{nq1:4d}{nq2:4d}{nq3:4d}\n")
        f.write(f"{len(qpoints):4d}\n")
        for item in qpoints:
            q_crystal = item["q_crystal"]
            f.write(f"{q_crystal[0]:24.15E}{q_crystal[1]:24.15E}{q_crystal[2]:24.15E}\n")


def write_q_index_map(path: str, qpoints: Sequence[Dict]) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump({"qpoints": list(qpoints)}, f, indent=2, ensure_ascii=False)
