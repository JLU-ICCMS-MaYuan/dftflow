#!/usr/bin/env python3
import json
import os
from math import gcd
from collections import OrderedDict
from itertools import product
from typing import Dict, List

from public.config_templates import load_config_file

from substitutionflow.selection import (
    atomic_number_to_period,
    parse_atomic_selection,
    selection_to_symbols,
    symbol_to_atomic_number,
)


class PoscarData:
    def __init__(
        self,
        comment: str,
        scale_line: str,
        lattice_lines: List[str],
        elements: List[str],
        counts: List[int],
        selective_line: str,
        coordinate_mode: str,
        coordinate_lines: List[str],
        tail_lines: List[str],
    ):
        self.comment = comment
        self.scale_line = scale_line
        self.lattice_lines = lattice_lines
        self.elements = elements
        self.counts = counts
        self.selective_line = selective_line
        self.coordinate_mode = coordinate_mode
        self.coordinate_lines = coordinate_lines
        self.tail_lines = tail_lines


def read_poscar(path: str) -> PoscarData:
    with open(path, "r", encoding="utf-8") as f:
        lines = [line.rstrip("\n") for line in f]

    if len(lines) < 8:
        raise ValueError(f"POSCAR 内容不完整: {path}")

    elements = lines[5].split()
    counts = [int(value) for value in lines[6].split()]
    if len(elements) != len(counts):
        raise ValueError("POSCAR 元素与计数列不匹配。")

    next_index = 7
    selective_line = ""
    if lines[next_index].strip().lower().startswith("selective"):
        selective_line = lines[next_index]
        next_index += 1

    coordinate_mode = lines[next_index]
    next_index += 1
    total_atoms = sum(counts)
    coordinate_lines = lines[next_index: next_index + total_atoms]
    if len(coordinate_lines) != total_atoms:
        raise ValueError("POSCAR 原子坐标数不足。")
    tail_lines = lines[next_index + total_atoms:]
    return PoscarData(
        comment=lines[0],
        scale_line=lines[1],
        lattice_lines=lines[2:5],
        elements=elements,
        counts=counts,
        selective_line=selective_line,
        coordinate_mode=coordinate_mode,
        coordinate_lines=coordinate_lines,
        tail_lines=tail_lines,
    )


def build_formula(elements: List[str], counts: List[int]) -> str:
    common_divisor = 0
    for count in counts:
        common_divisor = count if common_divisor == 0 else gcd(common_divisor, count)
    normalized_counts = [count // common_divisor for count in counts] if common_divisor > 1 else counts
    parts = []
    for element, count in zip(elements, normalized_counts):
        parts.append(f"{element}{'' if count == 1 else count}")
    return "".join(parts)


def substitute_poscar(data: PoscarData, replacements: Dict[str, str]) -> PoscarData:
    grouped_lines = []
    start = 0
    for count in data.counts:
        grouped_lines.append(data.coordinate_lines[start:start + count])
        start += count

    new_groups: "OrderedDict[str, List[str]]" = OrderedDict()
    for element, count, coords in zip(data.elements, data.counts, grouped_lines):
        symbol = replacements.get(element, element)
        new_groups.setdefault(symbol, [])
        new_groups[symbol].extend(coords)

    new_elements = list(new_groups.keys())
    new_counts = [len(new_groups[element]) for element in new_elements]
    new_coordinate_lines = []
    for element in new_elements:
        new_coordinate_lines.extend(new_groups[element])

    formula = build_formula(new_elements, new_counts)
    return PoscarData(
        comment=formula,
        scale_line=data.scale_line,
        lattice_lines=data.lattice_lines,
        elements=new_elements,
        counts=new_counts,
        selective_line=data.selective_line,
        coordinate_mode=data.coordinate_mode,
        coordinate_lines=new_coordinate_lines,
        tail_lines=data.tail_lines,
    )


def write_poscar(path: str, data: PoscarData) -> None:
    lines = [
        data.comment,
        data.scale_line,
        *data.lattice_lines,
        "  " + "  ".join(data.elements),
        "  " + "  ".join(str(count) for count in data.counts),
    ]
    if data.selective_line:
        lines.append(data.selective_line)
    lines.append(data.coordinate_mode)
    lines.extend(data.coordinate_lines)
    lines.extend(data.tail_lines)
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def dump_toml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if value is None:
        return json.dumps("")
    if isinstance(value, list):
        return "[" + ", ".join(dump_toml_value(item) for item in value) + "]"
    return str(value)


def dump_toml(data: Dict, prefix: str = "") -> str:
    lines = []
    scalar_keys = []
    dict_keys = []
    for key, value in data.items():
        if isinstance(value, dict):
            dict_keys.append((key, value))
        else:
            scalar_keys.append((key, value))
    for key, value in scalar_keys:
        lines.append(f"{key} = {dump_toml_value(value)}")
    for key, value in dict_keys:
        section = f"{prefix}.{key}" if prefix else key
        if lines:
            lines.append("")
        lines.append(f"[{section}]")
        nested = dump_toml(value, section)
        if nested:
            lines.append(nested)
    return "\n".join(lines)


def load_substitutionflow_config(config_file: str) -> Dict:
    config = load_config_file(config_file)
    if "substitutionflow" not in config:
        raise ValueError("配置缺少 [substitutionflow] 块。")
    return config


def resolve_input_structure_path(config_file: str) -> str:
    config = load_substitutionflow_config(config_file)
    ht_config = config["substitutionflow"]
    structure_filename = ht_config.get("structure_filename", "structure.vasp")
    config_dir = os.path.dirname(os.path.abspath(config_file))
    structure_path = os.path.join(config_dir, structure_filename)
    if not os.path.exists(structure_path):
        raise FileNotFoundError(f"找不到结构文件: {structure_path}")
    return structure_path


def validate_placeholder_elements(elements: List[str]) -> None:
    required = {"XA", "XB"}
    missing = required.difference(elements)
    if missing:
        raise ValueError(f"POSCAR 必须包含占位元素 XA/XB，缺少: {', '.join(sorted(missing))}")


def create_case_input_config(root_config: Dict) -> Dict:
    config = json.loads(json.dumps(root_config))
    config["chgcar_path"] = "./vasp_scf/CHGCAR"
    config["wavecar_path"] = "./vasp_scf/WAVECAR"
    return config


def build_case_relative_path(xa_symbol: str, xb_symbol: str, formula: str) -> str:
    xa_z = symbol_to_atomic_number(xa_symbol)
    xb_z = symbol_to_atomic_number(xb_symbol)
    xb_period = atomic_number_to_period(xb_z)
    return os.path.join(
        f"{xa_z:02d}.XA_{xa_symbol}",
        f"period_{xb_period}",
        f"{xb_z:02d}.{formula}",
    )


def generate_cases(input_path: str, config_file: str, output_dir: str) -> Dict:
    root_config = load_substitutionflow_config(config_file)
    ht_config = root_config["substitutionflow"]
    xa_symbols = selection_to_symbols(ht_config["XA"])
    xb_symbols = selection_to_symbols(ht_config["XB"])
    pressures = list(ht_config.get("pressures", [20, 50, 100]))
    structure_filename = ht_config.get("structure_filename", "structure.vasp")

    poscar = read_poscar(input_path)
    validate_placeholder_elements(poscar.elements)
    os.makedirs(output_dir, exist_ok=True)

    cases = OrderedDict()
    period_manifests: "OrderedDict[str, Dict]" = OrderedDict()
    for xa_symbol, xb_symbol in product(xa_symbols, xb_symbols):
        replaced = substitute_poscar(poscar, {"XA": xa_symbol, "XB": xb_symbol})
        formula = build_formula(replaced.elements, replaced.counts)
        case_rel = build_case_relative_path(xa_symbol, xb_symbol, formula)
        case_dir = os.path.join(output_dir, case_rel)
        os.makedirs(case_dir, exist_ok=True)

        structure_path = os.path.join(case_dir, structure_filename)
        write_poscar(structure_path, replaced)

        case_input = create_case_input_config(root_config)
        with open(os.path.join(case_dir, "input.toml"), "w", encoding="utf-8") as f:
            f.write(dump_toml(case_input).strip() + "\n")

        case_info = {
            "formula": formula,
            "xa_symbol": xa_symbol,
            "xb_symbol": xb_symbol,
            "pressures": pressures,
            "case_rel": case_rel,
            "case_dirname": os.path.basename(case_dir),
            "period_rel": os.path.dirname(case_rel),
        }
        period_rel = os.path.dirname(case_rel)
        period_entry = period_manifests.setdefault(
            period_rel,
            {
                "input_structure": os.path.abspath(input_path),
                "xa_symbol": xa_symbol,
                "pressures": pressures,
                "cases": [],
            },
        )
        period_entry["cases"].append(
            {
                "formula": formula,
                "xb_symbol": xb_symbol,
                "case_dir": os.path.basename(case_dir),
            }
        )
        cases[case_rel] = case_info

    for period_rel, manifest in period_manifests.items():
        manifest_path = os.path.join(output_dir, period_rel, "workflow_manifest.json")
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2, ensure_ascii=False)
    return {
        "root_config": root_config,
        "cases": cases,
        "pressures": pressures,
        "structure_filename": structure_filename,
    }
