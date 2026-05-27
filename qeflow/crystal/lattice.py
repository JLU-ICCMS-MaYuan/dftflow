#!/usr/bin/env python3
import math
import os
import re
import shlex
import subprocess
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Sequence, Tuple

import numpy as np

BOHR_RADIUS_ANGS = 0.529177210903


@dataclass(frozen=True)
class LatticeParameters:
    ibrav: int
    celldm: Tuple[float, float, float, float, float, float]
    source: str


def resolve_lattice_parameters(
    config: Dict[str, Any], lattice_angstrom: np.ndarray
) -> LatticeParameters:
    lattice_bohr = np.asarray(lattice_angstrom, dtype=float) / BOHR_RADIUS_ANGS
    exe = _find_cell2ibrav_executable(config.get("qe", {}).get("executable_path"))
    if exe:
        try:
            return _run_cell2ibrav(exe, lattice_bohr)
        except Exception:
            pass
    return _python_cell2ibrav(lattice_bohr)


def _find_cell2ibrav_executable(executable_path: Any) -> Optional[str]:
    value = str(executable_path).strip() if executable_path is not None else ""
    if not value:
        return None
    for token in reversed(shlex.split(value)):
        candidate = os.path.expandvars(os.path.expanduser(token))
        if os.path.basename(candidate) == "cell2ibrav.x" and os.access(candidate, os.X_OK):
            return candidate
        if os.path.isdir(candidate):
            tool = os.path.join(candidate, "cell2ibrav.x")
            if os.access(tool, os.X_OK):
                return tool
    return None


def _run_cell2ibrav(executable: str, lattice_bohr: np.ndarray) -> LatticeParameters:
    input_text = "\n".join(
        " ".join(f"{value:.16f}" for value in row) for row in lattice_bohr
    )
    input_text += "\n1\n"
    result = subprocess.run(
        [executable],
        input=input_text,
        text=True,
        capture_output=True,
        check=True,
    )
    ibrav_match = re.search(r"ibrav\s*=\s*([+-]?\d+)", result.stdout)
    if not ibrav_match:
        raise ValueError("cell2ibrav.x 输出中未找到 ibrav")
    celldm = [0.0] * 6
    for match in re.finditer(r"celldm\((\d)\)\s*=\s*([+-]?\d+(?:\.\d*)?(?:[EeDd][+-]?\d+)?)", result.stdout):
        index = int(match.group(1))
        if 1 <= index <= 6:
            celldm[index - 1] = float(match.group(2).replace("D", "E").replace("d", "e"))
    discrepancy = re.search(
        r"Discrepancy old/new in bohr\s*=\s*([+-]?\d+(?:\.\d*)?(?:[EeDd][+-]?\d+)?)",
        result.stdout,
    )
    if discrepancy and float(discrepancy.group(1).replace("D", "E").replace("d", "e")) > 1.0e-4:
        raise ValueError(f"cell2ibrav.x 重构晶胞偏差过大: {discrepancy.group(1)} bohr")
    return LatticeParameters(int(ibrav_match.group(1)), tuple(celldm), "cell2ibrav.x")


def _python_cell2ibrav(lattice_bohr: np.ndarray) -> LatticeParameters:
    a1, a2, a3 = (np.asarray(row, dtype=float) for row in lattice_bohr)
    ibrav = _at2ibrav(a1, a2, a3)
    celldm = _at2celldm(ibrav, 1.0, a1, a2, a3)
    return LatticeParameters(ibrav, tuple(celldm), "python")


def _norm(vector: np.ndarray) -> float:
    return float(np.linalg.norm(vector))


def _dot(left: np.ndarray, right: np.ndarray) -> float:
    return float(np.dot(left, right))


def _eqq(x: float, y: float) -> bool:
    return abs(x - y) < 1.0e-5


def _neqq(x: float, y: float) -> bool:
    return abs(x - y) > 1.0e-5


def _at2ibrav(a1: np.ndarray, a2: np.ndarray, a3: np.ndarray) -> int:
    v1, v2, v3 = _norm(a1), _norm(a2), _norm(a3)
    cosbc = _dot(a2, a3) / v2 / v3
    cosac = _dot(a1, a3) / v1 / v3
    cosab = _dot(a1, a2) / v1 / v2
    ibrav = 14
    if _eqq(v1, v2) and _eqq(v1, v3):
        if _eqq(cosab, cosac) and _eqq(cosab, cosbc):
            if _eqq(cosab, 0.0):
                ibrav = 1
            elif _eqq(cosab, 0.5):
                ibrav = 2
            elif _eqq(cosab, -1.0 / 3.0):
                ibrav = -3
            elif _eqq(abs(a1[2]), abs(a2[2])) and _eqq(abs(a2[2]), abs(a3[2])):
                ibrav = 5
            else:
                ibrav = -5
        elif _eqq(cosab, cosac) and _neqq(cosab, cosbc):
            if _eqq(abs(a1[0]), abs(a1[1])) and _eqq(abs(a2[0]), abs(a2[1])):
                ibrav = 7
            else:
                ibrav = 3
        elif _eqq(cosab, -cosac) and _eqq(cosab, cosbc) and _eqq(cosab, 1.0 / 3.0):
            ibrav = 3
        elif _eqq(abs(a1[0]), abs(a2[0])) and _eqq(abs(a2[1]), abs(a2[1])):
            ibrav = 11
    elif _eqq(v1, v2) and _neqq(v1, v3):
        if _eqq(cosab, 0.0) and _eqq(cosac, 0.0) and _eqq(cosbc, 0.0):
            ibrav = 6
        elif _eqq(cosab, -0.5) and _eqq(cosac, 0.0) and _eqq(cosbc, 0.0):
            ibrav = 4
        elif _eqq(cosac, 0.0) and _eqq(cosbc, 0.0):
            if _eqq(a1[0], a2[0]) and _eqq(a1[1], -a2[1]):
                ibrav = -9
            elif _eqq(a1[0], -a2[0]) and _eqq(a1[1], a2[1]):
                ibrav = 9
        elif _eqq(cosac, -cosbc):
            ibrav = -13
    elif _eqq(v1, v3) and _neqq(v1, v2):
        ibrav = 13
    elif _eqq(v2, v3) and _neqq(v1, v2):
        ibrav = 91
    elif _neqq(v1, v2) and _neqq(v1, v3) and _neqq(v2, v3):
        if _eqq(cosab, 0.0) and _eqq(cosac, 0.0) and _eqq(cosbc, 0.0):
            ibrav = 8
        elif _neqq(cosab, 0.0) and _eqq(cosac, 0.0) and _eqq(cosbc, 0.0):
            ibrav = 12
        elif _eqq(cosab, 0.0) and _neqq(cosac, 0.0) and _eqq(cosbc, 0.0):
            ibrav = -12
        elif _neqq(cosab, 0.0) and _neqq(cosac, 0.0) and _neqq(cosbc, 0.0):
            if (
                _eqq(abs(a1[0]), abs(a2[0]))
                and _eqq(abs(a1[2]), abs(a3[2]))
                and _eqq(abs(a2[1]), abs(a3[1]))
            ):
                ibrav = 10
            else:
                ibrav = 14
    return ibrav


def _at2celldm(
    ibrav: int, alat: float, a1: np.ndarray, a2: np.ndarray, a3: np.ndarray
) -> List[float]:
    celldm = [0.0] * 6
    if ibrav == 0:
        celldm[0] = 1.0
    elif ibrav == 1:
        celldm[0] = _norm(a1)
    elif ibrav == 2:
        celldm[0] = math.sqrt(_dot(a1, a1) * 2.0)
    elif ibrav in (3, -3):
        celldm[0] = math.sqrt(_dot(a1, a1) / 3.0) * 2.0
    elif ibrav == 4:
        celldm[0] = _norm(a1)
        celldm[2] = _norm(a3) / celldm[0]
    elif ibrav in (5, -5):
        celldm[0] = _norm(a1)
        celldm[3] = _dot(a1, a2) / celldm[0] / _norm(a2)
    elif ibrav == 6:
        celldm[0] = _norm(a1)
        celldm[2] = _norm(a3) / celldm[0]
    elif ibrav == 7:
        celldm[0] = abs(a1[0]) * 2.0
        celldm[2] = abs(a1[2] / a1[0])
    elif ibrav == 8:
        celldm[0] = _norm(a1)
        celldm[1] = _norm(a2) / celldm[0]
        celldm[2] = _norm(a3) / celldm[0]
    elif ibrav in (9, -9):
        celldm[0] = abs(a1[0]) * 2.0
        celldm[1] = abs(a2[1]) * 2.0 / celldm[0]
        celldm[2] = abs(a3[2]) / celldm[0]
    elif ibrav == 91:
        celldm[0] = _norm(a1)
        celldm[1] = abs(a2[1]) * 2.0 / celldm[0]
        celldm[2] = abs(a3[2]) * 2.0 / celldm[0]
    elif ibrav == 10:
        celldm[0] = abs(a1[0]) * 2.0
        celldm[1] = abs(a2[1]) * 2.0 / celldm[0]
        celldm[2] = abs(a3[2]) * 2.0 / celldm[0]
    elif ibrav == 11:
        celldm[0] = abs(a1[0]) * 2.0
        celldm[1] = abs(a1[1]) * 2.0 / celldm[0]
        celldm[2] = abs(a1[2]) * 2.0 / celldm[0]
    elif ibrav in (12, -12):
        celldm[0] = _norm(a1)
        celldm[1] = _norm(a2) / celldm[0]
        celldm[2] = _norm(a3) / celldm[0]
        if ibrav == 12:
            celldm[3] = _dot(a1, a2) / celldm[0] / _norm(a2)
        else:
            celldm[4] = _dot(a1, a3) / celldm[0] / _norm(a3)
    elif ibrav == 13:
        celldm[0] = abs(a1[0]) * 2.0
        celldm[1] = _norm(a2) / celldm[0]
        celldm[2] = abs(a1[2] / a1[0])
        celldm[3] = a2[0] / a1[0] / celldm[1] / 2.0
    elif ibrav == -13:
        celldm[0] = abs(a1[0]) * 2.0
        celldm[1] = abs(a2[1] / a2[0])
        celldm[2] = _norm(a3) / celldm[0]
        celldm[4] = a3[0] / a1[0] / celldm[2] / 2.0
    elif ibrav == 14:
        celldm[0] = _norm(a1)
        celldm[1] = _norm(a2) / celldm[0]
        celldm[2] = _norm(a3) / celldm[0]
        celldm[3] = _dot(a3, a2) / _norm(a2) / _norm(a3)
        celldm[4] = _dot(a3, a1) / celldm[0] / _norm(a3)
        celldm[5] = _dot(a1, a2) / celldm[0] / _norm(a2)
    celldm[0] *= alat
    return celldm
