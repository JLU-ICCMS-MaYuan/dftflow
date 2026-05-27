#!/usr/bin/env python3
import os

from public.job import write_job_script
from qeflow.phonon import console
from qeflow.phonon.context import PhononStepContext
from qeflow.io_utils import dump_json
from qeflow.input_deck.ph import write_split_ph_input_assign_coords, write_split_ph_input_assign_index
from qeflow.input_deck.pw import write_phonon_scf_input
from qeflow.io_reader.ph import parse_dyn0_qpoints


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


def prepare_split_assign_index_step(context: PhononStepContext) -> int:
    dyn0_path = os.path.join(context.workspace, f"{context.prefix}.dyn0")
    qpoints = parse_dyn0_qpoints(dyn0_path)
    slurm_header = str(context.config.get("slurm", {}).get("header", ""))
    q_map_payload = {"mode": "split_assign_index", "qpoints": []}
    for dir_index, qpoint in enumerate(qpoints):
        q_name = f"{dir_index + 1:02d}"
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
    dump_json(os.path.join(context.workspace, "q_index_map.json"), q_map_payload)
    return len(q_map_payload["qpoints"])


def prepare_split_assign_coords_step(context: PhononStepContext) -> int:
    dyn0_path = os.path.join(context.workspace, f"{context.prefix}.dyn0")
    qpoints = parse_dyn0_qpoints(dyn0_path)
    slurm_header = str(context.config.get("slurm", {}).get("header", ""))
    q_map_payload = {"mode": "split_assign_coords", "qpoints": []}
    for dir_index, qpoint in enumerate(qpoints):
        q_name = f"{dir_index + 1:02d}"
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
    dump_json(os.path.join(context.workspace, "q_index_map.json"), q_map_payload)
    return len(q_map_payload["qpoints"])
