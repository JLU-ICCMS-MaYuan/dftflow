#!/usr/bin/env python3
import json
import os
import shutil
from typing import Dict, List


def load_q_index_map(path: str) -> Dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _copy_if_exists(src: str, dst: str, missing: List[str]) -> None:
    if not os.path.exists(src):
        missing.append(src)
        return
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    if os.path.isdir(src):
        shutil.copytree(src, dst, dirs_exist_ok=True)
    else:
        shutil.copy2(src, dst)


def merge_split_results(root_dir: str, prefix: str, q_map_path: str) -> None:
    payload = load_q_index_map(q_map_path)
    qpoints = payload.get("qpoints", [])
    if not qpoints:
        raise ValueError("q_index_map.json 中没有 q 点信息。")

    missing: List[str] = []
    root_ph0 = os.path.join(root_dir, "_ph0")
    os.makedirs(root_ph0, exist_ok=True)
    first_dir = os.path.join(root_dir, f"q_{1:02d}")
    first_phsave = os.path.join(first_dir, "_ph0", f"{prefix}.phsave")
    _copy_if_exists(first_phsave, os.path.join(root_ph0, f"{prefix}.phsave"), missing)

    for item in qpoints:
        index = int(item["index"])
        q_dir = os.path.join(root_dir, f"q_{index:02d}")
        dyn_xml = os.path.join(q_dir, f"{prefix}.dyn{index}.xml")
        dyn_plain = os.path.join(q_dir, f"{prefix}.dyn{index}")
        if os.path.exists(dyn_xml):
            _copy_if_exists(dyn_xml, os.path.join(root_dir, f"{prefix}.dyn{index}.xml"), missing)
        else:
            _copy_if_exists(dyn_plain, os.path.join(root_dir, f"{prefix}.dyn{index}"), missing)

        q_ph_dir = os.path.join(q_dir, "_ph0", f"{prefix}.q_{index}")
        for dv_candidate in ("dvscf", "dvscf1", f"{prefix}.dvscf", f"{prefix}.dvscf1"):
            src = os.path.join(q_ph_dir, dv_candidate)
            if os.path.exists(src):
                _copy_if_exists(src, os.path.join(root_ph0, f"{prefix}.q_{index}", os.path.basename(src)), missing)
                break

    if missing:
        raise FileNotFoundError("merge 阶段缺少必要文件:\n" + "\n".join(missing))
