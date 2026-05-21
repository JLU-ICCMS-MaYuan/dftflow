#!/usr/bin/env python3
import os
from typing import Dict, Iterable, List, Optional, Tuple

from qeflow.qe_check.outputs import MatdynOut, PhOut, Q2rOut, RelaxOut, ScfOut
from qeflow.qe_input.fileio import list_split_directories, load_json


VALID_STATES = ("unstarted", "finished", "error")


def _finalize_state(
    output_exists: bool,
    finished: bool,
    errored: bool,
) -> str:
    if not output_exists:
        return "unstarted"
    if finished:
        return "finished"
    if errored:
        return "error"
    return "error"


def check_scf(workdir: str, filename: str = "scf.out") -> str:
    target = ScfOut(os.path.join(workdir, filename))
    return _finalize_state(target.exists(), target.is_finished(), target.has_error())


def check_relax(workdir: str) -> str:
    for candidate in ("relax.out", "vc-relax.out"):
        path = os.path.join(workdir, candidate)
        if os.path.exists(path):
            target = RelaxOut(path)
            return _finalize_state(True, target.is_finished(), target.has_error())
    return "unstarted"


def check_q2r(workdir: str, prefix: str) -> str:
    output = Q2rOut(os.path.join(workdir, "q2r.out"))
    fc_path = os.path.join(workdir, f"{prefix}.fc")
    if not output.exists():
        return "unstarted"
    if output.is_finished() and os.path.exists(fc_path):
        return "finished"
    if output.has_error() or not os.path.exists(fc_path):
        return "error"
    return "error"


def _load_q_index_payload(workdir: str) -> Dict[str, object]:
    payload = load_json(os.path.join(workdir, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return {}
    return payload


def _load_q_index_entries(workdir: str) -> List[Dict[str, object]]:
    payload = _load_q_index_payload(workdir)
    entries = payload.get("qpoints", [])
    return entries if isinstance(entries, list) else []


def _load_q_index_mode(workdir: str) -> str:
    payload = _load_q_index_payload(workdir)
    mode = payload.get("mode")
    return str(mode) if isinstance(mode, str) else ""


def collect_split_task_states(workdir: str) -> Dict[str, str]:
    entries = _load_q_index_entries(workdir)
    q_names: List[str] = []
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        q_name = str(entry.get("directory", "")).strip()
        if q_name:
            q_names.append(q_name)
    if not q_names:
        q_names = sorted(os.path.basename(q_dir) for q_dir in list_split_directories(workdir))
    return {
        q_name: check_split_task(os.path.join(workdir, q_name))
        for q_name in q_names
    }


def _resolve_merged_ph_root(workdir: str) -> str:
    tmp_ph0 = os.path.join(workdir, "tmp", "_ph0")
    if os.path.isdir(tmp_ph0) or os.path.isdir(os.path.join(workdir, "tmp")):
        return tmp_ph0
    return os.path.join(workdir, "_ph0")


def _collect_merge_required_paths(
    workdir: str,
    prefix: str,
    mode: str,
    entries: List[Dict[str, object]],
) -> Tuple[List[str], List[List[str]], bool]:
    merged_ph_root = _resolve_merged_ph_root(workdir)
    required_paths: List[str] = [os.path.join(merged_ph_root, f"{prefix}.phsave")]
    alternative_paths: List[List[str]] = []
    requires_patterns = mode == "split_assign_index"
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        index = entry.get("index")
        if index is None:
            continue
        q_index = int(index)
        required_paths.extend(
            [
                os.path.join(workdir, f"{prefix}.dyn{q_index}"),
                os.path.join(workdir, f"split_ph.in.{q_index}"),
                os.path.join(workdir, f"split_ph.out.{q_index}"),
            ]
        )
        if mode == "split_assign_index":
            if q_index == 1:
                alternative_paths.append(
                    [
                        os.path.join(merged_ph_root, f"{prefix}.dvscf1"),
                        os.path.join(merged_ph_root, "dvscf"),
                        os.path.join(merged_ph_root, f"{prefix}.q_1"),
                    ]
                )
            else:
                required_paths.append(os.path.join(merged_ph_root, f"{prefix}.q_{q_index}"))
        elif mode == "split_assign_coords":
            # coords 模式允许 dvscf 直接位于 _ph0/，不要求根目录必须有 q 子目录。
            pass
    return required_paths, alternative_paths, requires_patterns


def _merge_patterns_ready(workdir: str, prefix: str, entries: List[Dict[str, object]], mode: str) -> bool:
    phsave_dir = os.path.join(_resolve_merged_ph_root(workdir), f"{prefix}.phsave")
    if not os.path.isdir(phsave_dir):
        return False
    if mode == "split_assign_coords":
        return os.path.exists(os.path.join(phsave_dir, "patterns.1.xml"))
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        index = entry.get("index")
        if index is None:
            continue
        if not os.path.exists(os.path.join(phsave_dir, f"patterns.{int(index)}.xml")):
            return False
    return True


def check_merge(workdir: str, prefix: str) -> str:
    entries = _load_q_index_entries(workdir)
    if not entries:
        return "unstarted"
    mode = _load_q_index_mode(workdir)
    if mode not in {"split_assign_coords", "split_assign_index"}:
        return "error"

    split_states = []
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        q_name = str(entry.get("directory", "")).strip()
        if not q_name:
            continue
        split_states.append(check_split_task(os.path.join(workdir, q_name)))

    if summarize_task_states(split_states) != "finished":
        return "unstarted"

    required_paths, alternative_paths, requires_patterns = _collect_merge_required_paths(workdir, prefix, mode, entries)

    existing_count = sum(os.path.exists(path) for path in required_paths)
    alternative_count = sum(any(os.path.exists(path) for path in group) for group in alternative_paths)
    patterns_ready = _merge_patterns_ready(workdir, prefix, entries, mode) if requires_patterns or mode == "split_assign_coords" else True
    if existing_count == 0 and alternative_count == 0 and not patterns_ready:
        return "unstarted"
    if existing_count != len(required_paths) or alternative_count != len(alternative_paths) or not patterns_ready:
        return "error"
    return "finished"


def check_matdyn(
    workdir: str,
    output_name: str,
    product_name: str,
    archived_products: Optional[List[str]] = None,
) -> str:
    output = MatdynOut(os.path.join(workdir, output_name))
    archived = archived_products or []
    if not output.exists():
        return "unstarted"
    product_exists = os.path.exists(os.path.join(workdir, product_name))
    archive_ready = all(os.path.exists(os.path.join(workdir, path)) for path in archived)
    if output.is_finished() and product_exists and archive_ready:
        return "finished"
    if output.has_error() or not product_exists or not archive_ready:
        return "error"
    return "error"


def check_phonon_plot(workdir: str, prefix: str, matdyn_phband_state: str, matdyn_phdos_state: str) -> str:
    required = [
        os.path.join(workdir, "phband", "phonon_band.csv"),
        os.path.join(workdir, "phband", "phonon_band_labels.csv"),
        os.path.join(workdir, "phband", "phonon_band.png"),
        os.path.join(workdir, "phdos", "phonon_dos_atoms.csv"),
        os.path.join(workdir, "phdos", "phonon_dos_elements.csv"),
        os.path.join(workdir, "phdos", "phonon_dos.png"),
    ]
    existing_count = sum(os.path.exists(path) for path in required)
    if matdyn_phband_state != "finished" or matdyn_phdos_state != "finished":
        return "unstarted"
    if existing_count == 0:
        return "unstarted"
    if existing_count == len(required):
        return "finished"
    return "error"


def check_split_task(q_dir: str) -> str:
    scf = ScfOut(os.path.join(q_dir, "scf.out"))
    ph = PhOut(os.path.join(q_dir, "split_ph.out"))
    has_any_output = scf.exists() or ph.exists()
    if ph.is_finished() and scf.is_finished():
        return "finished"
    if ph.has_error() or scf.has_error():
        return "error"
    if not has_any_output:
        return "unstarted"
    return "error"


def check_ph(workdir: str, prefix: str) -> Dict[str, object]:
    dyn0_path = os.path.join(workdir, f"{prefix}.dyn0")
    dyn0_ready = os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0
    root_ph = PhOut(os.path.join(workdir, "ph.out"))
    task_states = collect_split_task_states(workdir)
    if not task_states:
        if dyn0_ready and root_ph.is_finished():
            state = "finished"
        elif dyn0_ready:
            state = "finished"
        elif root_ph.has_error():
            state = "error"
        elif root_ph.exists():
            state = "error"
        else:
            state = "unstarted"
        return {
            "state": state,
            "dyn0": "finished" if dyn0_ready else "unstarted",
            "tasks": {},
        }

    unique_states = set(task_states.values())
    if task_states and unique_states == {"finished"}:
        state = "finished"
    elif "error" in unique_states:
        state = "error"
    elif task_states:
        state = "unstarted" if unique_states == {"unstarted"} else "error"
    elif dyn0_ready:
        state = "finished"
    else:
        state = "unstarted"
    return {
        "state": state,
        "dyn0": "finished" if dyn0_ready else "unstarted",
        "tasks": task_states,
    }


def summarize_task_states(task_states: Iterable[str]) -> str:
    states = set(task_states)
    if not states:
        return "unstarted"
    if states == {"finished"}:
        return "finished"
    if "error" in states:
        return "error"
    return "unstarted"
