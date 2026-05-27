#!/usr/bin/env python3
from typing import Dict


def derive_filenames(prefix: str) -> Dict[str, str]:
    return {
        "fildyn": f"{prefix}.dyn",
        "dyn0": f"{prefix}.dyn0",
        "flfrc": f"{prefix}.fc",
        "flfrq": f"{prefix}.phband.freq",
        "fldos": f"{prefix}.phdos.dat",
    }

