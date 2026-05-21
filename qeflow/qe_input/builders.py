#!/usr/bin/env python3
import copy
import os
from typing import Any, Dict, Iterable, List, Sequence, Tuple

import numpy as np

from public.executables import resolve_qe_command
from public.job import write_job_script
from qeflow.common import resolve_atomic_species_entries
from qeflow.qe_input.fileio import write_text
from qeflow.qe_input.kpath import build_matdyn_band_path, run_vaspkit_for_kpath


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
SCF_SYSTEM_KEYS = ("ibrav", "ecutwfc", "ecutrho", "occupations", "smearing", "degauss")
SCF_ELECTRON_KEYS = ("conv_thr", "mixing_beta", "electron_maxstep")


def to_qe_value(value: Any) -> str:
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


def get_formula(elements: Sequence[str], counts: Sequence[int]) -> str:
    formula = ""
    for element, count in zip(elements, counts):
        formula += str(element)
        if int(count) >= 1:
            formula += str(int(count))
    return formula


def parse_poscar(poscar_path: str) -> Dict[str, Any]:
    if not os.path.exists(poscar_path):
        raise FileNotFoundError(f"找不到 {poscar_path} 文件")
    with open(poscar_path, "r", encoding="utf-8") as handle:
        lines = handle.readlines()
    scale = float(lines[1].strip())
    lattice = []
    for index in range(2, 5):
        lattice.append([float(item) * scale for item in lines[index].split()])
    elements = lines[5].split()
    if elements[0].isdigit():
        elements = lines[0].split()
        if not elements:
            raise ValueError("无法在 POSCAR 中找到元素名称。")
    counts = [int(item) for item in lines[6].split()]
    coord_line = lines[7].strip()
    if coord_line.lower().startswith("s"):
        coord_line = lines[8].strip()
        start_line = 9
    else:
        start_line = 8
    total_atoms = sum(counts)
    positions = []
    for index in range(total_atoms):
        positions.append(lines[start_line + index].split()[:3])
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


def get_kpoints(lattice: np.ndarray, kmesh: Any) -> List[int]:
    if isinstance(kmesh, list) and len(kmesh) == 3:
        return [int(kmesh[0]), int(kmesh[1]), int(kmesh[2])]
    reciprocal = 2 * np.pi * np.linalg.inv(lattice).T
    lengths = np.linalg.norm(reciprocal, axis=1)
    spacing = float(kmesh) if isinstance(kmesh, (int, float)) else 0.04
    return [int(max(1, np.ceil(length / (spacing * 2 * np.pi)))) for length in lengths]


def resolve_prefix(config: Dict[str, Any], struct_info: Dict[str, Any]) -> str:
    prefix = config["qe_phonon"].get("prefix")
    return str(prefix) if prefix else get_formula(struct_info["elements"], struct_info["counts"])


def derive_filenames(prefix: str) -> Dict[str, str]:
    return {
        "fildyn": f"{prefix}.dyn",
        "dyn0": f"{prefix}.dyn0",
        "flfrc": f"{prefix}.fc",
        "flfrq": f"{prefix}.phband.freq",
        "fldos": f"{prefix}.phdos.dat",
    }


def build_qe_commands(config: Dict[str, Any]) -> Dict[str, str]:
    base = config.get("qe", {}).get("executable_path")
    return {
        "pw": resolve_qe_command(base, "pw.x"),
        "ph": resolve_qe_command(base, "ph.x"),
        "q2r": resolve_qe_command(base, "q2r.x"),
        "matdyn": resolve_qe_command(base, "matdyn.x"),
    }


def _write_atomic_species(handle, config: Dict[str, Any], elements: Iterable[str]) -> None:
    handle.write("ATOMIC_SPECIES\n")
    for element, mass, pseudo_file in resolve_atomic_species_entries(config, list(elements)):
        handle.write(f"  {element:3} {mass:8.4f} {pseudo_file}\n")
    handle.write("\n")


def _write_positions_and_cell(handle, struct_info: Dict[str, Any]) -> None:
    coord_type = str(struct_info["coord_type"]).lower()
    if coord_type.startswith("d"):
        handle.write("ATOMIC_POSITIONS {crystal}\n")
    else:
        handle.write("ATOMIC_POSITIONS {angstrom}\n")
    atom_index = 0
    for element_index, element in enumerate(struct_info["elements"]):
        for _ in range(struct_info["counts"][element_index]):
            pos = struct_info["positions"][atom_index]
            handle.write(
                f"  {element:3} {pos[0]:21.17f} {pos[1]:21.17f} {pos[2]:21.17f}\n"
            )
            atom_index += 1
    handle.write("\n")
    handle.write("CELL_PARAMETERS {angstrom}\n")
    for vec in struct_info["lattice"]:
        handle.write(f"  {vec[0]:21.17f} {vec[1]:21.17f} {vec[2]:21.17f}\n")
    handle.write("\n")


def write_scf_input(workspace: str, config: Dict[str, Any], struct_info: Dict[str, Any], prefix: str) -> str:
    scf_cfg = copy.deepcopy(config["qe_phonon"]["scf"])
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
        handle.write("  ibrav = 0\n")
        for key in SCF_SYSTEM_KEYS:
            if key == "ibrav":
                continue
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
        _write_atomic_species(handle, config, struct_info["elements"])
        _write_positions_and_cell(handle, struct_info)
        kpoints = scf_cfg.get("kpoints")
        final_kmesh = (
            [int(kpoints[0]), int(kpoints[1]), int(kpoints[2])]
            if isinstance(kpoints, list) and len(kpoints) == 3
            else get_kpoints(struct_info["lattice"], scf_cfg.get("kmesh"))
        )
        handle.write("K_POINTS automatic\n")
        handle.write(
            f"  {final_kmesh[0]} {final_kmesh[1]} {final_kmesh[2]} 0 0 0\n"
        )
    outdir = str(scf_cfg.get("outdir", "./out")).strip("'").strip('"')
    if outdir:
        os.makedirs(os.path.join(workspace, outdir), exist_ok=True)
    return path


def _resolve_grid(
    section: Dict[str, Any],
    vector_key: str,
    fallback_keys: Tuple[str, str, str],
) -> Tuple[int, int, int]:
    vector = section.get(vector_key)
    if isinstance(vector, list) and len(vector) == 3:
        return int(vector[0]), int(vector[1]), int(vector[2])
    values = [section.get(key) for key in fallback_keys]
    if all(value is not None for value in values):
        return int(values[0]), int(values[1]), int(values[2])
    raise ValueError(f"缺少 {vector_key} 配置")


def write_ph_input(
    workspace: str,
    config: Dict[str, Any],
    prefix: str,
    filename: str = "ph.in",
    dyn0_only: bool = False,
) -> str:
    ph_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    nq1, nq2, nq3 = _resolve_grid(ph_cfg, "qpoints", ("nq1", "nq2", "nq3"))
    ph_cfg["nq1"] = nq1
    ph_cfg["nq2"] = nq2
    ph_cfg["nq3"] = nq3
    ph_cfg.pop("qpoints", None)
    ph_cfg["fildyn"] = derive_filenames(prefix)["fildyn"]
    if dyn0_only:
        ph_cfg["start_q"] = 1
        ph_cfg["last_q"] = 1
    outdir = config["qe_phonon"]["scf"].get("outdir", "./out")
    path = os.path.join(workspace, filename)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&inputph\n")
        handle.write(f"  prefix = {to_qe_value(prefix)}\n")
        handle.write(f"  outdir = {to_qe_value(outdir)}\n")
        for key, value in ph_cfg.items():
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n")
    return path


def write_split_ph_input_assign_index(
    q_dir: str,
    config: Dict[str, Any],
    prefix: str,
    ph_q_index: int,
) -> str:
    ph_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    nq1, nq2, nq3 = _resolve_grid(ph_cfg, "qpoints", ("nq1", "nq2", "nq3"))
    ph_cfg["nq1"] = nq1
    ph_cfg["nq2"] = nq2
    ph_cfg["nq3"] = nq3
    ph_cfg.pop("qpoints", None)
    ph_cfg["fildyn"] = derive_filenames(prefix)["fildyn"]
    ph_cfg["ldisp"] = True
    ph_cfg["start_q"] = ph_q_index
    ph_cfg["last_q"] = ph_q_index
    outdir = config["qe_phonon"]["scf"].get("outdir", "./out")
    path = os.path.join(q_dir, "split_ph.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&inputph\n")
        handle.write(f"  prefix = {to_qe_value(prefix)}\n")
        handle.write(f"  outdir = {to_qe_value(outdir)}\n")
        for key, value in ph_cfg.items():
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n")
    return path


def write_split_ph_input_assign_coords(
    q_dir: str,
    config: Dict[str, Any],
    prefix: str,
    q_crystal: Sequence[float],
) -> str:
    ph_cfg = copy.deepcopy(config["qe_phonon"]["ph"])
    ph_cfg["ldisp"] = False
    ph_cfg["fildyn"] = derive_filenames(prefix)["fildyn"]
    ph_cfg.pop("qpoints", None)
    ph_cfg.pop("start_q", None)
    ph_cfg.pop("last_q", None)
    outdir = config["qe_phonon"]["scf"].get("outdir", "./out")
    path = os.path.join(q_dir, "split_ph.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&inputph\n")
        handle.write(f"  prefix = {to_qe_value(prefix)}\n")
        handle.write(f"  outdir = {to_qe_value(outdir)}\n")
        for key, value in ph_cfg.items():
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n")
        handle.write(
            f"{float(q_crystal[0]):.15E} {float(q_crystal[1]):.15E} {float(q_crystal[2]):.15E}\n"
        )
    return path


def write_q2r_input(workspace: str, config: Dict[str, Any], prefix: str) -> str:
    q2r_cfg = copy.deepcopy(config["qe_phonon"]["q2r"])
    names = derive_filenames(prefix)
    path = os.path.join(workspace, "q2r.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&input\n")
        handle.write(f"  fildyn = {to_qe_value(names['fildyn'])}\n")
        handle.write(f"  flfrc = {to_qe_value(names['flfrc'])}\n")
        for key, value in q2r_cfg.items():
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n")
    return path


def write_matdyn_phband_input(
    workspace: str,
    config: Dict[str, Any],
    struct_info: Dict[str, Any],
    prefix: str,
) -> str:
    band_cfg = copy.deepcopy(config["qe_phonon"]["phband"])
    names = derive_filenames(prefix)
    kpath_file = str(band_cfg.get("kpath_file", "KPATH.in"))
    if not os.path.exists(os.path.join(workspace, kpath_file)):
        run_vaspkit_for_kpath(workspace)
    path_points, _ = build_matdyn_band_path(
        workspace,
        prefix,
        struct_info["lattice"],
        kpath_file,
        int(band_cfg.get("kpath_points", 100)),
    )
    path = os.path.join(workspace, "matdyn_phband.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&input\n")
        handle.write(f"  asr = {to_qe_value(config['qe_phonon']['q2r'].get('zasr', 'simple'))}\n")
        handle.write(f"  flfrc = {to_qe_value(names['flfrc'])}\n")
        handle.write(f"  flfrq = {to_qe_value(names['flfrq'])}\n")
        if band_cfg.get("q_in_band_form", True):
            handle.write("  q_in_band_form = .true.\n")
        handle.write("/\n")
        handle.write(f"{len(path_points)}\n")
        for coords, _label, weight in path_points:
            handle.write(
                f"{float(coords[0]):16.10f} {float(coords[1]):16.10f} {float(coords[2]):16.10f} {int(weight)}\n"
            )
    return path


def write_matdyn_phdos_input(workspace: str, config: Dict[str, Any], prefix: str) -> str:
    dos_cfg = copy.deepcopy(config["qe_phonon"]["phdos"])
    names = derive_filenames(prefix)
    nk1, nk2, nk3 = _resolve_grid(dos_cfg, "qpoints", ("nk1", "nk2", "nk3"))
    path = os.path.join(workspace, "matdyn_phdos.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&input\n")
        handle.write(f"  asr = {to_qe_value(config['qe_phonon']['q2r'].get('zasr', 'simple'))}\n")
        handle.write(f"  flfrc = {to_qe_value(names['flfrc'])}\n")
        handle.write(f"  fldos = {to_qe_value(names['fldos'])}\n")
        handle.write(f"  dos = {to_qe_value(dos_cfg.get('dos', True))}\n")
        handle.write(f"  ndos = {to_qe_value(dos_cfg.get('ndos', 200))}\n")
        handle.write(f"  nk1 = {nk1}\n")
        handle.write(f"  nk2 = {nk2}\n")
        handle.write(f"  nk3 = {nk3}\n")
        handle.write("/\n")
    return path


def write_root_run_script(
    workspace: str,
    script_name: str,
    slurm_header: str,
    commands: List[str],
    announce: bool = True,
) -> str:
    path = os.path.join(workspace, script_name)
    write_job_script(path, slurm_header, commands, announce=announce)
    return path


def write_split_run_script(q_dir: str, slurm_header: str, pw_cmd: str, ph_cmd: str, announce: bool = True) -> str:
    path = os.path.join(q_dir, "run_split.sh")
    write_job_script(
        path,
        slurm_header,
        [
            f"{pw_cmd} < scf.in > scf.out",
            f"{ph_cmd} < split_ph.in > split_ph.out",
        ],
        announce=announce,
    )
    return path


def write_phonon_marker(workspace: str, filename: str, content: str) -> str:
    path = os.path.join(workspace, filename)
    write_text(path, content)
    return path
