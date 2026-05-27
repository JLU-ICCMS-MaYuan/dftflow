#!/usr/bin/env python3
import copy
import os
from typing import Any, Dict, Sequence, Tuple

from qeflow.input_deck.filenames import derive_filenames
from qeflow.input_deck.values import to_qe_value


def write_ph_input(
    workspace: str,
    config: Dict[str, Any],
    prefix: str,
    filename: str = "ph.in",
    dyn0_only: bool = False,
) -> str:
    ph_cfg = ph_grid_config(config, prefix)
    if dyn0_only:
        ph_cfg["start_q"] = 1
        ph_cfg["last_q"] = 1
    return write_inputph(workspace, "ph.in" if filename is None else filename, config, prefix, ph_cfg)


def write_split_ph_input_assign_index(
    q_dir: str,
    config: Dict[str, Any],
    prefix: str,
    ph_q_index: int,
) -> str:
    ph_cfg = ph_grid_config(config, prefix)
    ph_cfg["ldisp"] = True
    ph_cfg["start_q"] = ph_q_index
    ph_cfg["last_q"] = ph_q_index
    return write_inputph(q_dir, "split_ph.in", config, prefix, ph_cfg)


def write_split_ph_input_assign_coords(
    q_dir: str,
    config: Dict[str, Any],
    prefix: str,
    q_crystal: Sequence[float],
) -> str:
    ph_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    ph_cfg["ldisp"] = False
    ph_cfg["fildyn"] = derive_filenames(prefix)["fildyn"]
    ph_cfg.pop("qpoints", None)
    ph_cfg.pop("start_q", None)
    ph_cfg.pop("last_q", None)
    path = write_inputph(q_dir, "split_ph.in", config, prefix, ph_cfg)
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(f"{float(q_crystal[0]):.15E} {float(q_crystal[1]):.15E} {float(q_crystal[2]):.15E}\n")
    return path


def ph_grid_config(config: Dict[str, Any], prefix: str) -> Dict[str, Any]:
    ph_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    nq1, nq2, nq3 = resolve_grid(ph_cfg, "qpoints", ("nq1", "nq2", "nq3"))
    ph_cfg["nq1"] = nq1
    ph_cfg["nq2"] = nq2
    ph_cfg["nq3"] = nq3
    ph_cfg.pop("qpoints", None)
    ph_cfg["fildyn"] = derive_filenames(prefix)["fildyn"]
    return ph_cfg


def write_inputph(
    workspace: str,
    filename: str,
    config: Dict[str, Any],
    prefix: str,
    ph_cfg: Dict[str, Any],
) -> str:
    outdir = config["qe_phonon"]["scf"].get("outdir", "./out")
    path = os.path.join(workspace, filename)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&inputph\n")
        handle.write(f"  prefix = {to_qe_value(prefix)}\n")
        handle.write(f"  outdir = {to_qe_value(outdir)}\n")
        for key, value in ph_cfg.items():
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n")
    return path


def resolve_grid(
    section: Dict[str, Any],
    vector_key: str,
    fallback_keys: Tuple[str, str, str],
) -> Tuple[int, int, int]:
    vector = section.get(vector_key)
    if isinstance(vector, list) and len(vector) == 3:
        return int(vector[0]), int(vector[1]), int(vector[2])
    values = [section.get(key) for key in fallback_keys]
    if all(value is not None for value in values):
        return int(values[0]), int(values[1]), int(values[2])
    raise ValueError(f"缺少 {vector_key} 配置")

