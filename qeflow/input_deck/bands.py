#!/usr/bin/env python3
import os
from typing import Any, Sequence

import numpy as np

from qeflow.input_deck.kpath import compute_reciprocal_lattice, parse_kpath_segments
from qeflow.input_deck.pw import write_pw_input


def write_eband_inputs(setup: Any) -> None:
    print("正在生成 eband.in ...")
    write_pw_input(setup, "eband.in", write_eband_kpoints, precision=8)
    write_banddata_inputs(setup)


def write_eband_kpoints(setup: Any, handle: Any) -> None:
    k_points_config = setup.cli_config.get("k_points", {})
    kpath_points = int(k_points_config.get("kpath_points", 100))
    kpath_file = str(k_points_config.get("kpath_file", "KPATH.in"))
    kpath_path = kpath_file if os.path.isabs(kpath_file) else os.path.join(setup.work_dir, kpath_file)
    if not os.path.exists(kpath_path):
        raise FileNotFoundError(f"找不到 KPATH.in: {kpath_path}")
    segments = parse_kpath_segments(kpath_path)
    if not segments:
        raise ValueError("KPATH.in 未解析到任何高对称点。")
    for index, segment in enumerate(segments, start=1):
        labels = "-".join(label for _coords, label in segment)
        print(f"[kpath] segment {index}: {labels}")
    write_gun_kpath_labels(setup.work_dir, setup.prefix, segments, setup.struct_info["lattice"])
    kpoint_lines = []
    for seg_idx, segment in enumerate(segments):
        for point_idx, (coords, label) in enumerate(segment):
            is_last_point = point_idx == len(segment) - 1
            is_jump = is_last_point and seg_idx < len(segments) - 1
            is_final_point = is_last_point and seg_idx == len(segments) - 1
            weight = 1 if is_jump or is_final_point else kpath_points
            kpoint_lines.append((coords, label, weight))
    write_indexed_kpath_labels(
        setup.work_dir,
        setup.prefix,
        segments,
        setup.struct_info["lattice"],
        [weight for _coords, _label, weight in kpoint_lines],
    )
    handle.write("K_POINTS crystal_b\n")
    handle.write(f"{len(kpoint_lines)}\n")
    for coords, label, weight in kpoint_lines:
        handle.write(f"  {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f} {weight:6d} ! {label}\n")


def write_gun_kpath_labels(work_dir: str, prefix: str, segments: Any, lattice: Sequence[Sequence[float]]) -> str:
    lattice_bohr = np.asarray(lattice, dtype=float) / 0.52917721092
    alat_bohr = float(np.linalg.norm(lattice_bohr[0]))
    reciprocal = np.linalg.inv(lattice_bohr).T * alat_bohr
    label_points = []
    for seg_idx, segment in enumerate(segments):
        if seg_idx == 0:
            label_points.extend(segment)
            continue
        previous_end_coords, previous_end_label = segments[seg_idx - 1][-1]
        label_points.extend([(previous_end_coords, previous_end_label), *segment[1:]])
    total_dist = 0.0
    last_cart = None
    label_path = os.path.join(work_dir, f"{prefix}_gun_band.labelinfo.dat")
    with open(label_path, "w", encoding="utf-8") as handle:
        handle.write("# label dist kx ky kz\n")
        for coords, label in label_points:
            cart = np.dot(coords, reciprocal)
            if last_cart is not None:
                total_dist += float(np.linalg.norm(cart - last_cart))
            handle.write(f"{label:8} {total_dist:12.6f} {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f}\n")
            last_cart = cart
    print(f"已生成高对称点坐标: {label_path}")
    return label_path


def write_indexed_kpath_labels(
    work_dir: str,
    prefix: str,
    segments: Any,
    lattice: Sequence[Sequence[float]],
    weights: Sequence[int],
) -> str:
    reciprocal = compute_reciprocal_lattice(lattice)
    label_points = []
    jump_starts = []
    for seg_idx, segment in enumerate(segments):
        for point_idx, (coords, label) in enumerate(segment):
            label_points.append((coords, label))
            jump_starts.append(seg_idx > 0 and point_idx == 0)
    if len(label_points) != len(weights):
        raise ValueError("高对称点数量与 K_POINTS 权重数量不一致。")
    indices = [1]
    for index in range(1, len(weights)):
        indices.append(indices[-1] + int(weights[index - 1]))
    total_dist = 0.0
    last_cart = None
    label_path = os.path.join(work_dir, f"{prefix}_band.labelinfo.dat")
    with open(label_path, "w", encoding="utf-8") as handle:
        for idx, (coords, label), is_jump in zip(indices, label_points, jump_starts):
            cart = np.dot(coords, reciprocal)
            if last_cart is not None and not is_jump:
                total_dist += float(np.linalg.norm(cart - last_cart))
            handle.write(f"{label:8} {idx:12d} {total_dist:16.10f} {coords[0]:16.10f} {coords[1]:16.10f} {coords[2]:16.10f}\n")
            last_cart = cart
    print(f"已生成高对称点坐标: {label_path}")
    return label_path


def write_banddata_inputs(setup: Any) -> None:
    outdir_val = setup.control_outdir
    eband_cfg = setup.cli_config.get("eband", {})
    lsym = eband_cfg.get("lsym", False)
    band_prefix = f"{setup.prefix}_band"
    with open(os.path.join(setup.work_dir, "elebanddata.in"), "w", encoding="utf-8") as handle:
        handle.write("&BANDS\n")
        handle.write(f" prefix = '{setup.prefix}',\n")
        handle.write(f" outdir = '{outdir_val}',\n")
        handle.write(f" filband = '{band_prefix}',\n")
        handle.write(" lp = .true.\n")
        handle.write("/\n")
    if isinstance(lsym, bool):
        lsym_str = "true" if lsym else "false"
    else:
        lsym_str = "true" if str(lsym).strip(".").lower() in ("true", "t", "1") else "false"
    with open(os.path.join(setup.work_dir, "elebandprojdata.in"), "w", encoding="utf-8") as handle:
        handle.write("&projwfc\n")
        handle.write(f" prefix = '{setup.prefix}',\n")
        handle.write(f" outdir = '{outdir_val}',\n")
        handle.write(f" lsym = .{lsym_str}.,\n")
        handle.write(f" filproj = '{band_prefix}'\n")
        handle.write("/\n")
