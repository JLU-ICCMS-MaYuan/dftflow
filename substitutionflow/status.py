#!/usr/bin/env python3
import glob
import json
import os
import re
import subprocess
from contextlib import suppress
from collections import OrderedDict
from typing import Dict, Iterable, List, Optional, Set, Tuple

from public.config_templates import load_config_file


STEP_ORDER = ("mattersim_opt", "vasp_opt", "vasp_scf", "vasp_eledos", "vasp_process_eledos", "qe_phonon")
STATUS_ORDER = ("finished", "running", "error", "unstarted")
QE_PHONON_STATE_ORDER = ("finished", "running", "error", "unstarted")
PHONON_MODE_TO_WORKFLOW = {
    "nosplit": "qe_phonon_nosplit",
    "split_index": "qe_phonon_split_index",
    "split_coords": "qe_phonon_split_coords",
}

OPT_CONVERGED_MARKER = "reached required accuracy - stopping structural energy minimisation"
SCF_REQUIRED_MARKERS = (
    "FREE ENERGIE OF THE ION-ELECTRON SYSTEM",
    "General timing and accounting informations for this job",
    "aborting loop because EDIFF is reached",
)
SCF_FORBIDDEN_MARKER = "in kB  *"
SQUEUE_FORMAT = "%12A %12P %5t %25R %8M %Z"
DEFAULT_SQUEUE_USER = "mayuan"


def get_workflow_status_path(output_dir: str) -> str:
    return os.path.join(output_dir, "workflow_status.json")


def get_case_status_path(case_dir: str) -> str:
    return os.path.join(case_dir, "case_status.json")


def case_uses_mattersim(case_dir: str) -> bool:
    try:
        config = _load_case_config(case_dir)
    except Exception:
        return False
    return "mattersim_opt" in config


def _load_case_config(case_dir: str) -> Dict:
    return load_config_file(os.path.join(case_dir, "input.toml"))


def get_case_step_order(case_dir: str) -> Tuple[str, ...]:
    if case_uses_mattersim(case_dir):
        return STEP_ORDER
    return tuple(step for step in STEP_ORDER if step != "mattersim_opt")


def get_case_phonon_mode(case_dir: str) -> str:
    config = _load_case_config(case_dir)
    substitution_config = config.get("substitutionflow", {})
    mode = substitution_config.get("phonon_mode", "split_index")
    if mode is None:
        mode = "split_index"
    mode = str(mode).strip()
    if mode not in PHONON_MODE_TO_WORKFLOW:
        allowed = ", ".join(PHONON_MODE_TO_WORKFLOW)
        raise ValueError(f"[substitutionflow].phonon_mode 只允许 {allowed}，当前值为 {mode!r}。")
    return mode


def get_case_phonon_workflow(case_dir: str) -> str:
    return PHONON_MODE_TO_WORKFLOW[get_case_phonon_mode(case_dir)]


def get_case_phonon_workdir(case_dir: str) -> str:
    config = _load_case_config(case_dir)
    qe_phonon_config = config.get("qe_phonon", {})
    if isinstance(qe_phonon_config, dict) and "work_dir" in qe_phonon_config and qe_phonon_config["work_dir"] is not None:
        return str(qe_phonon_config["work_dir"])
    return get_case_phonon_workflow(case_dir)


def _load_json(path: str) -> Dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _save_json(path: str, payload: Dict) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, ensure_ascii=False)


def discover_cases(output_dir: str) -> "OrderedDict[str, Dict]":
    cases: "OrderedDict[str, Dict]" = OrderedDict()
    output_dir_abs = os.path.abspath(output_dir)
    output_dir_name = os.path.basename(output_dir_abs.rstrip(os.sep)) or os.path.basename(output_dir_abs)
    manifest_paths = sorted(
        glob.glob(os.path.join(output_dir_abs, "*", "period_*", "workflow_manifest.json"))
    )
    for manifest_path in manifest_paths:
        manifest = _load_json(manifest_path)
        period_dir = os.path.dirname(manifest_path)
        period_rel = os.path.relpath(period_dir, output_dir_abs)
        xa_dir_rel = os.path.dirname(period_rel)
        period_label = os.path.basename(period_dir)
        for case_entry in manifest.get("cases", []):
            case_dir = os.path.join(period_dir, case_entry["case_dir"])
            case_rel = os.path.relpath(case_dir, output_dir_abs)
            cases[case_rel] = {
                "case_rel": case_rel,
                "case_display_rel": os.path.join(output_dir_name, case_rel),
                "case_dir": case_dir,
                "period_dir": period_dir,
                "period_rel": period_rel,
                "xa_dir_rel": xa_dir_rel,
                "period_label": period_label,
                "input_structure": manifest["input_structure"],
                "xa_symbol": manifest["xa_symbol"],
                "formula": case_entry["formula"],
                "xb_symbol": case_entry["xb_symbol"],
                "pressures": list(manifest["pressures"]),
            }
    return cases


def _default_pressure_status(pressures: Iterable) -> Dict[str, Dict[str, str]]:
    result: Dict[str, Dict[str, str]] = OrderedDict()
    for pressure in pressures:
        pressure_key = f"{pressure:g}" if isinstance(pressure, (int, float)) else str(pressure)
        result[pressure_key] = {step: "unstarted" for step in STEP_ORDER}
    return result


def build_default_case_status(case_meta: Dict) -> Dict:
    step_order = get_case_step_order(case_meta["case_dir"])
    pressure_status = OrderedDict()
    for pressure in case_meta["pressures"]:
        pressure_key = f"{pressure:g}" if isinstance(pressure, (int, float)) else str(pressure)
        pressure_status[pressure_key] = {step: "unstarted" for step in step_order}
    return {
        "formula": case_meta["formula"],
        "xa_symbol": case_meta["xa_symbol"],
        "xb_symbol": case_meta["xb_symbol"],
        "input_structure": case_meta["input_structure"],
        "pressures": pressure_status,
    }


def load_case_status(case_dir: str) -> Dict:
    return _load_json(get_case_status_path(case_dir))


def save_case_status(case_dir: str, case_status: Dict) -> None:
    _save_json(get_case_status_path(case_dir), case_status)


def _ensure_case_status_file(case_meta: Dict) -> Dict:
    status_path = get_case_status_path(case_meta["case_dir"])
    expected = build_default_case_status(case_meta)
    step_order = get_case_step_order(case_meta["case_dir"])
    if not os.path.exists(status_path):
        save_case_status(case_meta["case_dir"], expected)
        return expected

    current = _load_json(status_path)
    current_pressures = current.get("pressures", {})
    if sorted(current_pressures.keys()) != sorted(expected["pressures"].keys()):
        save_case_status(case_meta["case_dir"], expected)
        return expected

    for key, value in expected.items():
        if key == "pressures":
            continue
        current[key] = value
    for pressure_key in expected["pressures"]:
        current_pressure = current.setdefault("pressures", {}).get(pressure_key, {})
        normalized_pressure = OrderedDict()
        for step in step_order:
            normalized_pressure[step] = current_pressure.get(step, "unstarted")
        current["pressures"][pressure_key] = normalized_pressure
    save_case_status(case_meta["case_dir"], current)
    return current


def _safe_read_text(path: str) -> str:
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        return f.read()


def _exists_and_nonempty(path: str) -> bool:
    return os.path.exists(path) and os.path.getsize(path) > 0


def _directory_has_any_file(path: str) -> bool:
    if not os.path.isdir(path):
        return False
    for _, _, files in os.walk(path):
        if files:
            return True
    return False


def _outcar_has_markers(outcar_path: str, required_markers: Iterable[str], forbidden_markers: Iterable[str]) -> bool:
    if not os.path.exists(outcar_path):
        return False
    content = _safe_read_text(outcar_path)
    if any(marker not in content for marker in required_markers):
        return False
    if any(marker in content for marker in forbidden_markers):
        return False
    return True


def _extract_nelm(outcar_path: str) -> Optional[int]:
    if not os.path.exists(outcar_path):
        return None
    content = _safe_read_text(outcar_path)
    match = re.search(r"\bNELM\s*=\s*(\d+)", content)
    if not match:
        return None
    return int(match.group(1))


def _extract_iteration_count(outcar_path: str) -> int:
    if not os.path.exists(outcar_path):
        return 0
    content = _safe_read_text(outcar_path)
    return len(re.findall(r"^\s*Iteration", content, flags=re.MULTILINE))


def _is_scf_converged(outcar_path: str) -> bool:
    if not _outcar_has_markers(outcar_path, SCF_REQUIRED_MARKERS, (SCF_FORBIDDEN_MARKER,)):
        return False
    nelm = _extract_nelm(outcar_path)
    if nelm is None:
        return False
    return _extract_iteration_count(outcar_path) < nelm


def _normalize_path(path: str) -> str:
    return os.path.normpath(os.path.abspath(path))


def _parse_squeue_output(output: str) -> Set[str]:
    workdirs: Set[str] = set()
    for line in output.splitlines():
        if not line.strip():
            continue
        parts = line.rstrip().split(maxsplit=5)
        if len(parts) < 6:
            continue
        workdirs.add(_normalize_path(parts[5]))
    return workdirs


def get_active_workdirs(user: str = DEFAULT_SQUEUE_USER) -> Set[str]:
    active_dirs = _get_local_active_workdirs()
    try:
        output = subprocess.check_output(
            ["squeue", "-u", user, "-h", "--format", SQUEUE_FORMAT],
            text=True,
        )
    except Exception:
        return active_dirs
    active_dirs.update(_parse_squeue_output(output))
    return active_dirs


def _is_same_uid_process(pid: str) -> bool:
    status_path = os.path.join("/proc", pid, "status")
    try:
        with open(status_path, "r", encoding="utf-8", errors="ignore") as f:
            for line in f:
                if line.startswith("Uid:"):
                    real_uid = line.split()[1]
                    return int(real_uid) == os.getuid()
    except Exception:
        return False
    return False


def _read_proc_text(path: str) -> str:
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        return f.read()


def _looks_like_workflow_process(cmdline: str) -> bool:
    if not cmdline:
        return False
    keywords = (
        "mattersim_opt",
        "vasp_opt",
        "vasp_scf",
        "vasp_eledos",
        "vasp_process_eledos",
        "qe_phonon",
        "vasp_std",
        "vasp_gam",
        "vasp_ncl",
        "pw.x",
        "ph.x",
        "q2r.x",
        "matdyn.x",
    )
    return any(keyword in cmdline for keyword in keywords)


def _get_local_active_workdirs() -> Set[str]:
    active_dirs: Set[str] = set()
    proc_root = "/proc"
    if not os.path.isdir(proc_root):
        return active_dirs
    current_pid = os.getpid()
    parent_pid = os.getppid()
    for pid in os.listdir(proc_root):
        if not pid.isdigit():
            continue
        pid_int = int(pid)
        if pid_int in {current_pid, parent_pid}:
            continue
        if not _is_same_uid_process(pid):
            continue
        cmdline_path = os.path.join(proc_root, pid, "cmdline")
        cwd_path = os.path.join(proc_root, pid, "cwd")
        with suppress(Exception):
            cmdline = _read_proc_text(cmdline_path).replace("\x00", " ").strip()
            if not _looks_like_workflow_process(cmdline):
                continue
            active_dirs.add(_normalize_path(os.readlink(cwd_path)))
    return active_dirs


def _path_matches_active_dir(target_dir: str, active_dirs: Set[str]) -> bool:
    target_norm = _normalize_path(target_dir)
    for active_dir in active_dirs:
        if target_norm == active_dir:
            return True
        if target_norm.startswith(active_dir + os.sep):
            return True
        if active_dir.startswith(target_norm + os.sep):
            return True
    return False


def _opt_has_trace(step_dir: str) -> bool:
    tracked = ("INCAR", "POSCAR", "POTCAR", "OUTCAR", "CONTCAR", "run_vasp.sh", "oneopt.sh", "fouropt.sh", "fopt.sh")
    return any(os.path.exists(os.path.join(step_dir, name)) for name in tracked) or _directory_has_any_file(step_dir)


def _mattersim_has_trace(step_dir: str) -> bool:
    tracked = ("POSCAR", "CONTCAR", "run_mattersim.sh", "run_mattersim_opt.py", "mattersim_opt.json", "mattersim_opt.log")
    return any(os.path.exists(os.path.join(step_dir, name)) for name in tracked) or _directory_has_any_file(step_dir)


def _mattersim_has_runtime_trace(step_dir: str) -> bool:
    tracked = ("CONTCAR", "mattersim_opt.json", "mattersim_opt.log")
    return any(os.path.exists(os.path.join(step_dir, name)) for name in tracked)


def _generic_step_has_trace(step_dir: str) -> bool:
    return _directory_has_any_file(step_dir)


def _process_has_trace(eledos_dir: str) -> bool:
    tracked = (
        os.path.join(eledos_dir, "pdos_merged.csv"),
        os.path.join(eledos_dir, "pdos_merged.png"),
        os.path.join(eledos_dir, "TDOS.dat"),
    )
    if any(os.path.exists(path) for path in tracked):
        return True
    if glob.glob(os.path.join(eledos_dir, "PDOS_*.dat")):
        return True
    return False


def _detect_mattersim_opt_status(pressure_dir: str, active_dirs: Set[str]) -> str:
    step_dir = os.path.join(pressure_dir, "mattersim_opt")
    json_path = os.path.join(step_dir, "mattersim_opt.json")
    contcar_path = os.path.join(step_dir, "CONTCAR")
    if _exists_and_nonempty(json_path) and _exists_and_nonempty(contcar_path):
        try:
            payload = _load_json(json_path)
        except Exception:
            payload = {}
        stages = payload.get("stages")
        if isinstance(stages, dict) and stages:
            if all(bool(stage_payload.get("converged")) for stage_payload in stages.values()):
                return "finished"
        elif bool(payload.get("stage1", {}).get("converged")) and bool(payload.get("stage2", {}).get("converged")):
                return "finished"
    if _path_matches_active_dir(step_dir, active_dirs):
        return "running"
    if _mattersim_has_runtime_trace(step_dir):
        return "error"
    if _mattersim_has_trace(step_dir):
        return "unstarted"
    return "unstarted"


def _detect_vasp_opt_status(pressure_dir: str, active_dirs: Set[str]) -> str:
    step_dir = os.path.join(pressure_dir, "vasp_opt")
    outcar_path = os.path.join(step_dir, "OUTCAR")
    contcar_path = os.path.join(step_dir, "CONTCAR")
    if (
        _exists_and_nonempty(contcar_path)
        and os.path.exists(outcar_path)
        and OPT_CONVERGED_MARKER in _safe_read_text(outcar_path)
    ):
        return "finished"
    if _path_matches_active_dir(step_dir, active_dirs):
        return "running"
    if _opt_has_trace(step_dir):
        return "error"
    return "unstarted"


def _detect_vasp_scf_status(pressure_dir: str, active_dirs: Set[str]) -> str:
    step_dir = os.path.join(pressure_dir, "vasp_scf")
    outcar_path = os.path.join(step_dir, "OUTCAR")
    chgcar_path = os.path.join(step_dir, "CHGCAR")
    wavecar_path = os.path.join(step_dir, "WAVECAR")
    if (
        _exists_and_nonempty(outcar_path)
        and _exists_and_nonempty(chgcar_path)
        and _exists_and_nonempty(wavecar_path)
        and _is_scf_converged(outcar_path)
    ):
        return "finished"
    if _path_matches_active_dir(step_dir, active_dirs):
        return "running"
    if _generic_step_has_trace(step_dir):
        return "error"
    return "unstarted"


def _detect_vasp_eledos_status(pressure_dir: str, active_dirs: Set[str]) -> str:
    step_dir = os.path.join(pressure_dir, "vasp_eledos")
    outcar_path = os.path.join(step_dir, "OUTCAR")
    doscar_path = os.path.join(step_dir, "DOSCAR")
    if (
        _exists_and_nonempty(outcar_path)
        and _exists_and_nonempty(doscar_path)
        and _is_scf_converged(outcar_path)
    ):
        return "finished"
    if _path_matches_active_dir(step_dir, active_dirs):
        return "running"
    if _generic_step_has_trace(step_dir):
        return "error"
    return "unstarted"


def _detect_vasp_process_eledos_status(pressure_dir: str) -> str:
    eledos_dir = os.path.join(pressure_dir, "vasp_eledos")
    at_ef_files = [path for path in glob.glob(os.path.join(eledos_dir, "*_at_ef.csv")) if _exists_and_nonempty(path)]
    if at_ef_files:
        return "finished"
    if _process_has_trace(eledos_dir):
        return "error"
    return "unstarted"


def _normalize_qe_phonon_state(value: object) -> str:
    state = str(value).strip() if value is not None else "unstarted"
    if state not in QE_PHONON_STATE_ORDER:
        return "unstarted"
    return state


def _summarize_qe_phonon_tasks(tasks_payload: object) -> str:
    if not isinstance(tasks_payload, dict) or not tasks_payload:
        return "unstarted"
    states = [_normalize_qe_phonon_state(state) for state in tasks_payload.values()]
    if states and all(state == "finished" for state in states):
        return "finished"
    if any(state == "running" for state in states):
        return "running"
    if any(state == "error" for state in states):
        return "error"
    return "unstarted"


def reduce_qe_phonon_workflow_status(workflow_name: str, workflow_status: Dict) -> str:
    ph_payload = workflow_status.get("ph", {})
    if workflow_name == "qe_phonon_nosplit":
        stage_states = (
            _normalize_qe_phonon_state(workflow_status.get("scf")),
            _normalize_qe_phonon_state(ph_payload.get("state") if isinstance(ph_payload, dict) else ph_payload),
            _normalize_qe_phonon_state(workflow_status.get("q2r")),
            _normalize_qe_phonon_state(workflow_status.get("matdyn_phband")),
            _normalize_qe_phonon_state(workflow_status.get("matdyn_phdos")),
        )
    elif workflow_name in {"qe_phonon_split_index", "qe_phonon_split_coords"}:
        tasks_payload = ph_payload.get("tasks") if isinstance(ph_payload, dict) else {}
        stage_states = (
            _normalize_qe_phonon_state(workflow_status.get("scf")),
            _normalize_qe_phonon_state(ph_payload.get("dyn0") if isinstance(ph_payload, dict) else None),
            _summarize_qe_phonon_tasks(tasks_payload),
            _normalize_qe_phonon_state(workflow_status.get("merge")),
            _normalize_qe_phonon_state(workflow_status.get("q2r")),
            _normalize_qe_phonon_state(workflow_status.get("matdyn_phband")),
            _normalize_qe_phonon_state(workflow_status.get("matdyn_phdos")),
        )
    else:
        raise ValueError(f"不支持的 qe_phonon 工作流: {workflow_name}")

    if stage_states and all(state == "finished" for state in stage_states):
        return "finished"
    for state in stage_states:
        if state != "finished":
            return state if state in {"running", "error", "unstarted"} else "unstarted"
    return "unstarted"


def _detect_qe_phonon_status(case_dir: str, pressure_dir: str, active_dirs: Set[str]) -> str:
    step_dir = os.path.join(pressure_dir, get_case_phonon_workdir(case_dir))
    workflow_status_path = os.path.join(step_dir, "workflow_status.json")
    if _path_matches_active_dir(step_dir, active_dirs):
        return "running"
    if _exists_and_nonempty(workflow_status_path):
        try:
            workflow_status = _load_json(workflow_status_path)
        except Exception:
            workflow_status = {}
        return reduce_qe_phonon_workflow_status(get_case_phonon_workflow(case_dir), workflow_status)
    return "unstarted"


def _detect_case_pressure_status(case_dir: str, pressure_key: str, active_dirs: Set[str]) -> Dict[str, str]:
    pressure_dir = os.path.join(case_dir, pressure_key)
    step_order = get_case_step_order(case_dir)
    status_map = OrderedDict()
    for step in step_order:
        if step == "mattersim_opt":
            status_map[step] = _detect_mattersim_opt_status(pressure_dir, active_dirs)
        elif step == "vasp_opt":
            status_map[step] = _detect_vasp_opt_status(pressure_dir, active_dirs)
        elif step == "vasp_scf":
            status_map[step] = _detect_vasp_scf_status(pressure_dir, active_dirs)
        elif step == "vasp_eledos":
            status_map[step] = _detect_vasp_eledos_status(pressure_dir, active_dirs)
        elif step == "vasp_process_eledos":
            status_map[step] = _detect_vasp_process_eledos_status(pressure_dir)
        elif step == "qe_phonon":
            status_map[step] = _detect_qe_phonon_status(case_dir, pressure_dir, active_dirs)
    return status_map


def _build_workflow_summary(output_dir: str, cases: "OrderedDict[str, Dict]") -> Dict:
    case_index: Dict[str, Dict] = OrderedDict()
    for case_rel, case_meta in cases.items():
        case_status = load_case_status(case_meta["case_dir"])
        case_index[case_rel] = {
            "formula": case_meta["formula"],
            "xa_symbol": case_meta["xa_symbol"],
            "xb_symbol": case_meta["xb_symbol"],
            "input_structure": case_status["input_structure"],
            "pressures": case_status["pressures"],
        }
    return {
        "root_output_dir": os.path.abspath(output_dir),
        "cases": case_index,
    }


def save_workflow_status(output_dir: str, workflow_status: Dict) -> None:
    _save_json(get_workflow_status_path(output_dir), workflow_status)


def load_workflow_status(output_dir: str) -> Dict:
    return _load_json(get_workflow_status_path(output_dir))


def create_initial_status(output_dir: str) -> Dict:
    cases = discover_cases(output_dir)
    for case_meta in cases.values():
        _ensure_case_status_file(case_meta)
    summary = _build_workflow_summary(output_dir, cases)
    save_workflow_status(output_dir, summary)
    return summary


def refresh_status(output_dir: str, squeue_output: Optional[str] = None, squeue_user: str = DEFAULT_SQUEUE_USER) -> Dict:
    cases = discover_cases(output_dir)
    active_dirs = _parse_squeue_output(squeue_output) if squeue_output is not None else get_active_workdirs(squeue_user)
    for case_meta in cases.values():
        case_status = _ensure_case_status_file(case_meta)
        updated = {
            "formula": case_status["formula"],
            "xa_symbol": case_status["xa_symbol"],
            "xb_symbol": case_status["xb_symbol"],
            "input_structure": case_status["input_structure"],
            "pressures": OrderedDict(),
        }
        for pressure_key in case_status["pressures"]:
            updated["pressures"][pressure_key] = _detect_case_pressure_status(case_meta["case_dir"], pressure_key, active_dirs)
        save_case_status(case_meta["case_dir"], updated)
    summary = _build_workflow_summary(output_dir, cases)
    save_workflow_status(output_dir, summary)
    return summary


def _count_case_pressure_units(workflow_status: Dict) -> Dict[str, int]:
    counts = {status: 0 for status in STATUS_ORDER}
    for case_status in workflow_status.get("cases", {}).values():
        for pressure_status in case_status.get("pressures", {}).values():
            unit_state = "finished"
            if any(step_state == "running" for step_state in pressure_status.values()):
                unit_state = "running"
            elif any(step_state == "error" for step_state in pressure_status.values()):
                unit_state = "error"
            elif any(step_state == "unstarted" for step_state in pressure_status.values()):
                unit_state = "unstarted"
            counts[unit_state] += 1
    return counts


def summarize_status(workflow_status: Dict) -> str:
    total_cases = len(workflow_status.get("cases", {}))
    return f"总 case 数: {total_cases}"
