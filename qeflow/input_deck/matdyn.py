#!/usr/bin/env python3
import copy
import os
from typing import Any, Dict

from qeflow.input_deck.filenames import derive_filenames
from qeflow.input_deck.kpath import build_matdyn_band_path, run_vaspkit_for_kpath
from qeflow.input_deck.ph import resolve_grid
from qeflow.input_deck.values import to_qe_value


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
            handle.write(f"{float(coords[0]):16.10f} {float(coords[1]):16.10f} {float(coords[2]):16.10f} {int(weight)}\n")
    return path


def write_matdyn_phdos_input(workspace: str, config: Dict[str, Any], prefix: str) -> str:
    dos_cfg = copy.deepcopy(config["qe_phonon"]["phdos"])
    names = derive_filenames(prefix)
    nk1, nk2, nk3 = resolve_grid(dos_cfg, "qpoints", ("nk1", "nk2", "nk3"))
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

