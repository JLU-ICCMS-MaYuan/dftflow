#!/usr/bin/env python3
import os
import subprocess
from typing import Dict, Iterable, List, Optional, Set, Tuple

from qeflow.qe_check.outputs import MatdynOut, PhOut, Q2rOut, RelaxOut, ScfOut
from qeflow.qe_input.fileio import list_split_directories, load_json


VALID_STATES = ("unstarted", "running", "finished", "error")


def get_running_workdirs(user: Optional[str] = None) -> Set[str]:
    target_user = user or os.environ.get("USER", "")
    if not target_user:
        return set()
    cmd = f"squeue -u {target_user} -h --format='%12A %12P %5t %25R %8M %Z'"
    result = subprocess.run(
        cmd,
        shell=True,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return set()
    workdirs: Set[str] = set()
    for raw_line in result.stdout.splitlines():
        line = raw_line.rstrip()
        if not line:
            continue
        parts = line.split(maxsplit=5)
        if len(parts) < 6:
            continue
        state = parts[2].strip()
        workdir = parts[5].strip()
        if state in {"R", "PD"} and workdir:
            workdirs.add(os.path.abspath(workdir))
    return workdirs


def _finalize_state(
    output_exists: bool,
    finished: bool,
    errored: bool,
    workdir: str,
    running_dirs: Set[str],
) -> str:
    workdir_abs = os.path.abspath(workdir)
    if workdir_abs in running_dirs:
        return "running"
    if not output_exists:
        return "unstarted"
    if finished:
        return "finished"
    if errored:
        return "error"
    return "error"


def check_scf(workdir: str, filename: str = "scf.out", running_dirs: Optional[Set[str]] = None) -> str:
    running = running_dirs or get_running_workdirs()
    target = ScfOut(os.path.join(workdir, filename))
    return _finalize_state(target.exists(), target.is_finished(), target.has_error(), workdir, running)


def check_relax(workdir: str, running_dirs: Optional[Set[str]] = None) -> str:
    running = running_dirs or get_running_workdirs()
    for candidate in ("relax.out", "vc-relax.out"):
        path = os.path.join(workdir, candidate)
        if os.path.exists(path):
            target = RelaxOut(path)
            return _finalize_state(True, target.is_finished(), target.has_error(), workdir, running)
    return "running" if os.path.abspath(workdir) in running else "unstarted"


def check_q2r(workdir: str, prefix: str, running_dirs: Optional[Set[str]] = None) -> str:
    running = running_dirs or get_running_workdirs()
    output = Q2rOut(os.path.join(workdir, "q2r.out"))
    fc_path = os.path.join(workdir, f"{prefix}.fc")
    if os.path.abspath(workdir) in running:
        return "running"
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


def check_merge(workdir: str, prefix: str, running_dirs: Optional[Set[str]] = None) -> str:
    running = running_dirs or get_running_workdirs()
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
        split_states.append(check_split_task(os.path.join(workdir, q_name), running))

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
    running_dirs: Optional[Set[str]] = None,
) -> str:
    running = running_dirs or get_running_workdirs()
    output = MatdynOut(os.path.join(workdir, output_name))
    if os.path.abspath(workdir) in running:
        return "running"
    if not output.exists():
        return "unstarted"
    if output.is_finished() and os.path.exists(os.path.join(workdir, product_name)):
        return "finished"
    if output.has_error():
        return "error"
    return "error"


def check_split_task(q_dir: str, running_dirs: Optional[Set[str]] = None) -> str:
    running = running_dirs or get_running_workdirs()
    q_dir_abs = os.path.abspath(q_dir)
    scf = ScfOut(os.path.join(q_dir, "scf.out"))
    ph = PhOut(os.path.join(q_dir, "split_ph.out"))
    has_any_output = scf.exists() or ph.exists()
    if q_dir_abs in running:
        return "running"
    if ph.is_finished() and scf.is_finished():
        return "finished"
    if ph.has_error() or scf.has_error():
        return "error"
    if not has_any_output:
        return "unstarted"
    return "error"


def check_ph(workdir: str, prefix: str, running_dirs: Optional[Set[str]] = None) -> Dict[str, object]:
    running = running_dirs or get_running_workdirs()
    dyn0_path = os.path.join(workdir, f"{prefix}.dyn0")
    dyn0_ready = os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0
    root_ph = PhOut(os.path.join(workdir, "ph.out"))
    split_dirs = list_split_directories(workdir)
    if not split_dirs:
        if os.path.abspath(workdir) in running:
            state = "running"
        elif dyn0_ready and root_ph.is_finished():
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
            "dyn0": "finished" if dyn0_ready else ("running" if os.path.abspath(workdir) in running else "unstarted"),
            "tasks": {},
        }

    task_states = {
        os.path.basename(q_dir): check_split_task(q_dir, running)
        for q_dir in split_dirs
    }
    unique_states = set(task_states.values())
    if task_states and unique_states == {"finished"}:
        state = "finished"
    elif "error" in unique_states:
        state = "error"
    elif "running" in unique_states:
        state = "running"
    elif task_states:
        state = "unstarted" if unique_states == {"unstarted"} else "error"
    elif dyn0_ready:
        state = "finished"
    else:
        state = "unstarted"
    return {
        "state": state,
        "dyn0": "finished" if dyn0_ready else ("running" if os.path.abspath(workdir) in running else "unstarted"),
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
    if "running" in states:
        return "running"
    return "unstarted"
