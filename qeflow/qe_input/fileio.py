#!/usr/bin/env python3
import json
import os
import shutil
from typing import Any, Dict, List, Optional, Tuple

from public.config_templates import load_config_file
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_phonon_allowed_keys,
    get_qe_phonon_parameters_template,
)


def load_phonon_config(config_file: str) -> Dict[str, Any]:
    template = get_qe_phonon_parameters_template()
    return apply_qe_config_overrides(
        template,
        load_config_file(config_file),
        get_qe_phonon_allowed_keys(),
    )


def read_text(path: str) -> str:
    with open(path, "r", encoding="utf-8") as handle:
        return handle.read()


def write_text(path: str, content: str) -> None:
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(content)


def load_json(path: str, default: Optional[Any] = None) -> Any:
    if not os.path.exists(path):
        return default
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def dump_json(path: str, payload: Any) -> None:
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, ensure_ascii=False)


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
) -> Tuple[Dict[str, Any], str, str]:
    current_dir = os.getcwd()
    config = load_phonon_config(config_file)
    work_dir_name = str(config["qe_phonon"]["work_dir"])
    if os.path.basename(os.path.abspath(current_dir)) == work_dir_name:
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
        config_target = os.path.join(workspace, "input.toml")
        if os.path.abspath(config_file) != os.path.abspath(config_target):
            shutil.copy2(config_file, config_target)

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
