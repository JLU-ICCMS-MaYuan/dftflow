#!/usr/bin/env python3
from typing import Any, Dict, Iterable

from mattersimflow._public_compat import build_cli_config


MATTERSIM_SHARED_ALLOWED_KEYS = (
    "mattersim_source_path",
    "mattersim",
    "slurm",
)


def _base_mattersim_template() -> Dict[str, Any]:
    return {
        "mattersim_source_path": "external/mattersim/src",
        "mattersim": {
            "executable_path": "python",
        },
        "slurm": {
            "header": "",
        },
    }


def get_mattersim_opt_parameters_template() -> Dict[str, Any]:
    template = _base_mattersim_template()
    template["mattersim_opt"] = {
        "model": "mattersim-v1.0.0-1m",
        "device": "cpu",
        "stage1": {
            "optimizer": "FIRE",
            "constrain_symmetry": True,
            "fmax": 0.10,
            "ionic_steps": 200,
        },
        "stage2": {
            "optimizer": "FIRE",
            "cell_filter": "FrechetCellFilter",
            "constrain_symmetry": True,
            "fix_axis": False,
            "fmax": 0.05,
            "ionic_steps": 300,
        },
    }
    return template


def get_mattersim_opt_allowed_keys() -> Iterable[str]:
    return (*MATTERSIM_SHARED_ALLOWED_KEYS, "mattersim_opt")


def apply_mattersim_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any], allowed_keys: Iterable[str]) -> Dict[str, Any]:
    return build_cli_config(template, input_config, allowed_keys)
