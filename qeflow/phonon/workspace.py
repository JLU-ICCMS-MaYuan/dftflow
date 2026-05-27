#!/usr/bin/env python3
import os
import shutil
from typing import Any, Dict, List, Optional, Tuple

from public.config_templates import load_config_file
from qeflow.common import normalize_qe_config_paths
from qeflow.qe_input_deck.lattice import validate_no_manual_lattice_params
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_phonon_allowed_keys,
    get_qe_phonon_parameters_template,
)


WORKFLOW_DEFAULT_WORKDIRS = {
    "qe_phonon_nosplit": "qe_phonon_nosplit",
    "qe_phonon_split_coords": "qe_phonon_split_coords",
    "qe_phonon_split_index": "qe_phonon_split_index",
}


def load_phonon_config(config_file: str) -> Dict[str, Any]:
    template = get_qe_phonon_parameters_template()
    raw_config = load_config_file(config_file)
    validate_no_manual_lattice_params(raw_config)
    config = apply_qe_config_overrides(
        template,
        raw_config,
        get_qe_phonon_allowed_keys(),
    )
    return normalize_qe_config_paths(config, config_file=config_file)


def resolve_config_path(config_file: str) -> str:
    return os.path.abspath(config_file)


def ensure_structure_source(struct_file: Optional[str]) -> str:
    if struct_file:
        if not os.path.exists(struct_file):
            raise FileNotFoundError(f"找不到结构文件: {struct_file}")
        return struct_file
    for candidate in ("POSCAR", "CONTCAR"):
        if os.path.exists(candidate):
            return candidate
    raise FileNotFoundError("找不到结构文件，请使用 -i 指定，或在当前目录准备 POSCAR/CONTCAR。")


def resolve_workspace(
    config_file: str,
    struct_file: Optional[str] = None,
    prepare_inputs: bool = False,
    workflow_name: Optional[str] = None,
) -> Tuple[Dict[str, Any], str, str]:
    current_dir = os.getcwd()
    resolved_config_file = resolve_config_path(config_file)
    raw_config = load_config_file(resolved_config_file)
    validate_no_manual_lattice_params(raw_config)
    config = apply_qe_config_overrides(
        get_qe_phonon_parameters_template(),
        raw_config,
        get_qe_phonon_allowed_keys(),
    )
    config = normalize_qe_config_paths(config, config_file=resolved_config_file)
    qe_phonon_raw = raw_config.get("qe_phonon", {})
    has_explicit_work_dir = isinstance(qe_phonon_raw, dict) and "work_dir" in qe_phonon_raw
    if has_explicit_work_dir:
        work_dir_name = str(config["qe_phonon"]["work_dir"])
    elif workflow_name in WORKFLOW_DEFAULT_WORKDIRS:
        work_dir_name = WORKFLOW_DEFAULT_WORKDIRS[str(workflow_name)]
    else:
        work_dir_name = "qe_phonon"
    current_markers = (
        "q_index_map.json",
        "workflow_status.json",
        "run_scf.sh",
        "run_ph.sh",
        "run_q2r.sh",
        "run_matdyn_phband.sh",
        "run_matdyn_phdos.sh",
    )
    if os.path.basename(os.path.abspath(current_dir)) == work_dir_name:
        workspace = current_dir
    elif any(os.path.exists(os.path.join(current_dir, marker)) for marker in current_markers):
        workspace = current_dir
    else:
        workspace = os.path.join(current_dir, work_dir_name)
        if prepare_inputs:
            os.makedirs(workspace, exist_ok=True)

    if prepare_inputs:
        structure_target = os.path.join(workspace, "POSCAR")
        if struct_file is None and os.path.exists(structure_target):
            structure_source = structure_target
        else:
            structure_source = ensure_structure_source(struct_file)
        if os.path.abspath(structure_source) != os.path.abspath(structure_target):
            shutil.copy2(structure_source, structure_target)

    return config, current_dir, workspace


def list_split_directories(workspace: str) -> List[str]:
    entries: List[str] = []
    if not os.path.isdir(workspace):
        return entries
    for name in sorted(os.listdir(workspace)):
        if not (name.isdigit() and len(name) == 2):
            continue
        path = os.path.join(workspace, name)
        if os.path.isdir(path):
            entries.append(path)
    return entries
