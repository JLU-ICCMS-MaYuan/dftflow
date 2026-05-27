#!/usr/bin/env python3
from typing import Any, Dict

from qeflow.crystal.formula import get_formula


def resolve_prefix(config: Dict[str, Any], struct_info: Dict[str, Any]) -> str:
    prefix = config["qe_phonon"].get("prefix")
    return str(prefix) if prefix else get_formula(struct_info["elements"], struct_info["counts"])
