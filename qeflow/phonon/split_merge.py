#!/usr/bin/env python3
import os
import re
import shutil
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from typing import List, Optional, Sequence, Tuple

from qeflow.phonon.q_index_map import load_q_index_entries, load_q_index_mode
from qeflow.qe_check.checks import check_merge


@dataclass
class MergeResult:
    success: bool
    errors: List[str]
    dyn_summaries: List[Tuple[int, List[float]]]


def merge_split_results(workspace: str, prefix: str) -> MergeResult:
    mode = load_q_index_mode(workspace)
    if mode == "split_assign_coords":
        return merge_split_results_for_coords(workspace, prefix)
    if mode == "split_assign_index":
        return merge_split_results_for_index(workspace, prefix)
    return MergeResult(False, [f"q_index_map.json 缺少合法 mode: {mode or '未设置'}"], [])


def merge_split_results_for_coords(workspace: str, prefix: str) -> MergeResult:
    return _merge_split_results(workspace, prefix, SplitMergeStrategy("coords"))


def merge_split_results_for_index(workspace: str, prefix: str) -> MergeResult:
    return _merge_split_results(workspace, prefix, SplitMergeStrategy("index"))


class SplitMergeStrategy:
    def __init__(self, mode: str) -> None:
        if mode not in {"coords", "index"}:
            raise ValueError(f"不支持的 split merge 模式: {mode}")
        self.mode = mode

    def find_dyn_source(self, q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
        if self.mode == "coords":
            return _find_single_source(
                [
                    os.path.join(q_dir, f"{prefix}.dyn"),
                    os.path.join(q_dir, f"{prefix}.dyn{q_index}"),
                ],
                f"{q_dir} dyn{q_index}",
                errors,
            )
        exact = os.path.join(q_dir, f"{prefix}.dyn{q_index}")
        if os.path.exists(exact):
            return exact
        return _find_single_source([os.path.join(q_dir, f"{prefix}.dyn")], f"{q_dir} dyn{q_index}", errors)

    def find_q_phdir_source(self, q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
        q_ph_root = _resolve_ph_root(q_dir)
        exact = os.path.join(q_ph_root, f"{prefix}.q_{q_index}")
        if os.path.isdir(exact):
            return exact
        if self.mode == "coords":
            return None
        if q_index == 1:
            if os.path.exists(os.path.join(q_ph_root, f"{prefix}.dvscf1")):
                return q_ph_root
            if os.path.exists(os.path.join(q_ph_root, "dvscf")):
                return q_ph_root
            errors.append(f"{q_dir} q_{q_index}: 缺少 tmp/_ph0/{prefix}.dvscf1、_ph0/dvscf 或 _ph0/{prefix}.q_1")
            return None
        errors.append(f"{q_dir} q_{q_index}: 缺少 tmp/_ph0/{prefix}.q_{q_index} 目录")
        return None

    def find_phsave_source(self, q_dir: str, prefix: str, q_index: int, errors: List[str]) -> Optional[str]:
        phsave = os.path.join(_resolve_ph_root(q_dir), f"{prefix}.phsave")
        if not os.path.isdir(phsave):
            errors.append(f"{q_dir}: 缺少 tmp/_ph0/{prefix}.phsave")
            return None
        pattern_name = "patterns.1.xml" if self.mode == "coords" else f"patterns.{q_index}.xml"
        pattern_path = os.path.join(phsave, pattern_name)
        if not os.path.exists(pattern_path):
            errors.append(f"{q_dir}: 缺少 tmp/_ph0/{prefix}.phsave/{pattern_name}")
            return None
        return phsave

    def copy_q_phdir(self, source: str, target_root: str, prefix: str, q_index: int) -> None:
        if self.mode == "index" and q_index == 1 and os.path.basename(source) != f"{prefix}.q_1":
            for filename in (f"{prefix}.dvscf1", f"{prefix}.dvscf_paw1", "dvscf"):
                dvscf_source = os.path.join(source, filename)
                if os.path.exists(dvscf_source):
                    _copy_file(dvscf_source, os.path.join(target_root, filename))
            return
        _copy_tree(source, os.path.join(target_root, f"{prefix}.q_{q_index}"))

    def copy_phsave(self, source: str, target_root: str, prefix: str, q_index: int) -> None:
        dst = os.path.join(target_root, f"{prefix}.phsave")
        _copy_tree(source, dst)
        if self.mode == "index":
            _merge_patterns_for_index(source, dst, q_index)


def _merge_split_results(workspace: str, prefix: str, strategy: SplitMergeStrategy) -> MergeResult:
    entries = load_q_index_entries(workspace)
    errors: List[str] = []
    dyn_summaries: List[Tuple[int, List[float]]] = []
    merged_ph_root = _resolve_ph_root(workspace)
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

        dyn_source = strategy.find_dyn_source(q_dir, prefix, q_index_int, errors)
        q_phdir_source = strategy.find_q_phdir_source(q_dir, prefix, q_index_int, errors)
        phsave_source = strategy.find_phsave_source(q_dir, prefix, q_index_int, errors)

        if os.path.exists(split_in):
            _copy_file(split_in, os.path.join(workspace, f"split_ph.in.{q_index_int}"))
        if os.path.exists(split_out):
            _copy_file(split_out, os.path.join(workspace, f"split_ph.out.{q_index_int}"))
        if dyn_source is not None:
            dyn_target = os.path.join(workspace, f"{prefix}.dyn{q_index_int}")
            _copy_file(dyn_source, dyn_target)
            dyn_summaries.append((q_index_int, _extract_lowest_frequencies(dyn_target)))
        if q_phdir_source is not None:
            strategy.copy_q_phdir(q_phdir_source, merged_ph_root, prefix, q_index_int)
        elif strategy.mode == "coords":
            q_ph_root = _resolve_ph_root(q_dir)
            for filename in (f"{prefix}.dvscf1", f"{prefix}.dvscf_paw1", "dvscf"):
                dvscf_source = os.path.join(q_ph_root, filename)
                if os.path.exists(dvscf_source):
                    _copy_file(dvscf_source, os.path.join(merged_ph_root, filename))
        if phsave_source is not None:
            strategy.copy_phsave(phsave_source, merged_ph_root, prefix, q_index_int)

    if errors:
        return MergeResult(False, errors, dyn_summaries)
    return MergeResult(check_merge(workspace, prefix) == "finished", [], dyn_summaries)


def _resolve_ph_root(workdir: str) -> str:
    tmp_ph0 = os.path.join(workdir, "tmp", "_ph0")
    if os.path.isdir(tmp_ph0) or os.path.isdir(os.path.join(workdir, "tmp")):
        return tmp_ph0
    return os.path.join(workdir, "_ph0")


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
            match = re.search(
                r"freq\s*\(\s*\d+\s*\)\s*=\s*([-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?)",
                line,
                flags=re.IGNORECASE,
            )
            if match is not None:
                values.append(float(match.group(1)))
    if not values:
        with open(dyn_path, "r", encoding="utf-8", errors="ignore") as handle:
            for line in handle:
                values.extend(float(item) for item in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?", line))
    return sorted(values)[:limit]
