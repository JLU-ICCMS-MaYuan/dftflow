#!/usr/bin/env python3
import os
from typing import Any, Callable, Dict, Iterable, List, Sequence, Tuple

import numpy as np

from qeflow.common import resolve_atomic_species_entries
from qeflow.crystal.kmesh import get_kpoints_from_spacing
from qeflow.input_deck.lattice import (
    write_atomic_positions,
    write_cell_parameters_if_needed,
    write_system_lattice,
)
from qeflow.input_deck.values import to_qe_value


PW_SECTIONS = ("CONTROL", "SYSTEM", "ELECTRONS")
SCF_CONTROL_KEYS = (
    "calculation",
    "restart_mode",
    "prefix",
    "outdir",
    "pseudo_dir",
    "verbosity",
    "tprnfor",
    "tstress",
    "forc_conv_thr",
    "etot_conv_thr",
)
SCF_SYSTEM_KEYS = ("ecutwfc", "ecutrho", "occupations", "smearing", "degauss")
SCF_ELECTRON_KEYS = ("conv_thr", "mixing_beta", "electron_maxstep")


def write_pw_input(setup: Any, filename: str, kpoints_writer: Callable[[Any, Any], None], precision: int = 17) -> None:
    if setup.struct_info is None:
        raise ValueError("缺少结构信息，无法写 QE 输入。")
    path = os.path.join(setup.work_dir, filename)
    with open(path, "w", encoding="utf-8") as handle:
        for section in PW_SECTIONS:
            handle.write(f"&{section}\n")
            if section == "SYSTEM":
                handle.write(f"  nat  = {setup.struct_info['positions'].shape[0]}\n")
                handle.write(f"  ntyp = {len(setup.struct_info['elements'])}\n")
                write_system_lattice(handle, setup.config, setup.struct_info)
            for key, value in setup.qe_params.get(section, {}).items():
                handle.write(f"  {key} = {value}\n")
            handle.write("/\n\n")
        write_atomic_species(handle, setup.config, setup.struct_info["elements"])
        write_atomic_positions(handle, setup.struct_info, precision=precision)
        write_cell_parameters_if_needed(handle, setup.config, setup.struct_info, precision=precision)
        kpoints_writer(setup, handle)


def write_scf_inputs(setup: Any) -> None:
    print("正在生成 scf.in...")
    write_pw_input(setup, "scf.in", write_scf_kpoints, precision=17)


def write_scf_kpoints(setup: Any, handle: Any) -> None:
    k_points_config = setup.cli_config.get("k_points", {})
    kpoints = k_points_config.get("kpoints")
    if isinstance(kpoints, list) and len(kpoints) == 3:
        kmesh = [int(kpoints[0]), int(kpoints[1]), int(kpoints[2])]
    else:
        kmesh = get_kpoints_from_spacing(setup.struct_info["lattice"], k_points_config.get("kmesh"), 0.04)
    handle.write("K_POINTS automatic\n")
    handle.write(f"  {int(kmesh[0])} {int(kmesh[1])} {int(kmesh[2])} 0 0 0\n")


def write_nscf_inputs(setup: Any) -> None:
    print("正在生成 nscf.in...")
    write_pw_input(setup, "nscf.in", write_nscf_kpoints, precision=8)


def write_nscf_kpoints(setup: Any, handle: Any) -> None:
    k_points_config = setup.cli_config.get("k_points", {})
    kpoints = k_points_config.get("kpoints")
    kmesh = kpoints if kpoints is not None else get_kpoints_from_spacing(
        setup.struct_info["lattice"],
        k_points_config.get("kmesh"),
        0.02,
    )
    if kmesh is None:
        raise ValueError("k_points 需要提供 kmesh 或 kpoints")
    coords, total = uniform_crystal_kpoints(kmesh, bool(k_points_config.get("wan", False)))
    handle.write("K_POINTS crystal\n")
    handle.write(f"{total}\n")
    for line in coords:
        handle.write(f" {line}\n")


def uniform_crystal_kpoints(kpoints: Sequence[int], wan: bool) -> Tuple[List[str], int]:
    n1, n2, n3 = [int(value) for value in kpoints]
    if n1 <= 0 or n2 <= 0 or n3 <= 0:
        raise ValueError("kpoints 中的 n1/n2/n3 必须大于 0")
    total = n1 * n2 * n3
    lines = []
    for x in range(n1):
        for y in range(n2):
            for z in range(n3):
                if wan:
                    lines.append(f"{x/n1:21.17f}{y/n2:21.17f}{z/n3:21.17f}")
                else:
                    lines.append(f"{x/n1:21.17f}{y/n2:21.17f}{z/n3:21.17f}{1/total:14.6e}")
    return lines, total


def write_phonon_scf_input(workspace: str, config: Dict[str, Any], struct_info: Dict[str, Any], prefix: str) -> str:
    scf_cfg = dict(config["qe_phonon"]["scf"])
    path = os.path.join(workspace, "scf.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&CONTROL\n")
        for key in SCF_CONTROL_KEYS:
            if key == "prefix":
                value = prefix
            elif key == "pseudo_dir":
                value = config.get("pp_dir")
            else:
                value = scf_cfg.get(key)
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n\n")
        handle.write("&SYSTEM\n")
        handle.write(f"  nat = {struct_info['positions'].shape[0]}\n")
        handle.write(f"  ntyp = {len(struct_info['elements'])}\n")
        write_system_lattice(handle, config, struct_info)
        for key in SCF_SYSTEM_KEYS:
            value = scf_cfg.get(key)
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n\n")
        handle.write("&ELECTRONS\n")
        for key in SCF_ELECTRON_KEYS:
            value = scf_cfg.get(key)
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n\n")
        write_atomic_species(handle, config, struct_info["elements"])
        write_atomic_positions(handle, struct_info)
        write_cell_parameters_if_needed(handle, config, struct_info)
        kpoints = scf_cfg.get("kpoints")
        final_kmesh = (
            [int(kpoints[0]), int(kpoints[1]), int(kpoints[2])]
            if isinstance(kpoints, list) and len(kpoints) == 3
            else get_kpoints_from_spacing(struct_info["lattice"], scf_cfg.get("kmesh"), default_spacing=0.04)
        )
        handle.write("K_POINTS automatic\n")
        handle.write(f"  {final_kmesh[0]} {final_kmesh[1]} {final_kmesh[2]} 0 0 0\n")
    outdir = str(scf_cfg.get("outdir", "./out")).strip("'").strip('"')
    if outdir:
        os.makedirs(os.path.join(workspace, outdir), exist_ok=True)
    return path


def write_atomic_species(handle: Any, config: Dict[str, Any], elements: Iterable[str]) -> None:
    handle.write("ATOMIC_SPECIES\n")
    for element, mass, pseudo_file in resolve_atomic_species_entries(config, list(elements)):
        handle.write(f"  {element:3} {mass:8.4f} {pseudo_file}\n")
    handle.write("\n")
