#!/usr/bin/env python3
from typing import Any, Dict

from public.config_templates import merge_parameter_template


def _base_wannier_template() -> Dict[str, Any]:
    return {
        "slurm": {
            "header": "",
        },
        "wannier90": {
            "executable_path": "wannier90.x",
        },
        "pw2wannier90": {
            "executable_path": "pw2wannier90.x",
            "input_file": None,
            "prefix": None,
            "outdir": "./",
            "seedname": None,
            "write_amn": True,
            "write_mmn": True,
            "write_unk": None,
            "spin_component": None,
            "wan_mode": None,
        },
        "postw90": {
            "executable_path": "postw90.x",
        },
        "projections": {
            "list": [],
        },
        "k_points": {
            "mp_grid": [4, 4, 4],
            "kpoints": None,
            "kpath_file": "KPATH.in",
        },
    }


def get_wannier_init_parameters_template() -> Dict[str, Any]:
    template = _base_wannier_template()
    template["win"] = {
        "system_name": None,
        "num_wann": None,
        "num_bands": None,
        "exclude_bands": None,
        "iprint": None,
        "num_iter": None,
        "dis_num_iter": None,
        "dis_froz_min": None,
        "dis_froz_max": None,
        "dis_win_min": None,
        "dis_win_max": None,
        "write_hr": None,
        "write_bvec": None,
        "bands_plot": False,
        "bands_plot_format": None,
        "wvfn_formatted": None,
        "wannier_plot": None,
        "wannier_plot_format": None,
        "fermi_energy": None,
        "fermi_surface_plot": None,
        "dos": None,
        "dos_kmesh": None,
        "dos_project": None,
    }
    return template


def get_wannier_post_parameters_template() -> Dict[str, Any]:
    template = _base_wannier_template()
    template["dos"] = {
        "dos": True,
        "kmesh": None,
        "project": None,
        "dos_project": None,
    }
    return template


def apply_wannier_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any]) -> Dict[str, Any]:
    return merge_parameter_template(template, input_config)
