#!/usr/bin/env python3
from typing import Any, Dict

from public.config_templates import merge_parameter_template


def _base_qe_template() -> Dict[str, Any]:
    return {
        "qe": {
            "executable_path": None,
        },
        "slurm": {
            "header": "",
        },
        "k_points": {
            "kmesh": 0.04,
            "kpoints": None,
            "wan": False,
            "kpath_points": 100,
            "kpath_file": "KPATH.in",
        },
        "pseudo_map": {},
        "qe_params": {},
    }


def get_qe_scf_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_params"] = {
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
    }
    return template


def get_qe_nscf_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_params"] = {
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
    }
    template["nscf_params"] = {}
    return template


def get_qe_eband_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_params"] = {
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
    }
    template["eband_params"] = {}
    template["eband"] = {
        "filband": None,
        "lsym": ".true.",
    }
    template["bands"] = {
        "executable_path": None,
        "args": "",
    }
    template["projwfc"] = {
        "executable_path": None,
        "args": "-pd .true.",
    }
    return template


def get_qe_eledos_parameters_template() -> Dict[str, Any]:
    template = _base_qe_template()
    template["qe_params"] = {
        "CONTROL": {
            "outdir": "'./tmp'",
        },
    }
    template["eledos"] = {
        "DeltaE": 0.01,
        "emin": -10,
        "emax": 30,
        "ngauss": 0,
        "degauss": 0.0,
        "fildos": None,
        "filpdos": None,
        "filproj": None,
        "outdir": None,
    }
    template["dos"] = {
        "executable_path": None,
        "args": "-pd .true.",
    }
    template["projwfc"] = {
        "executable_path": None,
        "args": "-pd .true.",
    }
    return template


def apply_qe_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any]) -> Dict[str, Any]:
    return merge_parameter_template(template, input_config)
