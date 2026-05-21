#!/usr/bin/env python3
import argparse
import csv
import os
import re
import shlex
import shutil
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np

from public.job import add_run_submit_arguments
from qeflow.phonon import _shared
from qeflow.phonon import console
from qeflow.phonon._shared import (
    PhononStepContext,
    build_labeled_commands,
    create_context,
    print_stage,
    refresh_workflow_status,
)
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_q2r import prepare_q2r_step
from qeflow.qe_input.builders import derive_filenames, write_root_run_script


PHBAND_DIR = "phband"
PHDOS_DIR = "phdos"
PROCESS_SCRIPT = "submit_process_phonon.sh"


def phband_archive_files(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [
        os.path.join(PHBAND_DIR, names["flfrq"]),
        os.path.join(PHBAND_DIR, f"{prefix}_band.labelinfo.dat"),
    ]


def phdos_archive_files(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [os.path.join(PHDOS_DIR, names["fldos"])]


def phonon_plot_files() -> List[str]:
    return [
        os.path.join(PHBAND_DIR, "phonon_band.csv"),
        os.path.join(PHBAND_DIR, "phonon_band_labels.csv"),
        os.path.join(PHBAND_DIR, "phonon_band.png"),
        os.path.join(PHDOS_DIR, "phonon_dos_atoms.csv"),
        os.path.join(PHDOS_DIR, "phonon_dos_elements.csv"),
        os.path.join(PHDOS_DIR, "phonon_dos.png"),
    ]


def collect_phband_outputs(context: PhononStepContext) -> None:
    names = derive_filenames(context.prefix)
    _copy_required(
        context.workspace,
        PHBAND_DIR,
        [
            names["flfrq"],
            f"{context.prefix}_band.labelinfo.dat",
        ],
    )


def collect_phdos_outputs(context: PhononStepContext) -> None:
    names = derive_filenames(context.prefix)
    _copy_required(context.workspace, PHDOS_DIR, [names["fldos"]])


def _copy_required(workspace: str, target_dir_name: str, filenames: Sequence[str]) -> None:
    target_dir = os.path.join(workspace, target_dir_name)
    os.makedirs(target_dir, exist_ok=True)
    missing = [filename for filename in filenames if not os.path.exists(os.path.join(workspace, filename))]
    if missing:
        raise FileNotFoundError("缺少待归档文件: " + ", ".join(missing))
    for filename in filenames:
        shutil.copy2(os.path.join(workspace, filename), os.path.join(target_dir, filename))


def build_collect_phband_commands(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [
        f"mkdir -p {shlex.quote(PHBAND_DIR)}",
        f"cp -f {shlex.quote(names['flfrq'])} {shlex.quote(PHBAND_DIR)}/",
        f"cp -f {shlex.quote(prefix + '_band.labelinfo.dat')} {shlex.quote(PHBAND_DIR)}/",
    ]


def build_collect_phdos_commands(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [
        f"mkdir -p {shlex.quote(PHDOS_DIR)}",
        f"cp -f {shlex.quote(names['fldos'])} {shlex.quote(PHDOS_DIR)}/",
    ]


def prepare_process_phonon_step(context: PhononStepContext) -> str:
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)
    config_arg = shlex.quote(context.config_file)
    commands = build_labeled_commands(
        [
            ("运行 q2r", "bash run_q2r.sh"),
            ("运行 matdyn 声子谱", "bash run_matdyn_phband.sh"),
            ("归档声子谱结果", f"qe_process_phonon -c {config_arg} --collect phband"),
            ("运行 matdyn 声子 DOS", "bash run_matdyn_phdos.sh"),
            ("归档声子 DOS 结果", f"qe_process_phonon -c {config_arg} --collect phdos"),
            ("绘图并写出 CSV", f"qe_process_phonon -c {config_arg} --plot"),
        ]
    )
    return write_root_run_script(
        context.workspace,
        PROCESS_SCRIPT,
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
        announce=False,
    )


def prepare_matdyn_phband_collect_step(context: PhononStepContext) -> str:
    prepare_matdyn_phband_step(context)
    return write_root_run_script(
        context.workspace,
        "submit_matdyn_phband.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        ["bash run_matdyn_phband.sh", *build_collect_phband_commands(context.prefix)],
        announce=False,
    )


def prepare_matdyn_phdos_collect_step(context: PhononStepContext) -> str:
    prepare_matdyn_phdos_step(context)
    return write_root_run_script(
        context.workspace,
        "submit_matdyn_phdos.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        ["bash run_matdyn_phdos.sh", *build_collect_phdos_commands(context.prefix)],
        announce=False,
    )


def run_process_phonon_steps(context: PhononStepContext, include_plot: bool) -> int:
    prepare_q2r_step(context)
    prepare_matdyn_phband_step(context)
    prepare_matdyn_phdos_step(context)
    stage_no = 1
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["q2r"] != "finished":
        stage_no = _run_process_action(context, stage_no, "运行 q2r", lambda: _shared.run_shell("bash run_q2r.sh", context.workspace))
    if workflow["matdyn_phband"] != "finished":
        if _root_phband_outputs_exist(context):
            stage_no = _run_process_action(context, stage_no, "归档 phband", lambda: collect_phband_outputs(context))
        else:
            stage_no = _run_process_action(context, stage_no, "运行 matdyn_phband", lambda: _shared.run_shell("bash run_matdyn_phband.sh", context.workspace))
            stage_no = _run_process_action(context, stage_no, "归档 phband", lambda: collect_phband_outputs(context))
    refresh_workflow_status(context.workspace, context.prefix)
    if workflow["matdyn_phdos"] != "finished":
        if _root_phdos_outputs_exist(context):
            stage_no = _run_process_action(context, stage_no, "归档 phdos", lambda: collect_phdos_outputs(context))
        else:
            stage_no = _run_process_action(context, stage_no, "运行 matdyn_phdos", lambda: _shared.run_shell("bash run_matdyn_phdos.sh", context.workspace))
            stage_no = _run_process_action(context, stage_no, "归档 phdos", lambda: collect_phdos_outputs(context))
    if include_plot:
        print_stage(stage_no, "绘制声子谱和声子 DOS")
        try:
            plot_phonon_outputs(context)
        finally:
            refresh_workflow_status(context.workspace, context.prefix)
    return 0


def _run_process_action(context: PhononStepContext, stage_no: int, label: str, action) -> int:
    print_stage(stage_no, label)
    try:
        action()
    except Exception:
        refresh_workflow_status(context.workspace, context.prefix)
        raise
    refresh_workflow_status(context.workspace, context.prefix)
    return stage_no + 1


def _root_phband_outputs_exist(context: PhononStepContext) -> bool:
    names = derive_filenames(context.prefix)
    return os.path.exists(os.path.join(context.workspace, names["flfrq"])) and os.path.exists(
        os.path.join(context.workspace, f"{context.prefix}_band.labelinfo.dat")
    )


def _root_phdos_outputs_exist(context: PhononStepContext) -> bool:
    return os.path.exists(os.path.join(context.workspace, derive_filenames(context.prefix)["fldos"]))


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


def build_phonon_band_tables(context: PhononStepContext) -> Tuple[List[Dict[str, object]], List[Dict[str, object]]]:
    label_path = os.path.join(context.workspace, PHBAND_DIR, f"{context.prefix}_band.labelinfo.dat")
    freq_path = os.path.join(context.workspace, PHBAND_DIR, derive_filenames(context.prefix)["flfrq"])
    labels = _read_labelinfo(label_path)
    if len(labels) < 1:
        raise ValueError("高对称点 labelinfo 为空。")
    frequencies = _read_band_frequencies(freq_path, int(np.sum(context.struct_info["counts"])) * 3)
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
    numeric_rows = _read_numeric_rows(dos_path)
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


def _read_labelinfo(path: str) -> List[Dict[str, object]]:
    labels: List[Dict[str, object]] = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            parts = line.split()
            if len(parts) < 6:
                continue
            labels.append(
                {
                    "label": parts[0].replace("GAMMA", "Γ"),
                    "point_index": int(parts[1]),
                    "distance": float(parts[2]),
                    "kx": float(parts[3]),
                    "ky": float(parts[4]),
                    "kz": float(parts[5]),
                }
            )
    return labels


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


def _read_band_frequencies(path: str, expected_modes: int) -> List[List[float]]:
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        lines = handle.readlines()

    qe_plot_rows = _read_qe_plot_frequencies(lines, expected_modes, path)
    if qe_plot_rows is not None:
        return qe_plot_rows

    freq_pattern = re.compile(
        r"freq\s*\(\s*\d+\s*\)\s*=\s*[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?\s*(?:\[?[A-Za-z]+\]?)?\s*=\s*([-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?)",
        flags=re.IGNORECASE,
    )
    grouped: List[List[float]] = []
    current: List[float] = []
    numeric_rows: List[List[float]] = []
    for line in lines:
        match = freq_pattern.search(line)
        if match is not None:
            current.append(float(match.group(1)))
            if len(current) == expected_modes:
                grouped.append(current)
                current = []
            continue
        values = _float_values(line)
        if len(values) >= expected_modes:
            numeric_rows.append(values[-expected_modes:])
    if grouped:
        return grouped
    if current:
        raise ValueError("声子谱 freq 行数量不是模式数的整数倍。")
    if numeric_rows:
        return numeric_rows
    raise ValueError(f"无法从声子谱文件解析频率: {path}")


def _read_qe_plot_frequencies(lines: Sequence[str], expected_modes: int, path: str) -> Optional[List[List[float]]]:
    header_pattern = re.compile(r"&plot\s+nbnd\s*=\s*(\d+)\s*,\s*nks\s*=\s*(\d+)", flags=re.IGNORECASE)
    header_index = -1
    nbnd = 0
    nks = 0
    for index, line in enumerate(lines):
        match = header_pattern.search(line)
        if match is None:
            continue
        header_index = index
        nbnd = int(match.group(1))
        nks = int(match.group(2))
        break
    if header_index < 0:
        return None
    if nbnd != expected_modes:
        raise ValueError(f"声子谱模式数与结构不一致: 文件 nbnd={nbnd}，结构期望 {expected_modes}")

    rows: List[List[float]] = []
    line_index = header_index + 1
    for point_index in range(1, nks + 1):
        line_index = _next_nonempty_line(lines, line_index)
        if line_index >= len(lines):
            raise ValueError(f"声子谱文件缺少第 {point_index} 个 q 点坐标: {path}")
        coord_values = _float_values(lines[line_index])
        if len(coord_values) < 3:
            raise ValueError(f"声子谱文件第 {point_index} 个 q 点坐标无法解析: {lines[line_index].strip()}")
        line_index += 1

        modes: List[float] = []
        while len(modes) < nbnd:
            line_index = _next_nonempty_line(lines, line_index)
            if line_index >= len(lines):
                raise ValueError(f"声子谱文件第 {point_index} 个 q 点频率数量不足: 期望 {nbnd}，实际 {len(modes)}")
            modes.extend(_float_values(lines[line_index]))
            line_index += 1
        if len(modes) != nbnd:
            raise ValueError(f"声子谱文件第 {point_index} 个 q 点频率数量错误: 期望 {nbnd}，实际 {len(modes)}")
        rows.append(modes)

    if len(rows) != nks:
        raise ValueError(f"声子谱 q 点数量与 header 不一致: {len(rows)} != {nks}")
    return rows


def _next_nonempty_line(lines: Sequence[str], start_index: int) -> int:
    index = start_index
    while index < len(lines) and not lines[index].strip():
        index += 1
    return index


def _float_values(line: str) -> List[float]:
    return [float(item) for item in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?", line)]


def _read_numeric_rows(path: str) -> List[List[float]]:
    rows: List[List[float]] = []
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            values = _float_values(stripped)
            if values:
                rows.append(values)
    return rows


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


def run_process_phonon(
    config_file: str,
    struct_file: Optional[str],
    run_calc: bool,
    submit_job: bool,
    collect_only: object,
    plot_only: bool,
) -> int:
    context = create_context(config_file, struct_file)
    if run_calc:
        run_process_phonon_steps(context, include_plot=True)
    elif submit_job:
        prepare_process_phonon_step(context)
        console.generated(PROCESS_SCRIPT)
        console.submit_script(context.workspace, PROCESS_SCRIPT)
    elif collect_only:
        target = "all" if collect_only is True else str(collect_only)
        if target in {"all", "phband"}:
            collect_phband_outputs(context)
        if target in {"all", "phdos"}:
            collect_phdos_outputs(context)
    elif plot_only:
        plot_phonon_outputs(context)
    else:
        prepare_process_phonon_step(context)
    refresh_workflow_status(context.workspace, context.prefix)
    if run_calc:
        console.completion("q2r + matdyn + collect + plot")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO 声子 q2r/matdyn 后处理编排与绘图")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    parser.add_argument(
        "--collect",
        nargs="?",
        const="all",
        choices=("all", "phband", "phdos"),
        help="只归档已有结果，可选 phband/phdos；省略目标时同时归档二者",
    )
    parser.add_argument("--plot", action="store_true", help="只读取 phband/phdos 归档结果并绘图")
    add_run_submit_arguments(parser, "本地运行 q2r/phband/phdos、归档并绘图", "提交 q2r/phband/phdos 与归档脚本")
    args = parser.parse_args()
    if args.collect and args.plot:
        parser.error("--collect 不能与 --plot 同时使用。")
    if (args.collect or args.plot) and (args.run or args.submit):
        parser.error("--collect/--plot 不能与 -r/-s 同时使用。")
    raise SystemExit(run_process_phonon(args.config, args.input, args.run, args.submit, args.collect, args.plot))


if __name__ == "__main__":
    main()
