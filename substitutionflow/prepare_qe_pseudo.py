#!/usr/bin/env python3
import argparse
import os
import re
import shutil
from collections import OrderedDict
from dataclasses import dataclass
from typing import Dict, List, Optional

from public.config_templates import load_config_file
from qeflow.common import extract_pseudo_element_token
from substitutionflow.prepare_potcar import (
    discover_required_elements,
    parse_selection_file,
)
from substitutionflow.selection import symbol_to_atomic_number
from substitutionflow.structure_substitution import dump_toml


QE_PSEUDO_LIB_DIRNAME = "qe_pseudo_lib"
QE_LOG_FILENAME = "qe_pseudo_lib.log"


@dataclass
class QePseudoCandidate:
    element: str
    filename: str
    source_path: str
    z_valence: Optional[float]
    config_text: str


def _format_float(value: Optional[float]) -> str:
    if value is None:
        return "unknown"
    if abs(value - round(value)) < 1e-6:
        return str(int(round(value)))
    return f"{value:g}"


def parse_qe_pseudo_candidate(source_path: str, element: str) -> QePseudoCandidate:
    with open(source_path, "r", encoding="utf-8", errors="ignore") as f:
        content = f.read()

    z_valence = _extract_z_valence(content)
    config_text = _extract_valence_configuration(content)
    return QePseudoCandidate(
        element=element,
        filename=os.path.basename(source_path),
        source_path=source_path,
        z_valence=z_valence,
        config_text=config_text,
    )


def _extract_z_valence(content: str) -> Optional[float]:
    patterns = (
        r"\bz_valence\b\s*=\s*[\"']?\s*([0-9.+\-EeDd]+)",
        r"\bZ_VALENCE\b\s*=\s*[\"']?\s*([0-9.+\-EeDd]+)",
        r"<z_valence>\s*([0-9.+\-EeDd]+)\s*</z_valence>",
    )
    for pattern in patterns:
        match = re.search(pattern, content, flags=re.IGNORECASE)
        if not match:
            continue
        value_text = match.group(1).replace("D", "E").replace("d", "e")
        try:
            return float(value_text)
        except ValueError:
            continue
    return None


def _extract_valence_configuration(content: str) -> str:
    patterns = (
        r"\b(?:valence_configuration|electron_configuration|configuration)\b\s*=\s*[\"']([^\"']+)[\"']",
        r"<(?:valence_configuration|electron_configuration|configuration)>\s*([^<]+?)\s*</(?:valence_configuration|electron_configuration|configuration)>",
    )
    for pattern in patterns:
        match = re.search(pattern, content, flags=re.IGNORECASE)
        if match:
            text = " ".join(match.group(1).split())
            if text:
                return text
    return "unknown"


def discover_qe_candidate_groups(pseudo_root: str, required_elements: List[str]) -> Dict[str, List[QePseudoCandidate]]:
    pseudo_root = os.path.abspath(os.path.expanduser(pseudo_root))
    if not os.path.isdir(pseudo_root):
        raise FileNotFoundError(f"QE 赝势目录不存在: {pseudo_root}")

    required_set = set(required_elements)
    groups: Dict[str, List[QePseudoCandidate]] = {element: [] for element in required_elements}
    for entry in sorted(os.scandir(pseudo_root), key=lambda item: item.name):
        if not entry.is_file():
            continue
        token = extract_pseudo_element_token(entry.name)
        if not token:
            continue
        matched_element = next((element for element in required_elements if element.lower() == token.lower()), None)
        if matched_element is None or matched_element not in required_set:
            continue
        groups[matched_element].append(parse_qe_pseudo_candidate(entry.path, matched_element))

    for element, candidates in groups.items():
        if not candidates:
            raise FileNotFoundError(f"在 {pseudo_root} 中找不到元素 {element} 对应的 QE 赝势文件")
        groups[element] = sorted(candidates, key=lambda candidate: candidate.filename)
    return groups


def get_qe_pseudo_lib_dir(output_dir: str) -> str:
    return os.path.join(output_dir, QE_PSEUDO_LIB_DIRNAME)


def get_qe_log_path(output_dir: str) -> str:
    return os.path.join(get_qe_pseudo_lib_dir(output_dir), QE_LOG_FILENAME)


def write_qe_selection_log(selected: Dict[str, QePseudoCandidate], output_dir: str) -> str:
    os.makedirs(get_qe_pseudo_lib_dir(output_dir), exist_ok=True)
    log_path = get_qe_log_path(output_dir)
    with open(log_path, "w", encoding="utf-8") as f:
        for element in sorted(selected, key=symbol_to_atomic_number):
            f.write(f"{element} = {selected[element].filename}\n")
    return log_path


def append_selected_qe_candidate(
    selected: Dict[str, QePseudoCandidate],
    candidate: QePseudoCandidate,
    output_dir: str,
) -> str:
    selected[candidate.element] = candidate
    return write_qe_selection_log(selected, output_dir)


def load_existing_qe_selections(output_dir: str) -> Dict[str, str]:
    log_path = get_qe_log_path(output_dir)
    if not os.path.exists(log_path):
        print(f"找不到 QE 赝势选择记录文件: {log_path}")
        return {}
    print(f"读取已有 QE 赝势选择记录: {log_path}")
    return parse_selection_file(log_path)


def select_qe_candidates(
    candidate_groups: Dict[str, List[QePseudoCandidate]],
    output_dir: str,
    preselected: Optional[Dict[str, str]] = None,
) -> Dict[str, QePseudoCandidate]:
    selected: Dict[str, QePseudoCandidate] = OrderedDict()
    preselected = preselected or {}

    for element in sorted(candidate_groups, key=symbol_to_atomic_number):
        candidates = candidate_groups[element]
        preselected_name = preselected.get(element)
        if preselected_name is not None:
            matched = next((candidate for candidate in candidates if candidate.filename == preselected_name), None)
            if matched is None:
                raise ValueError(f"元素 {element} 的已选 QE 赝势 {preselected_name} 在当前 --pp 目录中不存在。")
            print(f"元素 {element} 使用记录文件中的 QE 赝势: {matched.filename}")
            append_selected_qe_candidate(selected, matched, output_dir)
            continue

        if len(candidates) == 1:
            print(f"元素 {element} 只有一个 QE 赝势候选，自动选择: {candidates[0].filename}")
            append_selected_qe_candidate(selected, candidates[0], output_dir)
            continue

        print(f"元素 {element} 检测到 {len(candidates)} 个 QE 赝势候选：")
        for index, candidate in enumerate(candidates, start=1):
            print(
                f"No.{index}: {candidate.filename}: "
                f"{_format_float(candidate.z_valence)}, {candidate.config_text}"
            )

        while True:
            try:
                answer = input("请输入编号: ").strip()
            except KeyboardInterrupt:
                print()
                raise
            if answer.isdigit():
                chosen_index = int(answer)
                if 1 <= chosen_index <= len(candidates):
                    append_selected_qe_candidate(selected, candidates[chosen_index - 1], output_dir)
                    break
            print("输入无效，请重新输入候选编号。")

    return selected


def copy_selected_qe_pseudos(selected: Dict[str, QePseudoCandidate], output_dir: str) -> str:
    target_dir = get_qe_pseudo_lib_dir(output_dir)
    os.makedirs(target_dir, exist_ok=True)
    for candidate in selected.values():
        shutil.copyfile(candidate.source_path, os.path.join(target_dir, candidate.filename))
    return get_qe_log_path(output_dir)


def update_case_input_pp_dir(case_input_path: str, output_dir: str) -> None:
    shared_dir = get_qe_pseudo_lib_dir(os.path.abspath(output_dir))
    config = load_config_file(case_input_path)
    config["pp_dir"] = shared_dir
    with open(case_input_path, "w", encoding="utf-8") as f:
        f.write(dump_toml(config).strip() + "\n")


def update_case_inputs_with_qe_pp_dir(output_dir: str) -> None:
    output_dir = os.path.abspath(output_dir)
    pattern = os.path.join(output_dir, "*", "period_*", "*", "input.toml")
    for case_input_path in sorted(path for path in glob_paths(pattern)):
        update_case_input_pp_dir(case_input_path, output_dir)


def glob_paths(pattern: str) -> List[str]:
    import glob

    return glob.glob(pattern)


def prepare_qe_pseudo_library(config_file: str, output_dir: str, pseudo_root: str) -> str:
    output_dir = os.path.abspath(output_dir)
    if not os.path.isdir(output_dir):
        raise FileNotFoundError(f"找不到 substitution 输出目录: {output_dir}")

    required_elements = discover_required_elements(config_file)
    print("需要准备的 QE 元素:", " ".join(required_elements))

    candidate_groups = discover_qe_candidate_groups(pseudo_root, required_elements)
    preselected = load_existing_qe_selections(output_dir)
    selected = select_qe_candidates(candidate_groups, output_dir, preselected=preselected)
    log_path = copy_selected_qe_pseudos(selected, output_dir)
    update_case_inputs_with_qe_pp_dir(output_dir)
    print(f"已生成 {get_qe_pseudo_lib_dir(output_dir)}")
    print(f"已写入 QE 赝势选择记录: {log_path}")
    return log_path


def main():
    parser = argparse.ArgumentParser(description="为 substitutionflow 准备本地 qe_pseudo_lib。")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径。")
    parser.add_argument("-o", "--output-dir", required=True, help="substitution 输出根目录。")
    parser.add_argument("--pp", "--pseudo-dir", dest="pseudo_dir", required=True, help="原始 QE 赝势目录。")
    args = parser.parse_args()

    prepare_qe_pseudo_library(args.config, args.output_dir, args.pseudo_dir)


if __name__ == "__main__":
    main()
