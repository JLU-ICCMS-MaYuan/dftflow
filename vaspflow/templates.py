#!/usr/bin/env python3
from typing import Any, Dict, Iterable

from public.config_templates import build_cli_config

VASP_SHARED_ALLOWED_KEYS = (
    "potcar_dir",
    "chgcar_path",
    "wavecar_path",
    "lobster_executable_path",
    "vasp",
    "slurm",
)


def _base_vasp_template() -> Dict[str, Any]:
    return {
        "potcar_dir": None,
        "chgcar_path": None,
        "wavecar_path": None,
        "lobster_executable_path": "/home/mayuan/bin/lobster-5.1.1",
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


def _vasp_cohp_base_params() -> Dict[str, Any]:
    return {
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


def get_vasp_cohp_uniform_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_cohp_uniform"] = _with_default_kmesh(_vasp_cohp_base_params())
    return template


def get_vasp_cohp_fatband_parameters_template() -> Dict[str, Any]:
    template = _base_vasp_template()
    template["vasp_cohp_fatband"] = {
            "SYSTEM": "VASP_COHP",
            "ALGO": "Normal",
            "LREAL": ".FALSE.",
            "ISTART": 1,
            "ICHARG": 11,
            "SYMPREC": 1e-5,
            "EDIFF": 1e-8,
            "NELM": 200,
            "ISMEAR": 0,
            "PREC": "Accurate",
            "IBRION": -1,
            "LORBIT": 12,
            "LWAVE": ".TRUE.",
            "LCHARG": ".TRUE.",
            "ISYM": 2,
            "NCORE": 4,
            "scf_kmesh": 0.03,
            "kpath_kmesh": 0.03,
            "kmesh_scheme": "Monkhorst-Pack", # Gamma
        }
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
            "ISTART": 0,
            "ICHARG": 2,
            "ISYM": 2,
            "ENCUT": 800,
            "PREC": "Accurate",
            "SYMPREC": 1e-5,
            "NCORE": 1,
            "ISMEAR": 0,
            "SIGMA": 0.05,
            "NELM": 200,
            "NELMIN": 6,
            "EDIFF": 1e-7,
            "EDIFFG": -0.01,
            "NSW": 500,
            "IBRION": 2,
            "ISIF": 3,
            "POTIM": 0.01,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
            "PSTRESS": 0.0,
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
            "ENCUT": 400,
            "PREC": "Normal",
            "NCORE": 4,
            "KSPACING": 0.5,
            "ISMEAR": 0,
            "SIGMA": 0.2,
            "NELM": 200,
            "NELMIN": 6,
            "EDIFF": 1e-4,
            "EDIFFG": -0.1,
            "NSW": 100,
            "IBRION": 2,
            "ISIF": 2,
            "LWAVE": ".FALSE.",
            "LCHARG": ".FALSE.",
        },
        "stage2": {
            "ISTART": 0,
            "ICHARG": 2,
            "ISYM": 2,
            "ENCUT": 500,
            "PREC": "Normal",
            "NCORE": 4,
            "KSPACING": 0.4,
            "ISMEAR": 0,
            "SIGMA": 0.1,
            "NELM": 200,
            "NELMIN": 6,
            "EDIFF": 1e-5,
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
            "ENCUT": 600,
            "PREC": "Accurate",
            "NCORE": 1,
            "KSPACING": 0.3,
            "ISMEAR": 0,
            "SIGMA": 0.05,
            "NELM": 200,
            "NELMIN": 6,
            "EDIFF": 1e-6,
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


def get_vasp_cohp_uniform_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_cohp_uniform")


def get_vasp_cohp_fatband_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_cohp_fatband")


def get_vasp_locpot_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_locpot")


def get_vasp_charge_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_charge")


def get_vasp_elf_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_elf")


def get_vasp_opt_allowed_keys() -> Iterable[str]:
    return (*VASP_SHARED_ALLOWED_KEYS, "vasp_opt")


def get_vasp_lobsterin_parameters_template() -> Dict[str, Any]:
    return {
        "lobster_executable_path": "/home/mayuan/bin/lobster-5.1.1",
        "slurm": {
            "header": "",
        },
    }


def get_vasp_lobsterin_allowed_keys() -> Iterable[str]:
    return ("lobster_executable_path", "slurm")


def apply_vasp_config_overrides(template: Dict[str, Any], input_config: Dict[str, Any], allowed_keys: Iterable[str]) -> Dict[str, Any]:
    return build_cli_config(template, input_config, allowed_keys)
