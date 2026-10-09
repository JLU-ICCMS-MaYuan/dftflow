#!/usr/bin/env python3
import os

from public.job import write_job_script
from qeflow.phonon import console
from qeflow.phonon.context import PhononStepContext
from qeflow.io_utils import dump_json
from qeflow.phonon.q_index_map import load_q_points_entries
from qeflow.input_deck.ph import write_split_ph_input_assign_coords, write_split_ph_input_assign_index
from qeflow.input_deck.pw import write_phonon_scf_input
from qeflow.io_reader.ph import parse_dyn0_qpoints


def _load_split_q_entries(context: PhononStepContext) -> list:
    """加载拆分所需的 q 点列表。

    优先使用 Rust 工具 `qf-gen-dyn0` 产出的 `q_points.json`（其中 `q_crystal` 是
    真正的晶体坐标，与 ph.x 输入的 `q` 同坐标系）。若缺失则回退到旧流程：从 `dyn0`
    解析（注意 ph.x 生成的 dyn0 坐标为笛卡尔坐标，旧 coords 模式拆分会受影响）。
    """
    entries = load_q_points_entries(context.workspace)
    if entries:
        return entries
    dyn0_path = os.path.join(context.workspace, f"{context.prefix}.dyn0")
    raw = parse_dyn0_qpoints(dyn0_path)
    entries = []
    for q in raw:
        entries.append(
            {
                "directory": f"{int(q['index']):02d}",
                "index": int(q["index"]),
                "q_crystal": [float(v) for v in q["q_crystal"]],
            }
        )
    return entries


def _append_split_step_commands(commands, context: PhononStepContext, q_dir: str, q_name: str, step: str, command: str) -> None:
    commands.extend(
        [
            console.echo_qeflow("-" * 60),
            console.echo_qeflow(f"workflow: {os.path.basename(context.workspace)}"),
            console.echo_qeflow(f"prefix: {context.prefix}"),
            console.echo_qeflow(f"q_dir: {q_name}"),
            console.echo_qeflow(f"step: {step}"),
            console.echo_qeflow(f"work_dir: {q_dir}"),
            console.echo_qeflow(f"command: {command}"),
            f'if ! cd "{q_dir}"; then',
            console.echo_qeflow(f"failed: {step}"),
            "  exit 1",
            "fi",
            f"if ! {command}; then",
            console.echo_qeflow(f"failed: {step}"),
            "  exit 1",
            "fi",
            console.echo_qeflow(f"done: {step}"),
        ]
    )


def _write_split_run_script(context: PhononStepContext, q_dir: str, q_name: str, slurm_header: str) -> str:
    pw_command = f"{context.commands['pw']} < scf.in > scf.out"
    ph_command = f"{context.commands['ph']} < split_ph.in > split_ph.out"
    commands = ["set -e"]
    _append_split_step_commands(commands, context, q_dir, q_name, "split_scf", pw_command)
    _append_split_step_commands(commands, context, q_dir, q_name, "split_ph", ph_command)
    path = os.path.join(q_dir, "run_split.sh")
    write_job_script(path, slurm_header, commands, announce=False)
    return path


def _ensure_q_index_map_written(context: PhononStepContext, entries: list, mode: str) -> None:
    """仅当 `q_index_map.json` 尚不存在时才写出（避免覆盖 Rust 产出的规范文件）。"""
    q_map_path = os.path.join(context.workspace, "q_index_map.json")
    if os.path.exists(q_map_path):
        return
    payload = {"mode": mode, "qpoints": entries}
    dump_json(q_map_path, payload)


def prepare_split_assign_index_step(context: PhononStepContext) -> int:
    qpoints = _load_split_q_entries(context)
    slurm_header = str(context.config.get("slurm", {}).get("header", ""))
    q_map_payload = {"mode": "split_assign_index", "qpoints": []}
    for qpoint in qpoints:
        q_name = f"{int(qpoint['index']):02d}"
        q_dir = os.path.join(context.workspace, q_name)
        os.makedirs(q_dir, exist_ok=True)
        write_phonon_scf_input(q_dir, context.config, context.struct_info, context.prefix)
        write_split_ph_input_assign_index(q_dir, context.config, context.prefix, int(qpoint["index"]))
        _write_split_run_script(context, q_dir, q_name, slurm_header)
        q_map_payload["qpoints"].append(
            {
                "directory": q_name,
                "index": int(qpoint["index"]),
                "q_crystal": qpoint["q_crystal"],
            }
        )
    _ensure_q_index_map_written(context, q_map_payload["qpoints"], "split_assign_index")
    return len(q_map_payload["qpoints"])


def prepare_split_assign_coords_step(context: PhononStepContext) -> int:
    qpoints = _load_split_q_entries(context)
    slurm_header = str(context.config.get("slurm", {}).get("header", ""))
    q_map_payload = {"mode": "split_assign_coords", "qpoints": []}
    for qpoint in qpoints:
        q_name = f"{int(qpoint['index']):02d}"
        q_dir = os.path.join(context.workspace, q_name)
        os.makedirs(q_dir, exist_ok=True)
        write_phonon_scf_input(q_dir, context.config, context.struct_info, context.prefix)
        write_split_ph_input_assign_coords(q_dir, context.config, context.prefix, qpoint["q_crystal"])
        _write_split_run_script(context, q_dir, q_name, slurm_header)
        q_map_payload["qpoints"].append(
            {
                "directory": q_name,
                "index": int(qpoint["index"]),
                "q_crystal": qpoint["q_crystal"],
            }
        )
    _ensure_q_index_map_written(context, q_map_payload["qpoints"], "split_assign_coords")
    return len(q_map_payload["qpoints"])
