#!/usr/bin/env python3
import os
import argparse
import re
import numpy as np

from qeflow.qe_io_reader.bands import (
    parse_elebanddata,
    parse_projwfc_file,
    read_cell_from_pw_input,
    read_kpath_points_from_pw_input,
    read_kpoints_weights_from_pw_input,
)


def guess_prefix_from_filename(path):
    name = os.path.basename(path)
    for suffix in ["_band.projwfc_up", "_band.projwfc_down", "_band.projwfc", "_band.dat"]:
        if name.endswith(suffix):
            name = name[: -len(suffix)]
            break
    if "." in name:
        return name.split(".")[0]
    return name


read_cell_from_eband = read_cell_from_pw_input
read_kpath_points_from_eband = read_kpath_points_from_pw_input
read_kpoints_weights_from_eband = read_kpoints_weights_from_pw_input


def generate_klist(points, weights):
    if len(points) != len(weights):
        raise ValueError("K_POINTS 点与权重数量不一致。")
    klist = []
    jump_starts = set()
    for i in range(len(points) - 1):
        start = points[i]
        end = points[i + 1]
        num = int(weights[i])
        if num < 1:
            raise ValueError("K_POINTS 权重必须为正整数。")
        if i > 0 and weights[i - 1] == 1:
            jump_starts.add(len(klist))
        for j in range(num):
            frac = float(j) / float(num)
            klist.append(start + (end - start) * frac)
    klist.append(points[-1])
    return np.array(klist, dtype=float), jump_starts


def get_recip_lattice(cell):
    vol = np.dot(cell[0], np.cross(cell[1], cell[2]))
    b1 = 2 * np.pi * np.cross(cell[1], cell[2]) / vol
    b2 = 2 * np.pi * np.cross(cell[2], cell[0]) / vol
    b3 = 2 * np.pi * np.cross(cell[0], cell[1]) / vol
    return np.array([b1, b2, b3], dtype=float)


def compute_kpath_dist(coords, cell, weights=None):
    recip = get_recip_lattice(cell)
    metric = recip @ recip.T
    dist = [0.0]
    jump_points = set()
    if weights:
        indices = [1]
        for i in range(1, len(weights)):
            indices.append(indices[-1] + int(weights[i - 1]))
        jump_indices = set(indices[i] for i in range(1, len(indices)) if weights[i - 1] == 1)
        jump_points = set(jump_indices)

    if not weights:
        steps = []
        for i in range(1, len(coords)):
            delta = np.array(coords[i]) - np.array(coords[i - 1])
            steps.append(float(np.sqrt(delta @ metric @ delta)))
        nonzero_steps = [s for s in steps if s > 1.0e-10]
        median_step = np.median(nonzero_steps) if nonzero_steps else 0.0
        jump_threshold = median_step * 5.0 if median_step > 0 else float("inf")

        for i in range(1, len(coords)):
            delta = np.array(coords[i]) - np.array(coords[i - 1])
            step = float(np.sqrt(delta @ metric @ delta))
            if step > jump_threshold:
                dist.append(dist[-1])
                continue
            dist.append(dist[-1] + step)
    else:
        for i in range(1, len(coords)):
            if i + 1 in jump_points:
                dist.append(dist[-1])
                continue
            delta = np.array(coords[i]) - np.array(coords[i - 1])
            dist.append(dist[-1] + float(np.sqrt(delta @ metric @ delta)))
    return dist


def normalize_band_prefix(prefix):
    if prefix.endswith("_band_proj"):
        return prefix[: -len("_band_proj")]
    if prefix.endswith("_proj"):
        return prefix[: -len("_proj")]
    return prefix




def build_channels(labels):
    group_map = {}
    for label in labels:
        parsed = parse_label(label)
        if not parsed:
            continue
        elem, atom_index, shell_num, base_letter, _suffix = parsed
        if base_letter not in ("p", "d", "f"):
            continue
        group = f"{elem}{atom_index}-{shell_num}{base_letter}"
        group_map.setdefault(group, []).append(label)

    channels = order_channels(labels, group_map)
    return channels, group_map


def build_sum_channels(labels):
    sum_map = {}
    ordered = []
    for label in labels:
        parsed = parse_label(label)
        if not parsed:
            continue
        elem, _atom_index, shell_num, base_letter, suffix = parsed
        suffix = suffix or ""
        sum_label = f"{elem}-{shell_num}{base_letter}{suffix}"
        if sum_label not in sum_map:
            sum_map[sum_label] = []
            ordered.append(sum_label)
        sum_map[sum_label].append(label)

    group_map = {}
    for label in ordered:
        parsed = parse_label(label)
        if not parsed:
            continue
        elem, _atom_index, shell_num, base_letter, _suffix = parsed
        if base_letter not in ("p", "d", "f"):
            continue
        group = f"{elem}-{shell_num}{base_letter}"
        group_map.setdefault(group, []).append(label)

    channels = order_channels(ordered, group_map)
    return channels, sum_map, group_map


def parse_label(label):
    match = re.match(r"^([A-Z][a-z]?)(\d+)?-(\d+)([spdf])(.+)?$", label)
    if not match:
        return None
    elem, atom_index, shell_num, base_letter, suffix = match.groups()
    atom_index = int(atom_index) if atom_index else 0
    return elem, atom_index, int(shell_num), base_letter, (suffix or "")


def order_channels(labels, group_map):
    suffix_orders = {
        "s": [""],
        "p": ["x", "y", "z"],
        "d": ["z2", "xz", "yz", "x2-y2", "xy"],
        "f": ["z3", "xz2", "yz2", "xyz", "x2y2", "y3x2-y2", "zx2-y2"],
    }
    base_order = ["s", "p", "d", "f"]

    grouped = {}
    for label in labels:
        parsed = parse_label(label)
        if not parsed:
            grouped.setdefault(label, []).append(label)
            continue
        elem, atom_index, shell_num, base_letter, suffix = parsed
        grouped.setdefault((elem, atom_index, shell_num, base_letter), []).append((suffix, label))

    ordered = []
    for key in sorted(
        [k for k in grouped if isinstance(k, tuple)],
        key=lambda x: (x[0], x[1], x[2], base_order.index(x[3])),
    ):
        elem, atom_index, shell_num, base_letter = key
        suffix_labels = {suffix: lbl for suffix, lbl in grouped[key]}
        for suffix in suffix_orders.get(base_letter, []):
            label = suffix_labels.get(suffix)
            if label:
                ordered.append(label)
        if atom_index == 0:
            group_label = f"{elem}-{shell_num}{base_letter}"
        else:
            group_label = f"{elem}{atom_index}-{shell_num}{base_letter}"
        if group_label in group_map:
            ordered.append(f"{group_label}__sum")

    for key in grouped:
        if not isinstance(key, tuple):
            ordered.extend(grouped[key])
    return ordered


def resolve_weight(channel, group_map, proj_index, proj_weights, k_idx, band_idx):
    if channel.endswith("__sum"):
        base = channel[:-5]
        weight = 0.0
        for label in group_map.get(base, []):
            weight += proj_weights[proj_index[label], k_idx, band_idx]
        return weight
    return proj_weights[proj_index[channel], k_idx, band_idx]


def main():
    parser = argparse.ArgumentParser(description="Process QE projwfc band projections.")
    parser.add_argument("-i", "--input", required=True, help="projwfc 投影文件（如 *.projwfc_up）")
    parser.add_argument("-b", "--bandfile", default="elebanddata.dat", help="bands.x 输出文件")
    parser.add_argument("--cell", default="eband.in", help="包含晶胞信息的 eband.in；若未写 CELL_PARAMETERS，则读取同目录 POSCAR")
    parser.add_argument("-o", "--output", help="输出文件名，默认 <prefix>_eleband_proj.dat")
    parser.add_argument("--prefix", help="输出文件前缀（覆盖自动推断）")
    args = parser.parse_args()

    input_path = args.input
    if not os.path.exists(input_path):
        raise FileNotFoundError(f"找不到输入文件: {input_path}")

    proj_result = parse_projwfc_file(input_path)
    if proj_result is None:
        raise ValueError("未识别 projwfc 投影文件格式。")

    proj_labels, proj_weights, nk, nbnd = proj_result
    coords, energies, nbnd_band, nks_band = parse_elebanddata(args.bandfile)

    if nk != nks_band or nbnd != nbnd_band:
        raise ValueError(f"投影文件与能带文件尺寸不一致: projwfc(nk={nk}, nbnd={nbnd}) vs bands(nk={nks_band}, nbnd={nbnd_band})")

    cell = read_cell_from_eband(args.cell)
    kpath_points, kpath_labels = read_kpath_points_from_eband(args.cell)
    kpoints, weights = read_kpoints_weights_from_eband(args.cell)
    klist, jump_starts = generate_klist(kpoints, weights)
    if len(klist) != nks_band:
        raise ValueError(f"K_POINTS 生成的 k 点数({len(klist)})与 bands 文件不一致(nks={nks_band})")
    kpath_dist = compute_kpath_dist(klist, cell, weights)
    channels, group_map = build_channels(proj_labels)
    proj_index = {label: idx for idx, label in enumerate(proj_labels)}

    prefix = args.prefix or guess_prefix_from_filename(input_path)
    output_path = args.output or f"{prefix}_eleband_proj.dat"
    sum_output_path = f"{prefix}_eleband_proj_sum.dat"
    band_prefix = normalize_band_prefix(prefix)
    band_output_path = f"{band_prefix}_band.dat"

    col_width = max(12, max(len(name) for name in channels) if channels else 12)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(f"# source = {input_path}\n")
        f.write(f"# bands = {args.bandfile}\n")
        f.write(f"# cell = {args.cell}\n")
        header_parts = [f"{'kpath_dist':>12}", f"{'Energy(eV)':>14}"]
        header_parts.extend(f"{name.replace('__sum',''):>{col_width}}" for name in channels)
        f.write(" ".join(header_parts) + "\n")
        for band_idx in range(nbnd):
            for k_idx in range(nk):
                energy = energies[k_idx, band_idx]
                weights = []
                for channel in channels:
                    weight = resolve_weight(channel, group_map, proj_index, proj_weights, k_idx, band_idx)
                    weights.append(f"{weight:{col_width}.6f}")
                row = f"{kpath_dist[k_idx]:12.6f} {energy:14.6f} " + " ".join(weights)
                f.write(row + "\n")
            f.write(f"#band={band_idx + 1}\n\n")

    sum_channels, sum_map, sum_group_map = build_sum_channels(proj_labels)
    sum_col_width = max(12, max(len(name) for name in sum_channels) if sum_channels else 12)

    with open(sum_output_path, "w", encoding="utf-8") as f:
        f.write(f"# source = {input_path}\n")
        f.write(f"# bands = {args.bandfile}\n")
        f.write(f"# cell = {args.cell}\n")
        f.write("# summed by element and shell (e.g., Y-4s)\n")
        header_parts = [f"{'kpath_dist':>12}", f"{'Energy(eV)':>14}"]
        header_parts.extend(f"{name.replace('__sum',''):>{sum_col_width}}" for name in sum_channels)
        f.write(" ".join(header_parts) + "\n")
        for band_idx in range(nbnd):
            for k_idx in range(nk):
                energy = energies[k_idx, band_idx]
                weights = []
                for channel in sum_channels:
                    if channel.endswith("__sum"):
                        base = channel[:-5]
                        weight = 0.0
                        for label in sum_group_map.get(base, []):
                            for base_label in sum_map[label]:
                                weight += proj_weights[proj_index[base_label], k_idx, band_idx]
                    else:
                        weight = 0.0
                        if channel in sum_map:
                            for base_label in sum_map[channel]:
                                weight += proj_weights[proj_index[base_label], k_idx, band_idx]
                    weights.append(f"{weight:{sum_col_width}.6f}")
                row = f"{kpath_dist[k_idx]:12.6f} {energy:14.6f} " + " ".join(weights)
                f.write(row + "\n")
            f.write(f"#band={band_idx + 1}\n\n")

    with open(band_output_path, "w", encoding="utf-8") as f:
        f.write(f"# source = {args.bandfile}\n")
        f.write(f"# cell = {args.cell}\n")
        f.write("# kpath_dist energy\n")
        for band_idx in range(nbnd):
            for k_idx in range(nk):
                energy = energies[k_idx, band_idx]
                f.write(f"{kpath_dist[k_idx]:12.6f} {energy:14.6f}\n")
            f.write(f"#band={band_idx + 1}\n\n")

    print(f"已生成投影能带数据: {output_path}")
    print(f"已生成元素汇总投影能带数据: {sum_output_path}")
    print(f"已生成能带数据: {band_output_path}")


if __name__ == "__main__":
    main()
