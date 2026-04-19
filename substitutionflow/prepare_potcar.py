#!/usr/bin/env python3
import argparse
import os
import re
import shutil
from collections import OrderedDict
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

from substitutionflow.selection import ELEMENT_SYMBOLS, selection_to_symbols, symbol_to_atomic_number
from substitutionflow.structure_substitution import (
    load_substitutionflow_config,
    read_poscar,
    resolve_input_structure_path,
)


LOG_FILENAME = "potcar_lib.log"
POTCAR_LIB_DIRNAME = "potcar_lib"
ORBITAL_LABELS = {0: "s", 1: "p", 2: "d", 3: "f", 4: "g", 5: "h"}


@dataclass
class PotcarCandidate:
    element: str
    candidate_name: str
    potcar_path: str
    zval: float
    config_text: str


def _format_occ(value: float) -> str:
    if abs(value - round(value)) < 1e-6:
        return str(int(round(value)))
    return f"{value:g}"


def _format_zval(value: float) -> str:
    return _format_occ(value)


def _orbital_label(l_value: int) -> str:
    return ORBITAL_LABELS.get(l_value, f"l{l_value}")


def _candidate_sort_key(candidate: PotcarCandidate) -> Tuple[int, str]:
    exact = 0 if candidate.candidate_name == candidate.element else 1
    return (exact, candidate.candidate_name)


def parse_potcar_candidate(potcar_path: str, element: str, candidate_name: str) -> PotcarCandidate:
    with open(potcar_path, "r", encoding="utf-8", errors="ignore") as f:
        lines = [line.rstrip("\n") for line in f]

    if len(lines) < 2:
        raise ValueError(f"POTCAR 内容不完整: {potcar_path}")

    zval_match = re.search(r"ZVAL\s*=\s*([0-9.]+)", "\n".join(lines))
    if zval_match:
        zval = float(zval_match.group(1))
    else:
        try:
            zval = float(lines[1].strip())
        except ValueError as exc:
            raise ValueError(f"无法从 POTCAR 中解析 ZVAL: {potcar_path}") from exc

    config_text = extract_valence_configuration(lines, zval)
    return PotcarCandidate(
        element=element,
        candidate_name=candidate_name,
        potcar_path=potcar_path,
        zval=zval,
        config_text=config_text,
    )


def extract_valence_configuration(lines: List[str], zval: float) -> str:
    try:
        start_index = next(i for i, line in enumerate(lines) if line.strip() == "Atomic configuration")
    except StopIteration:
        return "unknown"

    if start_index + 2 >= len(lines):
        return "unknown"

    entry_count_match = re.search(r"(\d+)\s+entries", lines[start_index + 1])
    if not entry_count_match:
        return "unknown"
    entry_count = int(entry_count_match.group(1))

    entries = []
    for line in lines[start_index + 2:]:
        parts = line.split()
        if len(parts) < 5:
            continue
        try:
            n_value = int(parts[0])
            l_value = int(parts[1])
            occ_value = float(parts[4])
        except ValueError:
            continue
        entries.append((n_value, l_value, occ_value))
        if len(entries) >= entry_count:
            break

    if not entries:
        return "unknown"

    remaining = zval
    valence_entries = []
    for n_value, l_value, occ_value in reversed(entries):
        if remaining <= 1e-8:
            break
        selected_occ = min(occ_value, remaining)
        if selected_occ > 1e-8:
            valence_entries.append((n_value, l_value, selected_occ))
            remaining -= selected_occ

    valence_entries.sort(key=lambda item: (item[0], item[1]))
    return " ".join(
        f"{n_value}{_orbital_label(l_value)}{_format_occ(occ_value)}"
        for n_value, l_value, occ_value in valence_entries
    ) or "unknown"


def discover_required_elements(config_file: str) -> List[str]:
    config = load_substitutionflow_config(config_file)
    ht_config = config["substitutionflow"]
    structure_path = resolve_input_structure_path(config_file)
    poscar = read_poscar(structure_path)

    symbols = set()
    for element in poscar.elements:
        if element == "XA":
            symbols.update(selection_to_symbols(ht_config["XA"]))
        elif element == "XB":
            symbols.update(selection_to_symbols(ht_config["XB"]))
        else:
            symbols.add(element)

    return sorted(symbols, key=symbol_to_atomic_number)


def discover_candidate_groups(pot_root: str, required_elements: List[str]) -> Dict[str, List[PotcarCandidate]]:
    pot_root = os.path.abspath(os.path.expanduser(pot_root))
    if not os.path.isdir(pot_root):
        raise FileNotFoundError(f"赝势目录不存在: {pot_root}")

    valid_symbols = {symbol for symbol in ELEMENT_SYMBOLS if symbol}
    required_set = set(required_elements)
    grouped: Dict[str, List[PotcarCandidate]] = {element: [] for element in required_elements}

    for entry_name in sorted(os.listdir(pot_root)):
        entry_path = os.path.join(pot_root, entry_name)
        if not os.path.isdir(entry_path):
            continue
        base_symbol = entry_name.split("_", 1)[0]
        if base_symbol not in valid_symbols or base_symbol not in required_set:
            continue
        potcar_path = os.path.join(entry_path, "POTCAR")
        if not os.path.isfile(potcar_path):
            continue
        grouped[base_symbol].append(parse_potcar_candidate(potcar_path, base_symbol, entry_name))

    for element, candidates in grouped.items():
        if not candidates:
            raise FileNotFoundError(f"在 {pot_root} 中找不到元素 {element} 的 POTCAR 候选")
        grouped[element] = sorted(candidates, key=_candidate_sort_key)
    return grouped


def parse_selection_file(selection_file: str) -> Dict[str, str]:
    if not os.path.exists(selection_file):
        raise FileNotFoundError(f"找不到赝势选择文件: {selection_file}")

    selections: Dict[str, str] = {}
    with open(selection_file, "r", encoding="utf-8") as f:
        for raw_line in f:
            line = raw_line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" not in line:
                raise ValueError(f"赝势选择文件格式错误: {raw_line.rstrip()}")
            element, candidate = [item.strip() for item in line.split("=", 1)]
            if not element or not candidate:
                raise ValueError(f"赝势选择文件格式错误: {raw_line.rstrip()}")
            selections[element] = candidate
    return selections


def get_log_path(output_dir: str) -> str:
    potcar_lib_dir = os.path.join(output_dir, POTCAR_LIB_DIRNAME)
    return os.path.join(potcar_lib_dir, LOG_FILENAME)


def write_selection_log(selected: Dict[str, PotcarCandidate], output_dir: str) -> str:
    potcar_lib_dir = os.path.join(output_dir, POTCAR_LIB_DIRNAME)
    os.makedirs(potcar_lib_dir, exist_ok=True)
    log_path = get_log_path(output_dir)
    with open(log_path, "w", encoding="utf-8") as f:
        for element in sorted(selected, key=symbol_to_atomic_number):
            f.write(f"{element} = {selected[element].candidate_name}\n")
    return log_path


def append_selected_candidate(selected: Dict[str, PotcarCandidate], candidate: PotcarCandidate, output_dir: str) -> str:
    selected[candidate.element] = candidate
    return write_selection_log(selected, output_dir)


def copy_selected_potcars(selected: Dict[str, PotcarCandidate], output_dir: str) -> str:
    potcar_lib_dir = os.path.join(output_dir, POTCAR_LIB_DIRNAME)
    os.makedirs(potcar_lib_dir, exist_ok=True)

    for element, candidate in selected.items():
        shutil.copyfile(candidate.potcar_path, os.path.join(potcar_lib_dir, element))

    return get_log_path(output_dir)


def load_existing_selections(output_dir: str) -> Dict[str, str]:
    log_path = get_log_path(output_dir)
    if not os.path.exists(log_path):
        print(f"找不到赝势选择记录文件: {log_path}")
        return {}
    print(f"读取已有赝势选择记录: {log_path}")
    return parse_selection_file(log_path)


def select_candidates(
    candidate_groups: Dict[str, List[PotcarCandidate]],
    output_dir: str,
    preselected: Optional[Dict[str, str]] = None,
) -> Dict[str, PotcarCandidate]:
    selected: Dict[str, PotcarCandidate] = OrderedDict()
    preselected = preselected or {}

    for element in sorted(candidate_groups, key=symbol_to_atomic_number):
        candidates = candidate_groups[element]
        preselected_name = preselected.get(element)
        if preselected_name is not None:
            matched = next((candidate for candidate in candidates if candidate.candidate_name == preselected_name), None)
            if matched is None:
                raise ValueError(f"元素 {element} 的已选赝势 {preselected_name} 在当前 --pot 目录中不存在。")
            print(f"元素 {element} 使用记录文件中的赝势: {matched.candidate_name}")
            append_selected_candidate(selected, matched, output_dir)
            continue

        if len(candidates) == 1:
            print(f"元素 {element} 只有一个赝势候选，自动选择: {candidates[0].candidate_name}")
            append_selected_candidate(selected, candidates[0], output_dir)
            continue

        print(f"元素 {element} 检测到 {len(candidates)} 个赝势候选：")
        for index, candidate in enumerate(candidates, start=1):
            print(
                f"No.{index}: {candidate.candidate_name}: "
                f"{_format_zval(candidate.zval)}, {candidate.config_text}"
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
                    append_selected_candidate(selected, candidates[chosen_index - 1], output_dir)
                    break
            print("输入无效，请重新输入候选编号。")

    return selected


def prepare_potcar_library(config_file: str, pot_root: str) -> str:
    output_dir = os.getcwd()
    required_elements = discover_required_elements(config_file)
    print("需要准备的元素:", " ".join(required_elements))

    candidate_groups = discover_candidate_groups(pot_root, required_elements)
    preselected = load_existing_selections(output_dir)
    selected = select_candidates(candidate_groups, output_dir, preselected=preselected)
    log_path = copy_selected_potcars(selected, output_dir)
    print(f"已生成 {os.path.join(output_dir, POTCAR_LIB_DIRNAME)}")
    print(f"已写入赝势选择记录: {log_path}")
    return log_path


def main():
    parser = argparse.ArgumentParser(description="为 substitutionflow 准备本地 potcar_lib。")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径。")
    parser.add_argument("--pot", required=True, help="原始赝势库目录。")
    args = parser.parse_args()

    prepare_potcar_library(args.config, args.pot)


if __name__ == "__main__":
    main()
