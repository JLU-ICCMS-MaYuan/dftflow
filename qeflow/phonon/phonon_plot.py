#!/usr/bin/env python3
import csv
import os
from typing import Dict, List

from qeflow.phonon.archive import PHBAND_DIR, PHDOS_DIR
from qeflow.phonon.context import PhononStepContext
from qeflow.phonon.phonon_tables import build_phonon_band_tables, build_phonon_dos_tables


def plot_phonon_outputs(context: PhononStepContext) -> None:
    band_rows, label_rows = build_phonon_band_tables(context)
    dos_atoms_rows, dos_elements_rows, element_names = build_phonon_dos_tables(context)
    _write_table(os.path.join(context.workspace, PHBAND_DIR, "phonon_band.csv"), band_rows)
    _write_table(os.path.join(context.workspace, PHBAND_DIR, "phonon_band_labels.csv"), label_rows)
    _write_table(os.path.join(context.workspace, PHDOS_DIR, "phonon_dos_atoms.csv"), dos_atoms_rows)
    _write_table(os.path.join(context.workspace, PHDOS_DIR, "phonon_dos_elements.csv"), dos_elements_rows)
    _plot_band(os.path.join(context.workspace, PHBAND_DIR, "phonon_band.png"), band_rows, label_rows)
    _plot_dos(
        os.path.join(context.workspace, PHDOS_DIR, "phonon_dos.png"),
        dos_elements_rows,
        ["TDOS", *element_names],
    )


def _write_table(path: str, rows: List[Dict[str, object]]) -> None:
    if not rows:
        raise ValueError(f"没有可写出的表格数据: {path}")
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    fieldnames = list(rows[0].keys())
    with open(path, "w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def _plot_band(path: str, band_rows: List[Dict[str, object]], label_rows: List[Dict[str, object]]) -> None:
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制声子谱图。") from exc

    x = [float(row["distance"]) for row in band_rows]
    mode_keys = [key for key in band_rows[0] if key.startswith("mode_")]
    fig, ax = plt.subplots(figsize=(6, 4))
    for key in mode_keys:
        ax.plot(x, [float(row[key]) for row in band_rows], color="#1f77b4", linewidth=1.0)
    for label in label_rows:
        ax.axvline(float(label["distance"]), color="0.75", linewidth=0.8)
    ax.set_xticks([float(label["distance"]) for label in label_rows])
    ax.set_xticklabels([str(label["label"]) for label in label_rows])
    ax.set_ylabel("Frequency (cm$^{-1}$)")
    ax.set_xlim(min(x), max(x))
    ax.grid(axis="y", color="0.9", linewidth=0.6)
    fig.tight_layout()
    fig.savefig(path, dpi=300)
    plt.close(fig)


def _plot_dos(path: str, dos_rows: List[Dict[str, object]], columns: List[str]) -> None:
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise ImportError("缺少 matplotlib，无法绘制声子 DOS 图。") from exc

    freq = [float(row["frequency_cm-1"]) for row in dos_rows]
    fig, ax = plt.subplots(figsize=(4, 4))
    for column in columns:
        if column not in dos_rows[0]:
            continue
        ax.plot([float(row[column]) for row in dos_rows], freq, linewidth=1.2, label=column)
    ax.set_xlabel("DOS")
    ax.set_ylabel("Frequency (cm$^{-1}$)")
    ax.grid(color="0.9", linewidth=0.6)
    ax.legend(frameon=False, fontsize=8)
    fig.tight_layout()
    fig.savefig(path, dpi=300)
    plt.close(fig)
