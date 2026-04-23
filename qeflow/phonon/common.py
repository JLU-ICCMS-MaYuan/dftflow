#!/usr/bin/env python3
import copy
import os
import shutil
import sys
from typing import Dict, Iterable, List, Sequence, Tuple

import numpy as np

from public.config_templates import load_config_file
from public.executables import resolve_qe_command
from public.job import write_job_script
from qeflow.common import get_atomic_mass, get_pseudo_entry
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_phonon_allowed_keys,
    get_qe_phonon_parameters_template,
)

SCF_CONTROL_KEYS = ("calculation", "restart_mode", "prefix", "outdir", "pseudo_dir", "verbosity", "tprnfor", "tstress", "forc_conv_thr", "etot_conv_thr")
SCF_SYSTEM_KEYS = ("ibrav", "ecutwfc", "ecutrho", "occupations", "smearing", "degauss")
SCF_ELECTRON_KEYS = ("conv_thr", "mixing_beta", "electron_maxstep")


def to_qe_value(value):
    if isinstance(value, bool):
        return ".true." if value else ".false."
    if isinstance(value, (int, float)):
        return str(value)
    text = str(value)
    if text.startswith("'") or text.startswith('"'):
        return text
    if text.lower() in {".true.", ".false."}:
        return text.lower()
    if text.lower() in {"true", "false"}:
        return ".true." if text.lower() == "true" else ".false."
    return f"'{text}'"


def get_formula(elements, counts):
    formula = ""
    for el, count in zip(elements, counts):
        formula += el
        if count >= 1:
            formula += str(count)
    return formula


def parse_poscar(poscar_path: str) -> Dict:
    if not os.path.exists(poscar_path):
        raise FileNotFoundError(f"找不到 {poscar_path} 文件")
    with open(poscar_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
    scale = float(lines[1].strip())
    lattice = []
    for i in range(2, 5):
        lattice.append([float(x) * scale for x in lines[i].split()])
    elements = lines[5].split()
    if elements[0].isdigit():
        elements = lines[0].split()
        if not elements:
            raise ValueError("无法在 POSCAR 中找到元素名称。")
    counts = [int(x) for x in lines[6].split()]
    coord_line = lines[7].strip()
    if coord_line.lower().startswith("s"):
        coord_line = lines[8].strip()
        start_line = 9
    else:
        start_line = 8
    positions = []
    total_atoms = sum(counts)
    for i in range(total_atoms):
        positions.append(lines[start_line + i].split()[:3])
    pos_array = np.array(positions, dtype=float)
    if not coord_line.lower().startswith("d"):
        pos_array *= scale
    return {
        "lattice": np.array(lattice),
        "elements": elements,
        "counts": counts,
        "coord_type": coord_line,
        "positions": pos_array,
    }


def get_kpoints(lattice, kmesh):
    if isinstance(kmesh, list) and len(kmesh) == 3:
        return kmesh
    b = 2 * np.pi * np.linalg.inv(lattice).T
    b_lengths = np.linalg.norm(b, axis=1)
    spacing = kmesh if isinstance(kmesh, float) else 0.04
    return [int(max(1, np.ceil(length / (spacing * 2 * np.pi)))) for length in b_lengths]


def load_phonon_config(config_file: str) -> Dict:
    template = get_qe_phonon_parameters_template()
    return apply_qe_config_overrides(template, load_config_file(config_file), get_qe_phonon_allowed_keys())


def resolve_phonon_qpoints(section: Dict, axes_keys: Sequence[str], vector_key: str = "qpoints") -> Tuple[int, int, int]:
    qpoints = section.get(vector_key)
    if isinstance(qpoints, list) and len(qpoints) == 3:
        return int(qpoints[0]), int(qpoints[1]), int(qpoints[2])
    values = [section.get(key) for key in axes_keys]
    if all(value is not None for value in values):
        return int(values[0]), int(values[1]), int(values[2])
    raise ValueError(f"缺少 {vector_key} 配置，且未提供兼容键 {axes_keys}")


def derive_prefix(config: Dict, struct_info: Dict) -> str:
    prefix = config["qe_phonon"].get("prefix")
    return prefix or get_formula(struct_info["elements"], struct_info["counts"])


def derive_phonon_filenames(prefix: str) -> Dict[str, str]:
    return {
        "fildyn": f"{prefix}.dyn",
        "flfrc": f"{prefix}.fc",
        "flfrq": f"{prefix}.phband.freq",
        "fldos": f"{prefix}.phdos.dat",
    }


def prepare_work_tree(work_dir: str, struct_file: str, config_file: str) -> str:
    os.makedirs(work_dir, exist_ok=True)
    target_poscar = os.path.join(work_dir, "POSCAR")
    if os.path.abspath(struct_file) != os.path.abspath(target_poscar):
        shutil.copy2(struct_file, target_poscar)
    target_config = os.path.join(work_dir, "input.toml")
    if os.path.abspath(config_file) != os.path.abspath(target_config):
        shutil.copy2(config_file, target_config)
    return target_poscar


def build_qe_commands(config: Dict) -> Dict[str, str]:
    qe_bin = config.get("qe", {}).get("executable_path")
    return {
        "pw": resolve_qe_command(qe_bin, "pw.x"),
        "ph": resolve_qe_command(qe_bin, "ph.x"),
        "q2r": resolve_qe_command(qe_bin, "q2r.x"),
        "matdyn": resolve_qe_command(qe_bin, "matdyn.x"),
    }


def _write_atomic_species(f, config: Dict, elements: Sequence[str]) -> None:
    pseudo_map = config.get("pseudo_map", {})
    f.write("ATOMIC_SPECIES\n")
    for el in elements:
        info = get_pseudo_entry(pseudo_map, el)
        mass = get_atomic_mass(el, info.get("mass"))
        pseudo = str(info["pseudo"]).strip()
        f.write(f"  {el:3} {mass:8.4f} {pseudo}\n")
    f.write("\n")


def _write_positions_and_cell(f, struct_info: Dict) -> None:
    coord_type = struct_info["coord_type"]
    f.write("ATOMIC_POSITIONS {crystal}\n" if coord_type.lower().startswith("d") else "ATOMIC_POSITIONS {angstrom}\n")
    atom_idx = 0
    for el_idx, el in enumerate(struct_info["elements"]):
        for _ in range(struct_info["counts"][el_idx]):
            pos = struct_info["positions"][atom_idx]
            f.write(f"  {el:3} {pos[0]:21.17f} {pos[1]:21.17f} {pos[2]:21.17f}\n")
            atom_idx += 1
    f.write("\n")
    f.write("CELL_PARAMETERS {angstrom}\n")
    for vec in struct_info["lattice"]:
        f.write(f"  {vec[0]:21.17f} {vec[1]:21.17f} {vec[2]:21.17f}\n")
    f.write("\n")


def write_scf_input(work_dir: str, config: Dict, struct_info: Dict, prefix: str) -> str:
    scf_cfg = copy.deepcopy(config["qe_phonon"]["scf"])
    outdir = scf_cfg.get("outdir", "./out")
    qe_input_path = os.path.join(work_dir, "scf.in")
    with open(qe_input_path, "w", encoding="utf-8") as f:
        f.write("&CONTROL\n")
        for key in SCF_CONTROL_KEYS:
            if key == "prefix":
                value = prefix
            elif key == "pseudo_dir":
                value = config.get("pp_dir")
            else:
                value = scf_cfg.get(key)
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n\n")
        f.write("&SYSTEM\n")
        f.write(f"  nat = {struct_info['positions'].shape[0]}\n")
        f.write(f"  ntyp = {len(struct_info['elements'])}\n")
        f.write("  ibrav = 0\n")
        for key in SCF_SYSTEM_KEYS:
            if key == "ibrav":
                continue
            value = scf_cfg.get(key)
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n\n")
        f.write("&ELECTRONS\n")
        for key in SCF_ELECTRON_KEYS:
            value = scf_cfg.get(key)
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n\n")
        _write_atomic_species(f, config, struct_info["elements"])
        _write_positions_and_cell(f, struct_info)
        kpoints = scf_cfg.get("kpoints")
        final_kmesh = kpoints if isinstance(kpoints, list) and len(kpoints) == 3 else get_kpoints(struct_info["lattice"], scf_cfg.get("kmesh"))
        f.write("K_POINTS automatic\n")
        f.write(f"  {int(final_kmesh[0])} {int(final_kmesh[1])} {int(final_kmesh[2])} 0 0 0\n")
    os.makedirs(os.path.join(work_dir, outdir), exist_ok=True)
    return qe_input_path


def write_ph_input(work_dir: str, config: Dict, prefix: str, filename: str = "ph.in") -> str:
    ph_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    filenames = derive_phonon_filenames(prefix)
    nq1, nq2, nq3 = resolve_phonon_qpoints(ph_cfg, ("nq1", "nq2", "nq3"))
    ph_cfg["nq1"] = nq1
    ph_cfg["nq2"] = nq2
    ph_cfg["nq3"] = nq3
    ph_cfg.pop("qpoints", None)
    ph_cfg["fildyn"] = filenames["fildyn"]
    scf_cfg = config["qe_phonon"]["scf"]
    path = os.path.join(work_dir, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write("&inputph\n")
        f.write(f"  prefix = {to_qe_value(prefix)}\n")
        f.write(f"  outdir = {to_qe_value(scf_cfg.get('outdir', './out'))}\n")
        for key, value in ph_cfg.items():
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n")
    return path


def write_split_ph_input_assignq(work_dir: str, config: Dict, prefix: str, q_index: int) -> str:
    split_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    filenames = derive_phonon_filenames(prefix)
    nq1, nq2, nq3 = resolve_phonon_qpoints(split_cfg, ("nq1", "nq2", "nq3"))
    split_cfg["nq1"] = nq1
    split_cfg["nq2"] = nq2
    split_cfg["nq3"] = nq3
    split_cfg.pop("qpoints", None)
    split_cfg["fildyn"] = filenames["fildyn"]
    split_cfg["ldisp"] = True
    split_cfg["start_q"] = q_index
    split_cfg["last_q"] = q_index
    scf_cfg = config["qe_phonon"]["scf"]
    path = os.path.join(work_dir, "split_ph.in")
    with open(path, "w", encoding="utf-8") as f:
        f.write("&inputph\n")
        f.write(f"  prefix = {to_qe_value(prefix)}\n")
        f.write(f"  outdir = {to_qe_value(scf_cfg.get('outdir', './out'))}\n")
        for key, value in split_cfg.items():
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n")
    return path


def write_split_ph_input_coordinate(work_dir: str, config: Dict, prefix: str, q_cart: Sequence[float]) -> str:
    split_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    filenames = derive_phonon_filenames(prefix)
    nq1, nq2, nq3 = resolve_phonon_qpoints(split_cfg, ("nq1", "nq2", "nq3"))
    split_cfg["nq1"] = nq1
    split_cfg["nq2"] = nq2
    split_cfg["nq3"] = nq3
    split_cfg.pop("qpoints", None)
    split_cfg["fildyn"] = filenames["fildyn"]
    split_cfg["ldisp"] = False
    split_cfg["start_q"] = None
    split_cfg["last_q"] = None
    scf_cfg = config["qe_phonon"]["scf"]
    path = os.path.join(work_dir, "split_ph.in")
    with open(path, "w", encoding="utf-8") as f:
        f.write("&inputph\n")
        f.write(f"  prefix = {to_qe_value(prefix)}\n")
        f.write(f"  outdir = {to_qe_value(scf_cfg.get('outdir', './out'))}\n")
        for key, value in split_cfg.items():
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n")
        f.write(f"{q_cart[0]:24.15E}{q_cart[1]:24.15E}{q_cart[2]:24.15E}\n")
    return path


def write_q2r_input(work_dir: str, config: Dict, prefix: str) -> str:
    q2r_cfg = copy.deepcopy(config["qe_phonon"]["q2r"])
    filenames = derive_phonon_filenames(prefix)
    q2r_cfg["flfrc"] = filenames["flfrc"]
    path = os.path.join(work_dir, "q2r.in")
    with open(path, "w", encoding="utf-8") as f:
        f.write("&input\n")
        f.write(f"  fildyn = {to_qe_value(filenames['fildyn'])}\n")
        for key, value in q2r_cfg.items():
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n")
    return path


def write_matdyn_band_input(work_dir: str, config: Dict, prefix: str, kpoint_lines: Sequence[Tuple[Sequence[float], str, int]]) -> str:
    band_cfg = copy.deepcopy(config["qe_phonon"]["phband"])
    filenames = derive_phonon_filenames(prefix)
    band_cfg["flfrq"] = filenames["flfrq"]
    path = os.path.join(work_dir, "matdyn_band.in")
    with open(path, "w", encoding="utf-8") as f:
        f.write("&input\n")
        f.write(f"  flfrc = {to_qe_value(filenames['flfrc'])}\n")
        for key, value in band_cfg.items():
            if key in {"kpath_file", "kpath_points"}:
                continue
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n")
        f.write(f"{len(kpoint_lines)}\n")
        for coords, _label, weight in kpoint_lines:
            f.write(f"  {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f} {int(weight):6d}\n")
    return path


def write_matdyn_dos_input(work_dir: str, config: Dict, prefix: str) -> str:
    dos_cfg = copy.deepcopy(config["qe_phonon"]["phdos"])
    filenames = derive_phonon_filenames(prefix)
    nk1, nk2, nk3 = resolve_phonon_qpoints(dos_cfg, ("nk1", "nk2", "nk3"))
    dos_cfg["nk1"] = nk1
    dos_cfg["nk2"] = nk2
    dos_cfg["nk3"] = nk3
    dos_cfg.pop("qpoints", None)
    dos_cfg["fldos"] = filenames["fldos"]
    path = os.path.join(work_dir, "matdyn_dos.in")
    with open(path, "w", encoding="utf-8") as f:
        f.write("&input\n")
        f.write(f"  flfrc = {to_qe_value(filenames['flfrc'])}\n")
        for key, value in dos_cfg.items():
            if value is not None:
                f.write(f"  {key} = {to_qe_value(value)}\n")
        f.write("/\n")
    return path


def copy_minimal_save_data(src_save: str, dst_save: str) -> None:
    if not os.path.exists(src_save):
        raise FileNotFoundError(f"找不到 prefix.save: {src_save}")
    os.makedirs(dst_save, exist_ok=True)
    for name in ("data-file-schema.xml", "charge-density.dat", "paw.txt"):
        src = os.path.join(src_save, name)
        if os.path.exists(src):
            shutil.copy2(src, os.path.join(dst_save, name))


def create_job_script(script_path: str, config: Dict, commands: Iterable[str]) -> None:
    slurm_header = config.get("slurm", {}).get("header", "")
    write_job_script(script_path, slurm_header, list(commands))


def write_python_helper_script(script_path: str, body: str) -> None:
    with open(script_path, "w", encoding="utf-8") as f:
        f.write(f"#!{sys.executable}\n")
        f.write(body if body.endswith("\n") else body + "\n")
    os.chmod(script_path, 0o755)
