#!/usr/bin/env python3
from typing import Any, Dict, Iterable

from public.config_templates import build_cli_config

VASP_SHARED_ALLOWED_KEYS = (
    "potcar_dir",
    "chgcar_path",
    "wavecar_path",
    "vasp",
    "slurm",
)


def _base_vasp_template() -> Dict[str, Any]:
    return {
        "potcar_dir": None,
        "chgcar_path": None,
        "wavecar_path": None,
        "vasp": {
            "executable_path": None,
        },
        "slurm": {
            "header": "",
        },
    }


def _with_default_kmesh(params: Dict[str, Any]) -> Dict[str, Any]:
    merged = {"kmesh": 0.025}
    merged.update(params)
    return merged


def get_vasp_scf_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_scf"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_Job",
            "ALGO": "Normal",
            "ISYM": 2,
            "ISTART": 0,
            "ICHARG": 2,
            "PREC": "Accurate",
            "IBRION": -1,
            "LWAVE": ".TRUE.",
            "LCHARG": ".TRUE.",
            "NCORE": 4,
        }
    )
    return template


def get_vasp_scf_converge_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_scf_converge"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_Job",
            "ALGO": "Normal",
            "ISYM": 2,
            "ISTART": 0,
            "ICHARG": 2,
            "PREC": "Accurate",
            "IBRION": -1,
            "LWAVE": ".TRUE.",
            "LCHARG": ".TRUE.",
            "NCORE": 4,
        }
    )
    return template


def get_vasp_eband_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_eband"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_Band",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "ISMEAR": 0,
            "SIGMA": 0.05,
            "LORBIT": 11,
            "LWAVE": ".TRUE.",
            "LCHARG": ".FALSE.",
            "NCORE": 4,
        }
    )
    return template


def get_vasp_eledos_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_eledos"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_DOS",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "ISMEAR": -5,
            "LORBIT": 11,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
            "NCORE": 4,
            "NEDOS": 2000,
        }
    )
    return template


def get_vasp_fermi_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_fermi"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_FermiSurface",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
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
    )
    return template


def get_vasp_cohp_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_cohp"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_COHP",
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "SYMPREC": 1e-5,
            "EDIFF": 1e-8,
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
    )
    return template


def get_vasp_locpot_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_locpot"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_LOCPOT",
            "ISYM": 2,
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
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
    )
    return template


def get_vasp_charge_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_charge"] = _with_default_kmesh(
        {
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
    )
    return template


def get_vasp_elf_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_elf"] = _with_default_kmesh(
        {
            "SYSTEM": "VASP_Job",
            "ISTART": 1,
            "ICHARG": 11,
            "PREC": "Accurate",
            "IBRION": -1,
            "LELF": ".TRUE.",
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
        }
    )
    return template


def get_vasp_opt_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_opt"] = _with_default_kmesh(
        {
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
    )
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
    return {
        "slurm": {
            "header": "",
        },
    }


def get_vasp_scf_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_scf")


def get_vasp_scf_converge_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_scf_converge")


def get_vasp_eband_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_eband")


def get_vasp_eledos_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_eledos")


def get_vasp_fermi_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_fermi")


def get_vasp_cohp_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_cohp")


def get_vasp_locpot_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_locpot")


def get_vasp_charge_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_charge")


def get_vasp_elf_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_elf")


def get_vasp_opt_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_opt")


def get_vasp_lobster_allowed_keys() -> Iterable[str]:
    return ("slurm",)


def apply_vasp_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any], allowed_keys: Iterable[str]) -> Dict[str, Any]:
    return build_cli_config(template, input_config, allowed_keys)
