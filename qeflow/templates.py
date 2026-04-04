#!/usr/bin/env python3
from typing import Any, Dict, Iterable

from public.config_templates import build_cli_config

QE_SHARED_ALLOWED_KEYS = ("qe", "slurm", "pseudo_map")


def _base_qe_template() -> Dict[str, Any]:
    return {
        "qe": {
            "executable_path": None,
        },
        "slurm": {
            "header": "",
        },
        "pseudo_map": {},
    }


def get_qe_scf_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_scf"] = {
        "CONTROL": {
            "calculation": "'scf'",
            "restart_mode": "'from_scratch'",
            "prefix": "'qe'",
            "outdir": "'./out'",
            "pseudo_dir": "'./pseudo'",
            "verbosity": "'high'",
            "tprnfor": ".true.",
            "tstress": ".true.",
            "forc_conv_thr": "1.0e-6",
            "etot_conv_thr": "1.0e-7",
        },
        "SYSTEM": {
            "ibrav": 0,
            "ecutwfc": 50.0,
            "ecutrho": 400.0,
            "occupations": "'smearing'",
            "smearing": "'gaussian'",
            "degauss": 0.01,
        },
        "ELECTRONS": {
            "conv_thr": 1.0e-8,
            "mixing_beta": 0.7,
            "electron_maxstep": 400,
        },
        "k_points": {
            "kmesh": 0.04,
            "kpoints": None,
            "wan": False,
        },
    }
    return template


def get_qe_nscf_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_nscf"] = {
        "CONTROL": {
            "calculation": "'nscf'",
            "restart_mode": "'from_scratch'",
            "prefix": "'qe'",
            "outdir": "'./out'",
            "pseudo_dir": "'./pseudo'",
            "verbosity": "'high'",
            "tprnfor": ".true.",
            "tstress": ".true.",
            "forc_conv_thr": "1.0e-6",
            "etot_conv_thr": "1.0e-7",
        },
        "SYSTEM": {
            "ibrav": 0,
            "ecutwfc": 50.0,
            "ecutrho": 400.0,
            "occupations": "'tetrahedra'",
        },
        "ELECTRONS": {
            "conv_thr": 1.0e-8,
            "mixing_beta": 0.7,
            "electron_maxstep": 400,
        },
        "k_points": {
            "kmesh": 0.02,
            "kpoints": None,
            "wan": False,
        },
    }
    return template


def get_qe_eband_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_eband"] = {
        "CONTROL": {
            "calculation": "'bands'",
            "restart_mode": "'from_scratch'",
            "prefix": "'qe'",
            "outdir": "'./out'",
            "pseudo_dir": "'./pseudo'",
            "verbosity": "'high'",
            "tprnfor": ".true.",
            "tstress": ".true.",
            "forc_conv_thr": "1.0e-6",
            "etot_conv_thr": "1.0e-7",
        },
        "SYSTEM": {
            "ibrav": 0,
            "ecutwfc": 50.0,
            "ecutrho": 400.0,
            "occupations": "'tetrahedra'",
        },
        "ELECTRONS": {
            "conv_thr": 1.0e-8,
            "mixing_beta": 0.7,
            "electron_maxstep": 400,
        },
        "k_points": {
            "kpath_points": 100,
            "kpath_file": "KPATH.in",
        },
        "eband": {
            "filband": None,
            "lsym": ".true.",
        },
        "bands": {
            "executable_path": None,
            "args": "",
        },
        "projwfc": {
            "executable_path": None,
            "args": "-pd .true.",
        },
    }
    return template


def get_qe_eledos_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_eledos"] = {
        "CONTROL": {
            "outdir": "'./tmp'",
        },
        "SYSTEM": {},
        "ELECTRONS": {},
        "eledos": {
            "DeltaE": 0.01,
            "emin": -10,
            "emax": 30,
            "ngauss": 0,
            "degauss": 0.0,
            "fildos": None,
            "filpdos": None,
            "filproj": None,
            "outdir": None,
        },
        "dos": {
            "executable_path": None,
            "args": "-pd .true.",
        },
        "projwfc": {
            "executable_path": None,
            "args": "-pd .true.",
        },
    }
    return template


def get_qe_scf_allowed_keys() -> Iterable[str]:
    return (*QE_SHARED_ALLOWED_KEYS, "qe_scf")


def get_qe_nscf_allowed_keys() -> Iterable[str]:
    return (*QE_SHARED_ALLOWED_KEYS, "qe_nscf")


def get_qe_eband_allowed_keys() -> Iterable[str]:
    return (*QE_SHARED_ALLOWED_KEYS, "qe_eband")


def get_qe_eledos_allowed_keys() -> Iterable[str]:
    return (*QE_SHARED_ALLOWED_KEYS, "qe_eledos")


def apply_qe_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any], allowed_keys: Iterable[str]) -> Dict[str, Any]:
    return build_cli_config(template, input_config, allowed_keys)
