#!/usr/bin/env python3
import os
import subprocess
from typing import List, Sequence, Tuple

import numpy as np


def run_vaspkit_for_kpath(work_dir: str) -> None:
    subprocess.run(
        ["vaspkit"],
        input="3\n303\n",
        text=True,
        check=True,
        cwd=work_dir,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def parse_kpath_segments(path: str, tol: float = 1.0e-6):
    points = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) != 4:
                continue
            try:
                kx, ky, kz = map(float, parts[:3])
            except ValueError:
                continue
            points.append(([kx, ky, kz], parts[3]))
    if len(points) % 2 != 0:
        raise ValueError(f"KPATH.in 格式异常，点数为奇数: {len(points)}")
    segments = []
    for index in range(0, len(points), 2):
        (k1, l1), (k2, l2) = points[index], points[index + 1]
        if not segments:
            segments.append([(k1, l1), (k2, l2)])
            continue
        previous_end = segments[-1][-1][0]
        if (
            abs(previous_end[0] - k1[0]) < tol
            and abs(previous_end[1] - k1[1]) < tol
            and abs(previous_end[2] - k1[2]) < tol
        ):
            segments[-1].append((k2, l2))
        else:
            segments.append([(k1, l1), (k2, l2)])
    return segments


def flatten_segments(segments):
    path_points = []
    for segment in segments:
        for index, (coords, label) in enumerate(segment):
            if index == 0 and path_points and path_points[-1][0] == coords and path_points[-1][1] == label:
                continue
            path_points.append((coords, label))
    return path_points


def compute_reciprocal_lattice(lattice: Sequence[Sequence[float]]) -> np.ndarray:
    lattice_array = np.array(lattice, dtype=float)
    volume = float(np.linalg.det(lattice_array))
    if abs(volume) < 1.0e-12:
        raise ValueError("晶胞体积过小，无法计算倒格子。")
    return 2.0 * np.pi * np.linalg.inv(lattice_array).T


def write_kpath_labels(
    work_dir: str,
    prefix: str,
    segments,
    lattice: Sequence[Sequence[float]],
    weights: Sequence[int],
) -> str:
    recip_lat = compute_reciprocal_lattice(lattice)
    label_points = []
    jump_starts = []
    for seg_idx, segment in enumerate(segments):
        for pt_idx, (coords, label) in enumerate(segment):
            label_points.append((coords, label))
            jump_starts.append(seg_idx > 0 and pt_idx == 0)
    if len(label_points) != len(weights):
        raise ValueError("高对称点数量与权重数量不一致。")
    indices = [1]
    for i in range(1, len(weights)):
        indices.append(indices[-1] + int(weights[i - 1]))
    total_dist = 0.0
    last_cart = None
    label_path = os.path.join(work_dir, f"{prefix}_band.labelinfo.dat")
    with open(label_path, "w", encoding="utf-8") as f:
        for idx, (coords, label), is_jump in zip(indices, label_points, jump_starts):
            cart = np.dot(np.array(coords, dtype=float), recip_lat)
            if last_cart is not None and not is_jump:
                total_dist += float(np.linalg.norm(cart - last_cart))
            f.write(
                f"{label:8} {idx:12d} {total_dist:16.10f} {coords[0]:16.10f} {coords[1]:16.10f} {coords[2]:16.10f}\n"
            )
            last_cart = cart
    return label_path


def build_matdyn_band_path(
    work_dir: str,
    prefix: str,
    lattice: Sequence[Sequence[float]],
    kpath_file: str,
    kpath_points: int,
) -> Tuple[List[Tuple[List[float], str, int]], str]:
    kpath_path = kpath_file if os.path.isabs(kpath_file) else os.path.join(work_dir, kpath_file)
    if not os.path.exists(kpath_path):
        raise FileNotFoundError(f"找不到 KPATH.in: {kpath_path}")
    segments = parse_kpath_segments(kpath_path)
    if not segments:
        raise ValueError("KPATH.in 未解析到任何高对称点。")
    flatten_segments(segments)
    kpoint_lines = []
    for seg_idx, segment in enumerate(segments):
        for pt_idx, (coords, label) in enumerate(segment):
            is_last_point = pt_idx == len(segment) - 1
            is_jump = is_last_point and seg_idx < len(segments) - 1
            is_final_point = is_last_point and seg_idx == len(segments) - 1
            weight = 1 if is_jump or is_final_point else int(kpath_points)
            kpoint_lines.append((coords, label, weight))
    label_path = write_kpath_labels(
        work_dir,
        prefix,
        segments,
        lattice,
        [weight for _coords, _label, weight in kpoint_lines],
    )
    return kpoint_lines, label_path
