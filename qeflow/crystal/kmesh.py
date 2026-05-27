#!/usr/bin/env python3
from typing import Any, List

import numpy as np


def get_kpoints_from_spacing(lattice: np.ndarray, kmesh: Any, default_spacing: float) -> List[int]:
    if isinstance(kmesh, list) and len(kmesh) == 3:
        return [int(kmesh[0]), int(kmesh[1]), int(kmesh[2])]
    reciprocal = 2 * np.pi * np.linalg.inv(lattice).T
    lengths = np.linalg.norm(reciprocal, axis=1)
    spacing = float(kmesh) if isinstance(kmesh, (int, float)) else default_spacing
    return [int(max(1, np.ceil(length / (spacing * 2 * np.pi)))) for length in lengths]

