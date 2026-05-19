#!/usr/bin/env python3
import os

from qeflow.phonon._shared import PhononStepContext, parse_dyn0_qpoints
from qeflow.qe_input.builders import (
    write_scf_input,
    write_split_ph_input_assign_coords,
    write_split_ph_input_assign_index,
    write_split_run_script,
)
from qeflow.qe_input.fileio import dump_json


def prepare_split_assign_index_step(context: PhononStepContext) -> int:
    dyn0_path = os.path.join(context.workspace, f"{context.prefix}.dyn0")
    qpoints = parse_dyn0_qpoints(dyn0_path)
    slurm_header = str(context.config.get("slurm", {}).get("header", ""))
    q_map_payload = {"mode": "split_assign_index", "qpoints": []}
    for dir_index, qpoint in enumerate(qpoints):
        q_name = f"{dir_index + 1:02d}"
        q_dir = os.path.join(context.workspace, q_name)
        os.makedirs(q_dir, exist_ok=True)
        write_scf_input(q_dir, context.config, context.struct_info, context.prefix)
        write_split_ph_input_assign_index(q_dir, context.config, context.prefix, int(qpoint["index"]))
        write_split_run_script(q_dir, slurm_header, context.commands["pw"], context.commands["ph"])
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
        write_scf_input(q_dir, context.config, context.struct_info, context.prefix)
        write_split_ph_input_assign_coords(q_dir, context.config, context.prefix, qpoint["q_crystal"])
        write_split_run_script(q_dir, slurm_header, context.commands["pw"], context.commands["ph"])
        q_map_payload["qpoints"].append(
            {
                "directory": q_name,
                "index": int(qpoint["index"]),
                "q_crystal": qpoint["q_crystal"],
            }
        )
    dump_json(os.path.join(context.workspace, "q_index_map.json"), q_map_payload)
    return len(q_map_payload["qpoints"])
