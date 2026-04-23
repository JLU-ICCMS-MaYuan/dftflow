#!/usr/bin/env python3
import os
import re
import shlex
import signal
import shutil
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from typing import Dict, List, Optional, Sequence, Tuple

from qeflow.qe_check.checks import check_merge, check_ph, check_q2r, check_scf, summarize_task_states
from qeflow.qe_input.builders import (
    build_qe_commands,
    derive_filenames,
    parse_poscar,
    resolve_prefix,
    write_matdyn_phband_input,
    write_matdyn_phdos_input,
    write_ph_input,
    write_q2r_input,
    write_root_run_script,
    write_scf_input,
    write_split_ph_input_assign_coords,
    write_split_ph_input_assign_index,
    write_split_run_script,
)
from qeflow.qe_input.fileio import dump_json, load_json, resolve_workspace
from qeflow.phonon.qgrid import generate_irreducible_qpoints, write_dyn0
from public.job import submit_script


WORKFLOW_STATUS_FILE = "workflow_status.json"
TASK_STATUS_FILE = "task_status.json"


def _run_shell(command: str, cwd: str) -> None:
    subprocess.run(command, shell=True, check=True, cwd=cwd)


def _submit_or_run(run_calc: bool, submit_job: bool, cwd: str, script_name: str) -> bool:
    if run_calc:
        _run_shell(f"bash {shlex.quote(script_name)}", cwd)
        return True
    if submit_job:
        submit_script(cwd, script_name)
        return True
    return False


def _print_stage(stage_no: int, message: str) -> None:
    print(f"{stage_no}. {message}")


def _ph_state(workflow: Dict[str, object]) -> str:
    ph_payload = workflow["ph"]
    return str(ph_payload["state"]) if isinstance(ph_payload, dict) else str(ph_payload)


def _dyn0_state(workflow: Dict[str, object]) -> str:
    ph_payload = workflow["ph"]
    return str(ph_payload["dyn0"]) if isinstance(ph_payload, dict) else "unstarted"


def _merge_state(workflow: Dict[str, object]) -> str:
    return str(workflow.get("merge", "unstarted"))


def _all_split_tasks_finished(workflow: Dict[str, object]) -> bool:
    tasks = workflow.get("tasks", {})
    if not isinstance(tasks, dict) or not tasks:
        return False
    return summarize_task_states(
        str(task["state"]) for task in tasks.values() if isinstance(task, dict)
    ) == "finished"


def _any_split_task_running(workflow: Dict[str, object]) -> bool:
    tasks = workflow.get("tasks", {})
    if not isinstance(tasks, dict) or not tasks:
        return False
    return summarize_task_states(
        str(task["state"]) for task in tasks.values() if isinstance(task, dict)
    ) == "running"


def _task_needs_run(workflow: Dict[str, object]) -> bool:
    tasks = workflow.get("tasks", {})
    if not isinstance(tasks, dict) or not tasks:
        return False
    return summarize_task_states(
        str(task["state"]) for task in tasks.values() if isinstance(task, dict)
    ) != "finished"


def _format_state_line(label: str, state: str) -> str:
    return f"{label}: {state}"


def print_status(prefix: str, workflow: Dict[str, object]) -> None:
    print(_format_state_line("scf", str(workflow["scf"])))
    ph_payload = workflow["ph"]
    if isinstance(ph_payload, dict):
        print(_format_state_line("ph", str(ph_payload["state"])))
        print(_format_state_line("dyn0", str(ph_payload["dyn0"])))
    print(_format_state_line("merge", _merge_state(workflow)))
    print(_format_state_line("q2r", str(workflow["q2r"])))
    print(_format_state_line("matdyn_phband", str(workflow["matdyn_phband"])))
    print(_format_state_line("matdyn_phdos", str(workflow["matdyn_phdos"])))
    tasks = workflow.get("tasks", {})
    if isinstance(tasks, dict) and tasks:
        for q_name in sorted(tasks):
            task_payload = tasks[q_name]
            if isinstance(task_payload, dict):
                print(_format_state_line(q_name, str(task_payload.get("state", "unstarted"))))
    print(f"prefix: {prefix}")


def refresh_split_task_status(workspace: str) -> Dict[str, Dict[str, str]]:
    from qeflow.qe_check.checks import check_split_task, get_running_workdirs

    running = get_running_workdirs()
    task_payload: Dict[str, Dict[str, str]] = {}
    q_names = _iter_q_directory_names(workspace)
    if not q_names:
        from qeflow.qe_input.fileio import list_split_directories

        q_names = [os.path.basename(q_dir) for q_dir in list_split_directories(workspace)]
    for q_name in q_names:
        q_dir = os.path.join(workspace, q_name)
        payload = {
            "directory": q_name,
            "state": check_split_task(q_dir, running),
        }
        dump_json(os.path.join(q_dir, TASK_STATUS_FILE), payload)
        task_payload[q_name] = payload
    return task_payload


def refresh_workflow_status(workspace: str, prefix: str) -> Dict[str, object]:
    from qeflow.qe_check.checks import check_matdyn, get_running_workdirs

    running = get_running_workdirs()
    workflow = {
        "scf": check_scf(workspace, running_dirs=running),
        "ph": check_ph(workspace, prefix, running_dirs=running),
        "merge": check_merge(workspace, prefix, running_dirs=running),
        "q2r": check_q2r(workspace, prefix, running_dirs=running),
        "matdyn_phband": check_matdyn(
            workspace,
            "matdyn_phband.out",
            f"{prefix}.phband.freq",
            running_dirs=running,
        ),
        "matdyn_phdos": check_matdyn(
            workspace,
            "matdyn_phdos.out",
            f"{prefix}.phdos.dat",
            running_dirs=running,
        ),
    }
    workflow["tasks"] = refresh_split_task_status(workspace)
    dump_json(os.path.join(workspace, WORKFLOW_STATUS_FILE), workflow)
    return workflow


def _load_q_index_entries(workspace: str) -> List[Dict[str, object]]:
    payload = load_json(os.path.join(workspace, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return []
    qpoints = payload.get("qpoints", [])
    return qpoints if isinstance(qpoints, list) else []


def _load_q_index_mode(workspace: str) -> str:
    payload = load_json(os.path.join(workspace, "q_index_map.json"), default={})
    if not isinstance(payload, dict):
        return ""
    mode = payload.get("mode")
    return str(mode) if isinstance(mode, str) else ""


def _iter_q_directory_names(workspace: str) -> List[str]:
    names: List[str] = []
    for entry in _load_q_index_entries(workspace):
        if not isinstance(entry, dict):
            continue
        directory = str(entry.get("directory", "")).strip()
        if directory:
            names.append(directory)
    return names


@dataclass
class MergeResult:
    success: bool
    errors: List[str]
    dyn_summaries: List[Tuple[int, List[float]]]


def _split_output_finished(path: str) -> bool:
    from qeflow.qe_check.outputs import PhOut

    return PhOut(path).is_finished()


def _find_single_source(
    candidates: Sequence[str],
    label: str,
    errors: List[str],
) -> Optional[str]:
    existing = [path for path in candidates if os.path.exists(path)]
    if not existing:
        errors.append(f"{label}: 缺少候选源文件")
        return None
    if len(existing) > 1:
        errors.append(f"{label}: 存在多个候选源文件 -> {', '.join(existing)}")
        return None
    return existing[0]


def _find_dyn_source_for_coords(q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
    return _find_single_source(
        [
            os.path.join(q_dir, f"{prefix}.dyn"),
            os.path.join(q_dir, f"{prefix}.dyn{q_index}"),
        ],
        f"{q_dir} dyn{q_index}",
        errors,
    )


def _find_dyn_source_for_index(q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
    exact = os.path.join(q_dir, f"{prefix}.dyn{q_index}")
    if os.path.exists(exact):
        return exact
    return _find_single_source(
        [os.path.join(q_dir, f"{prefix}.dyn")],
        f"{q_dir} dyn{q_index}",
        errors,
    )


def _find_q_phdir_source_for_coords(q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
    exact = os.path.join(q_dir, "_ph0", f"{prefix}.q_{q_index}")
    if os.path.isdir(exact):
        return exact
    return None


def _find_q_phdir_source_for_index(q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
    if q_index == 1:
        root_ph0 = os.path.join(q_dir, "_ph0")
        exact = os.path.join(root_ph0, f"{prefix}.q_1")
        if os.path.isdir(exact):
            return exact
        if os.path.exists(os.path.join(root_ph0, "dvscf")):
            return root_ph0
        errors.append(f"{q_dir} q_{q_index}: 缺少 _ph0/dvscf 或 _ph0/{prefix}.q_1")
        return None
    exact = os.path.join(q_dir, "_ph0", f"{prefix}.q_{q_index}")
    if os.path.isdir(exact):
        return exact
    errors.append(f"{q_dir} q_{q_index}: 缺少 _ph0/{prefix}.q_{q_index} 目录")
    return None


def _find_phsave_source_for_coords(q_dir: str, prefix: str, errors: List[str]) -> Optional[str]:
    phsave = os.path.join(q_dir, "_ph0", f"{prefix}.phsave")
    if not os.path.isdir(phsave):
        errors.append(f"{q_dir}: 缺少 _ph0/{prefix}.phsave")
        return None
    if not os.path.exists(os.path.join(phsave, "patterns.1.xml")):
        errors.append(f"{q_dir}: 缺少 _ph0/{prefix}.phsave/patterns.1.xml")
        return None
    return phsave


def _find_phsave_source_for_index(q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
    phsave = os.path.join(q_dir, "_ph0", f"{prefix}.phsave")
    if not os.path.isdir(phsave):
        errors.append(f"{q_dir}: 缺少 _ph0/{prefix}.phsave")
        return None
    pattern_path = os.path.join(phsave, f"patterns.{q_index}.xml")
    if not os.path.exists(pattern_path):
        errors.append(f"{q_dir}: 缺少 _ph0/{prefix}.phsave/patterns.{q_index}.xml")
        return None
    return phsave


def _copy_file(src: str, dst: str) -> None:
    os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
    shutil.copy2(src, dst)


def _copy_tree(src: str, dst: str) -> None:
    os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
    shutil.copytree(src, dst, dirs_exist_ok=True)


def _rewrite_pattern_qpoint_number(path: str, q_index: int) -> None:
    tree = ET.parse(path)
    root = tree.getroot()
    node = root.find(".//QPOINT_NUMBER")
    if node is not None:
        node.text = str(q_index)
        tree.write(path, encoding="utf-8", xml_declaration=True)


def _merge_phsave_tree(src: str, dst: str) -> None:
    _copy_tree(src, dst)


def _merge_patterns_for_index(src: str, dst: str, q_index: int) -> None:
    os.makedirs(dst, exist_ok=True)
    src_pattern = os.path.join(src, f"patterns.{q_index}.xml")
    dst_pattern = os.path.join(dst, f"patterns.{q_index}.xml")
    _copy_file(src_pattern, dst_pattern)
    compat_pattern = os.path.join(dst, "patterns.1.xml")
    _copy_file(src_pattern, compat_pattern)
    _rewrite_pattern_qpoint_number(compat_pattern, q_index)


def _extract_lowest_frequencies(dyn_path: str, limit: int = 5) -> List[float]:
    values: List[float] = []
    with open(dyn_path, "r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            if "freq" not in line.lower():
                continue
            values.extend(float(item) for item in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?", line))
    if not values:
        with open(dyn_path, "r", encoding="utf-8", errors="ignore") as handle:
            for line in handle:
                values.extend(float(item) for item in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?", line))
    return sorted(values)[:limit]


def merge_split_results_for_coords(workspace: str, prefix: str) -> MergeResult:
    entries = _load_q_index_entries(workspace)
    errors: List[str] = []
    dyn_summaries: List[Tuple[int, List[float]]] = []
    if not entries:
        return MergeResult(False, ["缺少 q_index_map.json 或其中不包含 qpoints"], dyn_summaries)

    for entry in entries:
        if not isinstance(entry, dict):
            errors.append("q_index_map.json 中存在非字典 qpoints 项")
            continue
        q_name = str(entry.get("directory", "")).strip()
        q_index = entry.get("index")
        if not q_name or q_index is None:
            errors.append(f"q_index_map.json 条目缺少 directory/index: {entry}")
            continue
        q_index_int = int(q_index)
        q_dir = os.path.join(workspace, q_name)
        split_out = os.path.join(q_dir, "split_ph.out")
        if not _split_output_finished(split_out):
            errors.append(f"{q_name}: split_ph.out 未完成，禁止 merge")
            continue

        split_in = os.path.join(q_dir, "split_ph.in")
        if not os.path.exists(split_in):
            errors.append(f"{q_name}: 缺少 split_ph.in")
        if not os.path.exists(split_out):
            errors.append(f"{q_name}: 缺少 split_ph.out")

        dyn_source = _find_dyn_source_for_coords(q_dir, prefix, q_index_int, errors)
        q_phdir_source = _find_q_phdir_source_for_coords(q_dir, prefix, q_index_int, errors)
        phsave_source = _find_phsave_source_for_coords(q_dir, prefix, errors)

        if os.path.exists(split_in):
            _copy_file(split_in, os.path.join(workspace, f"split_ph.in.{q_index_int}"))
        if os.path.exists(split_out):
            _copy_file(split_out, os.path.join(workspace, f"split_ph.out.{q_index_int}"))
        if dyn_source is not None:
            dyn_target = os.path.join(workspace, f"{prefix}.dyn{q_index_int}")
            _copy_file(dyn_source, dyn_target)
            dyn_summaries.append((q_index_int, _extract_lowest_frequencies(dyn_target)))
        if q_phdir_source is not None:
            _copy_tree(q_phdir_source, os.path.join(workspace, "_ph0", f"{prefix}.q_{q_index_int}"))
        else:
            dvscf_source = os.path.join(q_dir, "_ph0", "dvscf")
            if os.path.exists(dvscf_source):
                _copy_file(dvscf_source, os.path.join(workspace, "_ph0", "dvscf"))
        if phsave_source is not None:
            _merge_phsave_tree(phsave_source, os.path.join(workspace, "_ph0", f"{prefix}.phsave"))

    if errors:
        return MergeResult(False, errors, dyn_summaries)

    return MergeResult(check_merge(workspace, prefix) == "finished", [], dyn_summaries)


def merge_split_results_for_index(workspace: str, prefix: str) -> MergeResult:
    entries = _load_q_index_entries(workspace)
    errors: List[str] = []
    dyn_summaries: List[Tuple[int, List[float]]] = []
    if not entries:
        return MergeResult(False, ["缺少 q_index_map.json 或其中不包含 qpoints"], dyn_summaries)

    for entry in entries:
        if not isinstance(entry, dict):
            errors.append("q_index_map.json 中存在非字典 qpoints 项")
            continue
        q_name = str(entry.get("directory", "")).strip()
        q_index = entry.get("index")
        if not q_name or q_index is None:
            errors.append(f"q_index_map.json 条目缺少 directory/index: {entry}")
            continue
        q_index_int = int(q_index)
        q_dir = os.path.join(workspace, q_name)
        split_out = os.path.join(q_dir, "split_ph.out")
        if not _split_output_finished(split_out):
            errors.append(f"{q_name}: split_ph.out 未完成，禁止 merge")
            continue

        split_in = os.path.join(q_dir, "split_ph.in")
        if not os.path.exists(split_in):
            errors.append(f"{q_name}: 缺少 split_ph.in")
        if not os.path.exists(split_out):
            errors.append(f"{q_name}: 缺少 split_ph.out")

        dyn_source = _find_dyn_source_for_index(q_dir, prefix, q_index_int, errors)
        q_phdir_source = _find_q_phdir_source_for_index(q_dir, prefix, q_index_int, errors)
        phsave_source = _find_phsave_source_for_index(q_dir, prefix, q_index_int, errors)

        if os.path.exists(split_in):
            _copy_file(split_in, os.path.join(workspace, f"split_ph.in.{q_index_int}"))
        if os.path.exists(split_out):
            _copy_file(split_out, os.path.join(workspace, f"split_ph.out.{q_index_int}"))
        if dyn_source is not None:
            dyn_target = os.path.join(workspace, f"{prefix}.dyn{q_index_int}")
            _copy_file(dyn_source, dyn_target)
            dyn_summaries.append((q_index_int, _extract_lowest_frequencies(dyn_target)))
        if q_phdir_source is not None:
            if q_index_int == 1 and os.path.basename(q_phdir_source) != f"{prefix}.q_1":
                dvscf_source = os.path.join(q_phdir_source, "dvscf")
                if os.path.exists(dvscf_source):
                    _copy_file(dvscf_source, os.path.join(workspace, "_ph0", "dvscf"))
            else:
                _copy_tree(q_phdir_source, os.path.join(workspace, "_ph0", f"{prefix}.q_{q_index_int}"))
        if phsave_source is not None:
            dst = os.path.join(workspace, "_ph0", f"{prefix}.phsave")
            _copy_tree(phsave_source, dst)
            _merge_patterns_for_index(phsave_source, dst, q_index_int)

    if errors:
        return MergeResult(False, errors, dyn_summaries)

    return MergeResult(check_merge(workspace, prefix) == "finished", [], dyn_summaries)


def merge_split_results(workspace: str, prefix: str) -> MergeResult:
    mode = _load_q_index_mode(workspace)
    if mode == "split_assign_coords":
        return merge_split_results_for_coords(workspace, prefix)
    if mode == "split_assign_index":
        return merge_split_results_for_index(workspace, prefix)
    return MergeResult(False, [f"q_index_map.json 缺少合法 mode: {mode or '未设置'}"], [])


def _print_merge_result(prefix: str, result: MergeResult) -> None:
    if result.success:
        print("merge 成功，dyn 文件最低前 5 个频率如下：")
        for q_index, values in result.dyn_summaries:
            if values:
                formatted = ", ".join(f"{value:.6f}" for value in values)
            else:
                formatted = "无可解析频率"
            print(f"  {prefix}.dyn{q_index}: {formatted}")
        return
    print("merge 失败，发现以下问题：")
    for message in result.errors:
        print(f"  - {message}")


def run_ph_to_dyn0(workspace: str, ph_command: str, dyn0_path: str, output_path: str) -> None:
    # dyn0 预处理只针对根目录 ph.in；stderr 也重定向到 ph.out，避免主动终止时刷屏。
    command = f"{ph_command} < ph.in > ph.out 2>&1"
    process = subprocess.Popen(
        command,
        shell=True,
        cwd=workspace,
        preexec_fn=os.setsid,
    )
    terminated_after_dyn0 = False
    try:
        while True:
            if os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0:
                os.killpg(os.getpgid(process.pid), signal.SIGTERM)
                process.wait(timeout=10)
                terminated_after_dyn0 = True
                return
            if os.path.exists(output_path):
                with open(output_path, "r", encoding="utf-8", errors="ignore") as handle:
                    content = handle.read().lower()
                if "%%%%%%%%%%%%%%" in content or "error in routine" in content:
                    process.wait(timeout=10)
                    raise RuntimeError("ph.x 在生成 dyn0 前已报错，终止 dyn0 模式。")
            if process.poll() is not None:
                if os.path.exists(dyn0_path) and os.path.getsize(dyn0_path) > 0:
                    return
                raise RuntimeError("ph.x 已退出，但未生成有效的 dyn0 文件。")
            time.sleep(0.5)
    finally:
        if not terminated_after_dyn0 and process.poll() is None:
            os.killpg(os.getpgid(process.pid), signal.SIGTERM)
            process.wait(timeout=10)


def _resolve_xml_path(workspace: str, config: Dict[str, object], prefix: str) -> str:
    outdir = str(config["qe_phonon"]["scf"].get("outdir", "./out")).strip("'").strip('"')
    return os.path.join(workspace, outdir, f"{prefix}.save", "data-file-schema.xml")


def materialize_split_directories(
    workspace: str,
    config: Dict[str, object],
    prefix: str,
    mode: str,
) -> int:
    xml_path = _resolve_xml_path(workspace, config, prefix)
    if not os.path.exists(xml_path):
        raise FileNotFoundError(f"找不到不可约 q 点所需 XML: {xml_path}")
    qpoints = generate_irreducible_qpoints(
        xml_path,
        int(config["qe_phonon"]["ph"]["qpoints"][0]),
        int(config["qe_phonon"]["ph"]["qpoints"][1]),
        int(config["qe_phonon"]["ph"]["qpoints"][2]),
    )
    dyn0_path = os.path.join(workspace, derive_filenames(prefix)["dyn0"])
    write_dyn0(
        dyn0_path,
        int(config["qe_phonon"]["ph"]["qpoints"][0]),
        int(config["qe_phonon"]["ph"]["qpoints"][1]),
        int(config["qe_phonon"]["ph"]["qpoints"][2]),
        qpoints,
    )
    commands = build_qe_commands(config)
    slurm_header = str(config.get("slurm", {}).get("header", ""))
    struct_info = parse_poscar(os.path.join(workspace, "POSCAR"))
    q_map_payload = {"mode": mode, "qpoints": []}
    for dir_index, qpoint in enumerate(qpoints):
        q_name = f"q_{dir_index + 1:02d}"
        q_dir = os.path.join(workspace, q_name)
        os.makedirs(q_dir, exist_ok=True)
        write_scf_input(q_dir, config, struct_info, prefix)
        if mode == "split_assign_index":
            write_split_ph_input_assign_index(q_dir, config, prefix, int(qpoint["index"]))
        else:
            write_split_ph_input_assign_coords(q_dir, config, prefix, qpoint["q_crystal"])
        write_split_run_script(q_dir, slurm_header, commands["pw"], commands["ph"])
        q_map_payload["qpoints"].append(
            {
                "directory": q_name,
                "index": int(qpoint["index"]),
                "q_crystal": qpoint["q_crystal"],
                "q_cart": qpoint["q_cart"],
                "weight": qpoint["weight"],
            }
        )
    dump_json(os.path.join(workspace, "q_index_map.json"), q_map_payload)
    return len(q_map_payload["qpoints"])


@dataclass
class PhononStepContext:
    config: Dict[str, object]
    workspace: str
    prefix: str
    struct_info: Dict[str, object]
    commands: Dict[str, str]


def create_context(config_file: str, struct_file: Optional[str]) -> PhononStepContext:
    config, _root_dir, workspace = resolve_workspace(config_file, struct_file, prepare_inputs=True)
    struct_info = parse_poscar(os.path.join(workspace, "POSCAR"))
    prefix = resolve_prefix(config, struct_info)
    commands = build_qe_commands(config)
    return PhononStepContext(
        config=config,
        workspace=workspace,
        prefix=prefix,
        struct_info=struct_info,
        commands=commands,
    )


def prepare_scf(context: PhononStepContext) -> str:
    write_scf_input(context.workspace, context.config, context.struct_info, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_scf.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['pw']} < scf.in > scf.out"],
    )


def prepare_ph(context: PhononStepContext, dyn0_only: bool = False) -> str:
    write_ph_input(context.workspace, context.config, context.prefix, dyn0_only=dyn0_only)
    return write_root_run_script(
        context.workspace,
        "run_ph.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['ph']} < ph.in > ph.out"],
    )


def prepare_q2r(context: PhononStepContext) -> str:
    write_q2r_input(context.workspace, context.config, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_q2r.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['q2r']} < q2r.in > q2r.out"],
    )


def prepare_matdyn_phband(context: PhononStepContext) -> str:
    write_matdyn_phband_input(context.workspace, context.config, context.struct_info, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_matdyn_phband.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['matdyn']} < matdyn_phband.in > matdyn_phband.out"],
    )


def prepare_matdyn_phdos(context: PhononStepContext) -> str:
    write_matdyn_phdos_input(context.workspace, context.config, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_matdyn_phdos.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['matdyn']} < matdyn_phdos.in > matdyn_phdos.out"],
    )


def prepare_merge(context: PhononStepContext) -> str:
    return write_root_run_script(
        context.workspace,
        "run_merge.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{shlex.quote(sys.executable)} -m qeflow.phonon.qe_merge -c input.toml -r"],
    )


def prepare_root_inputs(context: PhononStepContext, mode: str) -> None:
    prepare_scf(context)
    prepare_ph(context, dyn0_only=(mode != "nosplit"))
    prepare_merge(context)
    prepare_q2r(context)
    prepare_matdyn_phband(context)
    prepare_matdyn_phdos(context)


def _root_step_scripts(workflow: Dict[str, object], include_ph: bool) -> List[str]:
    steps: List[str] = []
    if workflow["scf"] not in {"finished", "running"}:
        steps.append("run_scf.sh")
    if include_ph and _ph_state(workflow) not in {"finished", "running"}:
        steps.append("run_ph.sh")
    if workflow["q2r"] not in {"finished", "running"}:
        steps.append("run_q2r.sh")
    if workflow["matdyn_phband"] not in {"finished", "running"}:
        steps.append("run_matdyn_phband.sh")
    if workflow["matdyn_phdos"] not in {"finished", "running"}:
        steps.append("run_matdyn_phdos.sh")
    return steps


def _root_has_running_step(workflow: Dict[str, object], include_ph: bool) -> bool:
    if workflow["scf"] == "running":
        return True
    if include_ph and _ph_state(workflow) == "running":
        return True
    if workflow["q2r"] == "running":
        return True
    if workflow["matdyn_phband"] == "running":
        return True
    if workflow["matdyn_phdos"] == "running":
        return True
    return False


def _write_aggregate_script(context: PhononStepContext, script_name: str, scripts: List[str]) -> str:
    return write_root_run_script(
        context.workspace,
        script_name,
        str(context.config.get("slurm", {}).get("header", "")),
        [f"bash {script_name}" for script_name in scripts],
    )


def run_merge_stage(context: PhononStepContext) -> bool:
    result = merge_split_results(context.workspace, context.prefix)
    _print_merge_result(context.prefix, result)
    refresh_workflow_status(context.workspace, context.prefix)
    return result.success


def _run_root_scripts(context: PhononStepContext, scripts: List[str]) -> None:
    labels = {
        "run_scf.sh": "运行根目录 SCF",
        "run_ph.sh": "运行根目录 PH",
        "run_q2r.sh": "运行 q2r",
        "run_matdyn_phband.sh": "运行 matdyn_phband",
        "run_matdyn_phdos.sh": "运行 matdyn_phdos",
    }
    stage_no = 1
    for script_name in scripts:
        _print_stage(stage_no, labels.get(script_name, f"运行 {script_name}"))
        _run_shell(f"bash {shlex.quote(script_name)}", context.workspace)
        refresh_workflow_status(context.workspace, context.prefix)
        stage_no += 1


def _write_split_stage_script(context: PhononStepContext, workflow: Dict[str, object]) -> str:
    tasks = workflow.get("tasks", {})
    commands: List[str] = []
    if isinstance(tasks, dict):
        for q_name in _iter_q_directory_names(context.workspace):
            task_payload = tasks.get(q_name)
            if isinstance(task_payload, dict) and task_payload.get("state") not in {"finished", "running"}:
                commands.append(f"(cd {shlex.quote(q_name)} && bash run_split.sh)")
    return write_root_run_script(
        context.workspace,
        "submit_split_stage.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        commands,
    )


def run_standalone_ph(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, dyn0_only: bool) -> int:
    context = create_context(config_file, struct_file)
    prepare_scf(context)
    script = prepare_ph(context, dyn0_only=dyn0_only)
    if dyn0_only and run_calc:
        _print_stage(1, "运行根目录 dyn0 预处理")
        run_ph_to_dyn0(
            context.workspace,
            context.commands["ph"],
            os.path.join(context.workspace, f"{context.prefix}.dyn0"),
            os.path.join(context.workspace, "ph.out"),
        )
    else:
        if run_calc:
            _print_stage(1, "运行根目录 PH")
        elif submit_job:
            _print_stage(1, "提交根目录 PH")
        _submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_standalone_q2r(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool) -> int:
    context = create_context(config_file, struct_file)
    script = prepare_q2r(context)
    if run_calc:
        _print_stage(1, "运行 q2r")
    elif submit_job:
        _print_stage(1, "提交 q2r")
    _submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_standalone_matdyn(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, target: str) -> int:
    context = create_context(config_file, struct_file)
    if target == "phband":
        script = prepare_matdyn_phband(context)
        label = "matdyn_phband"
    else:
        script = prepare_matdyn_phdos(context)
        label = "matdyn_phdos"
    if run_calc:
        _print_stage(1, f"运行 {label}")
    elif submit_job:
        _print_stage(1, f"提交 {label}")
    _submit_or_run(run_calc, submit_job, context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def run_standalone_merge(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool) -> int:
    context = create_context(config_file, struct_file)
    script = prepare_merge(context)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if not (run_calc or submit_job):
        print_status(context.prefix, workflow)
        return 0
    if run_calc:
        _print_stage(1, "执行 merge")
        return 0 if run_merge_stage(context) else 1
    _print_stage(1, "提交 merge")
    submit_script(context.workspace, os.path.basename(script))
    refresh_workflow_status(context.workspace, context.prefix)
    return 0


def _ensure_split_materialized(context: PhononStepContext, mode: str) -> Dict[str, object]:
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if _dyn0_state(workflow) == "finished" and not load_json(os.path.join(context.workspace, "q_index_map.json")):
        _print_stage(2, "生成 split q 点输入")
        materialize_split_directories(context.workspace, context.config, context.prefix, mode)
        workflow = refresh_workflow_status(context.workspace, context.prefix)
    return workflow


def run_standalone_split(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, mode: str) -> int:
    context = create_context(config_file, struct_file)
    scf_state = check_scf(context.workspace)
    if scf_state != "finished":
        raise RuntimeError("split 模式依赖根目录已完成的 scf.out。")
    prepare_ph(context, dyn0_only=True)
    prepare_merge(context)
    prepare_q2r(context)
    prepare_matdyn_phband(context)
    prepare_matdyn_phdos(context)
    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if _all_split_tasks_finished(workflow):
        if _merge_state(workflow) != "finished":
            _print_stage(1, "执行 merge")
            if not run_merge_stage(context):
                return 1
            workflow = refresh_workflow_status(context.workspace, context.prefix)
        if submit_job:
            remaining_scripts = _root_step_scripts(workflow, include_ph=False)
            if remaining_scripts and not _root_has_running_step(workflow, include_ph=False):
                _print_stage(2, "提交 q2r + matdyn 后处理")
                aggregate = _write_aggregate_script(context, "submit_post_stage.sh", remaining_scripts)
                submit_script(context.workspace, os.path.basename(aggregate))
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    if _dyn0_state(workflow) != "finished":
        if run_calc:
            _print_stage(1, "运行根目录 dyn0 预处理")
            run_ph_to_dyn0(
                context.workspace,
                context.commands["ph"],
                os.path.join(context.workspace, f"{context.prefix}.dyn0"),
                os.path.join(context.workspace, "ph.out"),
            )
        elif submit_job:
            _print_stage(1, "提交根目录 dyn0 预处理")
            submit_script(context.workspace, "run_ph.sh")
            return 0
        elif not load_json(os.path.join(context.workspace, "q_index_map.json")):
            _print_stage(1, "生成 split q 点输入")
            materialize_split_directories(context.workspace, context.config, context.prefix, mode)
    workflow = _ensure_split_materialized(context, mode)
    tasks = workflow.get("tasks", {})
    q_names = _iter_q_directory_names(context.workspace)
    if run_calc or submit_job:
        for q_name in q_names:
            if run_calc:
                _print_stage(3, f"运行 split 任务 {q_name}")
                _run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
            else:
                _print_stage(3, f"提交 split 任务 {q_name}")
                submit_script(os.path.join(context.workspace, q_name), "run_split.sh")
    refresh_workflow_status(context.workspace, context.prefix)
    if not (run_calc or submit_job):
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
    return 0


def run_phonon_workflow(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, mode: str) -> int:
    context = create_context(config_file, struct_file)
    prepare_root_inputs(context, mode)
    workflow = refresh_workflow_status(context.workspace, context.prefix)

    if mode == "nosplit":
        remaining_scripts = _root_step_scripts(workflow, include_ph=True)
        if run_calc:
            _run_root_scripts(context, remaining_scripts)
        elif submit_job and remaining_scripts and not _root_has_running_step(workflow, include_ph=True):
            _print_stage(1, "提交整链剩余步骤")
            aggregate = _write_aggregate_script(context, "submit_phonon_workflow.sh", remaining_scripts)
            submit_script(context.workspace, os.path.basename(aggregate))
        print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
        return 0

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if workflow["scf"] != "finished" or _dyn0_state(workflow) != "finished":
        if run_calc:
            if workflow["scf"] != "finished":
                _print_stage(1, "运行根目录 SCF")
                _run_shell("bash run_scf.sh", context.workspace)
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            if _dyn0_state(workflow) != "finished":
                _print_stage(2, "运行根目录 dyn0 预处理")
                run_ph_to_dyn0(
                    context.workspace,
                    context.commands["ph"],
                    os.path.join(context.workspace, f"{context.prefix}.dyn0"),
                    os.path.join(context.workspace, "ph.out"),
                )
                workflow = refresh_workflow_status(context.workspace, context.prefix)
        elif submit_job:
            stage_scripts = []
            if workflow["scf"] not in {"finished", "running"}:
                stage_scripts.append("run_scf.sh")
            if _dyn0_state(workflow) not in {"finished", "running"}:
                stage_scripts.append("run_ph.sh")
            if stage_scripts:
                _print_stage(1, "提交前计算阶段")
                aggregate = _write_aggregate_script(context, "submit_pre_stage.sh", stage_scripts)
                submit_script(context.workspace, os.path.basename(aggregate))
            print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
            return 0

    workflow = _ensure_split_materialized(context, mode)

    if _task_needs_run(workflow):
        if run_calc:
            tasks = workflow.get("tasks", {})
            if isinstance(tasks, dict):
                for q_name in sorted(tasks):
                    task_payload = tasks[q_name]
                    if isinstance(task_payload, dict) and task_payload.get("state") != "finished":
                        _print_stage(4, f"运行 split 任务 {q_name}")
                        _run_shell("bash run_split.sh", os.path.join(context.workspace, q_name))
                        workflow = refresh_workflow_status(context.workspace, context.prefix)
        elif submit_job and not _any_split_task_running(workflow):
            _print_stage(2, "提交 split 阶段")
            aggregate = _write_split_stage_script(context, workflow)
            submit_script(context.workspace, os.path.basename(aggregate))
            print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
            return 0

    workflow = refresh_workflow_status(context.workspace, context.prefix)
    if _all_split_tasks_finished(workflow):
        if run_calc:
            if _merge_state(workflow) != "finished":
                _print_stage(5, "执行 merge")
                if not run_merge_stage(context):
                    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
                    return 1
                workflow = refresh_workflow_status(context.workspace, context.prefix)
            remaining_scripts = _root_step_scripts(workflow, include_ph=False)
            labels = {
                "run_q2r.sh": "运行 q2r",
                "run_matdyn_phband.sh": "运行 matdyn_phband",
                "run_matdyn_phdos.sh": "运行 matdyn_phdos",
            }
            stage_no = 6
            for script_name in remaining_scripts:
                _print_stage(stage_no, labels.get(script_name, f"运行 {script_name}"))
                _run_shell(f"bash {shlex.quote(script_name)}", context.workspace)
                refresh_workflow_status(context.workspace, context.prefix)
                stage_no += 1
        elif submit_job and not _root_has_running_step(workflow, include_ph=False):
            if _merge_state(workflow) != "finished":
                _print_stage(3, "提交 merge 阶段")
                aggregate = _write_aggregate_script(context, "submit_merge_stage.sh", ["run_merge.sh"])
                submit_script(context.workspace, os.path.basename(aggregate))
                print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
                return 0
            remaining_scripts = _root_step_scripts(workflow, include_ph=False)
            if remaining_scripts:
                _print_stage(4, "提交后处理阶段")
                aggregate = _write_aggregate_script(context, "submit_post_stage.sh", remaining_scripts)
                submit_script(context.workspace, os.path.basename(aggregate))
    print_status(context.prefix, refresh_workflow_status(context.workspace, context.prefix))
    return 0
