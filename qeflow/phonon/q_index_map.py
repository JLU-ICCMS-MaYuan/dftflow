#!/usr/bin/env python3
import os
from typing import Dict, List

from qeflow.qe_input.fileio import load_json


def load_q_index_entries(workspace: str) -> List[Dict[str, object]]:
    payload = load_json(os.path.join(workspace, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return []
    qpoints = payload.get("qpoints", [])
    return qpoints if isinstance(qpoints, list) else []


def load_q_index_mode(workspace: str) -> str:
    payload = load_json(os.path.join(workspace, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return ""
    mode = payload.get("mode")
    return str(mode) if isinstance(mode, str) else ""


def iter_q_directory_names(workspace: str) -> List[str]:
    names: List[str] = []
    for entry in load_q_index_entries(workspace):
        if not isinstance(entry, dict):
            continue
        directory = str(entry.get("directory", "")).strip()
        if directory:
            names.append(directory)
    return names
