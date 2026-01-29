#!/usr/bin/env python3
import argparse
import glob
import os

import matplotlib.pyplot as plt
import numpy as np

def get_recip_base(cell):
    """从实空间基矢计算倒空间基矢 (2*pi*b)"""
    v1, v2, v3 = cell
    vol = np.dot(v1, np.cross(v2, v3))
    b1 = 2 * np.pi * np.cross(v2, v3) / vol
    b2 = 2 * np.pi * np.cross(v3, v1) / vol
    b3 = 2 * np.pi * np.cross(v1, v2) / vol
    return np.array([b1, b2, b3])

def pick_file(directory, patterns, desc):
    matches = []
    for pattern in patterns:
        matches.extend(glob.glob(os.path.join(directory, pattern)))
    matches = sorted(set(matches))
    if not matches:
        raise FileNotFoundError(f"找不到 {desc}，目录: {directory}")
    if len(matches) > 1:
        print(f"Warning: 找到多个{desc}，默认使用 {matches[0]}")
    return matches[0]


def resolve_inputs(qe_dir, w90_dir, label_file=None):
    qe_dir = os.path.abspath(qe_dir)
    w90_dir = os.path.abspath(w90_dir)

    qe_band_file = pick_file(
        qe_dir,
        ["*_band", "*_band.dat", "elebanddata.dat"],
        "QE 能带数据",
    )
    w90_band_file = pick_file(
        w90_dir,
        ["*_band.dat"],
        "Wannier90 能带数据",
    )

    if label_file:
        label_path = os.path.abspath(label_file)
        if not os.path.exists(label_path):
            raise FileNotFoundError(f"找不到高对称点信息文件: {label_path}")
        return qe_band_file, w90_band_file, label_path

    label_patterns = ["*_band.labelinfo.dat", "*__band.labelinfo.dat", "qe_k_lable.dat"]
    label_path = None
    for target_dir in (w90_dir, qe_dir):
        try:
            label_path = pick_file(target_dir, label_patterns, "高对称点信息")
            break
        except FileNotFoundError:
            continue

    return qe_band_file, w90_band_file, label_path


def plot_comparison(qe_band_file, w90_band_file, w90_label_file, fermi_energy, output_img):
    # --- 1. 参数设置 ---
    # 晶胞参数 (来自 eband.in 或 Y1H6.win)
    cell = np.array([
        [-1.6848999334,  1.6848999334,  1.6848999334],
        [ 1.6848999334, -1.6848999334,  1.6848999334],
        [ 1.6848999334,  1.6848999334, -1.6848999334]
    ])
    
    # 费米能级 (来自 Y1H6.win)
    fermi_w90 = fermi_energy
    fermi_qe = fermi_energy 

    if not os.path.exists(qe_band_file):
        print(f"Error: 找不到 QE 数据文件 {qe_band_file}")
        return
    if not os.path.exists(w90_band_file):
        print(f"Error: 找不到 Wannier90 数据文件 {w90_band_file}")
        return

    # --- 2. 处理 QE 数据 (计算物理距离) ---
    b_basis = get_recip_base(cell)
    qe_k_dist = []
    qe_bands = []

    with open(qe_band_file, 'r') as f:
        lines = f.readlines()
    
    header = lines[0].split()
    nbnd = int(header[2].replace(',', ''))
    nks = int(header[5])

    cur_dist = 0.0
    prev_k_cart = None
    idx = 1
    for _ in range(nks):
        while idx < len(lines) and not lines[idx].strip():
            idx += 1
        if idx >= len(lines): break
        line_parts = lines[idx].split()
        k_frac = np.array([float(x) for x in line_parts[:3]])
        k_cart = k_frac @ b_basis
        if prev_k_cart is not None:
            cur_dist += np.linalg.norm(k_cart - prev_k_cart)
        qe_k_dist.append(cur_dist)
        prev_k_cart = k_cart
        
        idx += 1
        ebands = []
        while len(ebands) < nbnd and idx < len(lines):
            ebands.extend([float(x) for x in lines[idx].split()])
            idx += 1
        qe_bands.append(ebands)

    qe_bands = np.array(qe_bands).T - fermi_qe

    # --- 3. 读取 Wannier90 数据 ---
    w90_data = np.loadtxt(w90_band_file)
    w90_k = w90_data[:, 0]
    w90_e = w90_data[:, 1] - fermi_w90

    # --- 4. 绘图 ---
    plt.figure(figsize=(10, 7))

    # 绘制 QE 能带 (红色点)
    for i in range(nbnd):
        label = 'DFT (QE)' if i == 0 else ""
        plt.scatter(qe_k_dist, qe_bands[i], s=7, c='red', alpha=0.6, edgecolors='none', label=label)

    # 绘制 Wannier90 能带 (蓝色线)
    nks_w90 = len(qe_k_dist) # 假设 k 点数一致
    for i in range(0, len(w90_k), nks_w90):
        label = 'Wannier90' if i == 0 else ""
        plt.plot(w90_k[i:i+nks_w90], w90_e[i:i+nks_w90], 'b-', linewidth=1.5, alpha=0.8, label=label)

    # 绘制高对称点垂直线和标签
    if w90_label_file and os.path.exists(w90_label_file):
        with open(w90_label_file, 'r') as f:
            tick_coords = []
            tick_labels = []
            for line in f:
                parts = line.split()
                if not parts: continue
                label = parts[0].replace('GAMMA', r'$\Gamma$')
                dist = float(parts[2])
                plt.axvline(x=dist, color='black', linestyle='-', linewidth=0.5, alpha=0.3)
                tick_coords.append(dist)
                tick_labels.append(label)
            plt.xticks(tick_coords, tick_labels)

    plt.axhline(y=0, color='black', linestyle='--', linewidth=1, alpha=0.5)
    plt.ylabel('Energy - $E_f$ (eV)')
    plt.title('Band Structure Comparison: QE (Dots) vs Wannier90 (Lines)')
    
    # 自动去重 legend
    handles, labels = plt.gca().get_legend_handles_labels()
    by_label = dict(zip(labels, handles))
    plt.legend(by_label.values(), by_label.keys(), loc='upper right')

    plt.xlim(0, max(qe_k_dist))
    plt.ylim(-10, 10) # 能量显示范围
    plt.grid(True, axis='y', linestyle=':', alpha=0.4)
    
    plt.savefig(output_img, dpi=300, bbox_inches='tight')
    print(f"Comparison plot saved to {output_img}")

def main():
    parser = argparse.ArgumentParser(description='Compare QE and Wannier90 band structures.')
    parser.add_argument('--qe', default='.', help='QE 数据目录')
    parser.add_argument('--w90', default='.', help='Wannier90 数据目录')
    parser.add_argument('--label', default=None, help='高对称点信息文件（可选）')
    parser.add_argument('--fermi', type=float, default=23.3313, help='Fermi energy to align (eV)')
    parser.add_argument('--out', default='band_comparison.png', help='Output image filename')
    
    args = parser.parse_args()
    qe_band_file, w90_band_file, label_file = resolve_inputs(args.qe, args.w90, args.label)
    plot_comparison(qe_band_file, w90_band_file, label_file, args.fermi, args.out)

if __name__ == "__main__":
    main()
