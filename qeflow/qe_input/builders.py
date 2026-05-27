#!/usr/bin/env python3
from qeflow.qe_input.kpath import build_matdyn_band_path, run_vaspkit_for_kpath
from qeflow.qe_input.phonon_input import (
    build_qe_commands,
    derive_filenames,
    get_kpoints,
    resolve_prefix,
    to_qe_value,
    write_matdyn_phband_input,
    write_matdyn_phdos_input,
    write_ph_input,
    write_phonon_marker,
    write_q2r_input,
    write_root_run_script,
    write_scf_input,
    write_split_ph_input_assign_coords,
    write_split_ph_input_assign_index,
    write_split_run_script,
)
from qeflow.qe_input.structure import get_formula, get_kpoints_from_spacing, parse_poscar
