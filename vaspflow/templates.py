#!/usr/bin/env python3
from typing import Any, Dict

from public.config_templates import merge_parameter_template


def _base_vasp_template() -> Dict[str, Any]:
    return {
        "potcar_dir": None,
        "kmesh": None,
        "chgcar_path": None,
        "wavecar_path": None,
        "vasp": {
            "executable_path": None,
        },
        "slurm": {
            "header": "",
        },
    }


def get_vasp_scf_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["scf_params"] = {
        "SYSTEM": "VASP_Job",
        "ISTART": 0,
        "ICHARG": 2,
        "PREC": "Accurate",
        "IBRION": -1,
        "LWAVE": ".TRUE.",
        "LCHARG": ".TRUE.",
        "NCORE": 4,
    }
    return template


def get_vasp_eband_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["eband_params"] = {
        "SYSTEM": "VASP_Band",
        "ISTART": 1,
        "ICHARG": 11,
        "PREC": "Accurate",
        "IBRION": -1,
        "NSW": 0,
        "ISMEAR": 0,
        "SIGMA": 0.05,
        "LORBIT": 11,
        "LWAVE": ".TRUE.",
        "LCHARG": ".FALSE.",
        "NCORE": 4,
    }
    return template


def get_vasp_eledos_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["eledos_params"] = {
        "SYSTEM": "VASP_DOS",
        "ISTART": 1,
        "ICHARG": 11,
        "PREC": "Accurate",
        "IBRION": -1,
        "NSW": 0,
        "ISMEAR": -5,
        "LORBIT": 11,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE.",
        "NCORE": 4,
        "NEDOS": 2000,
    }
    return template


def get_vasp_fermisurface_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["fermisurface_params"] = {
        "SYSTEM": "VASP_FermiSurface",
        "ISTART": 1,
        "ICHARG": 11,
        "PREC": "Accurate",
        "IBRION": -1,
        "NSW": 0,
        "ISMEAR": 0,
        "SIGMA": 0.05,
        "LORBIT": 11,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE.",
    }
    return template


def get_vasp_cohp_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["cohp_params"] = {
        "SYSTEM": "VASP_COHP",
        "ISTART": 1,
        "ICHARG": 11,
        "SYMPREC": 1e-5,
        "EDIFF": 1e-8,
        "ALGO": "Normal",
        "NELM": 200,
        "NELMIN": 2,
        "ISMEAR": -5,
        "PREC": "Accurate",
        "IBRION": -1,
        "LORBIT": 12,
        "LWAVE": ".TRUE.",
        "LCHARG": ".TRUE.",
        "ISYM": -1,
        "NCORE": 4,
        "NEDOS": 2000,
    }
    return template


def get_vasp_locpot_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["locpot_params"] = {
        "SYSTEM": "VASP_LOCPOT",
        "ISTART": 1,
        "ICHARG": 11,
        "PREC": "Accurate",
        "IBRION": -1,
        "NSW": 0,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE.",
        "NCORE": 4,
        "LVHAR": ".TRUE.",
    }
    return template


def get_vasp_charge_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["charge_params"] = {
        "SYSTEM": "VASP_CHARGE",
        "ISTART": 1,
        "ICHARG": 11,
        "PREC": "Accurate",
        "IBRION": -1,
        "NSW": 0,
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE.",
        "NCORE": 4,
        "LORBIT": 11,
        "LPARD": ".TRUE.",
    }
    return template


def get_vasp_elf_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["elf_params"] = {
        "SYSTEM": "VASP_Job",
        "ISTART": 1,
        "ICHARG": 11,
        "PREC": "Accurate",
        "IBRION": -1,
        "LELF": ".TRUE.",
        "LWAVE": ".FALSE.",
        "LCHARG": ".FALSE.",
    }
    return template


def get_vasp_opt_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["opt_params"] = {
        "ENCUT": 520,
        "ISMEAR": 0,
        "SIGMA": 0.05,
        "ISYM": 0,
        "SYMPREC": 1e-5,
        "PSTRESS": 0.0,
        "NCORE": 4,
        "NELM": 200,
        "POTIM": 0.05,
        "EDIFF": 1e-7,
        "EDIFFG": -0.01,
        "IBRION": 2,
        "ISIF": 3,
        "KSPACING": None,
        "NPAR": None,
        "KPAR": None,
    }
    template["opt_stage_templates"] = {
        "stage1": {
            "ISTART": 0,
            "ICHARG": 2,
            "ISYM": 2,
            "ENCUT": 300,
            "PREC": "Normal",
            "NCORE": 4,
            "KSPACING": 0.8,
            "ISMEAR": 0,
            "SIGMA": 0.2,
            "NELM": 200,
            "NELMIN": 6,
            "EDIFF": 1e-3,
            "EDIFFG": -0.2,
            "NSW": 100,
            "IBRION": 2,
            "ISIF": 2,
            "POTIM": 0.3,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
        },
        "stage2": {
            "ISTART": 0,
            "ICHARG": 2,
            "ISYM": 2,
            "ENCUT": 400,
            "PREC": "Normal",
            "NCORE": 4,
            "KSPACING": 0.6,
            "ISMEAR": 0,
            "SIGMA": 0.1,
            "NELM": 200,
            "NELMIN": 6,
            "EDIFF": 1e-4,
            "EDIFFG": -0.1,
            "NSW": 200,
            "IBRION": 2,
            "ISIF": 4,
            "POTIM": 0.1,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
        },
        "stage3": {
            "ISTART": 0,
            "ICHARG": 2,
            "ISYM": 2,
            "ENCUT": 500,
            "PREC": "Accurate",
            "NCORE": 4,
            "KSPACING": 0.4,
            "ISMEAR": 0,
            "SIGMA": 0.05,
            "NELM": 90,
            "NELMIN": 6,
            "EDIFF": 1e-5,
            "EDIFFG": -0.05,
            "NSW": 300,
            "IBRION": 2,
            "ISIF": 3,
            "POTIM": 0.05,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
        },
    }
    return template


def get_vasp_lobster_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    return template


def apply_vasp_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any]) -> Dict[str, Any]:
    return merge_parameter_template(template, input_config)
