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
LATTICE_MODE_IBRAV_ANGSTROM = "ibrav_angstrom"
LATTICE_MODE_IBRAV_CELLDM = "ibrav_celldm"
LATTICE_MODE_CELL_PARAMETERS = "cell_parameters"
VALID_LATTICE_MODES = {
    LATTICE_MODE_IBRAV_ANGSTROM,
    LATTICE_MODE_IBRAV_CELLDM,
    LATTICE_MODE_CELL_PARAMETERS,
}
MANUAL_LATTICE_KEYS = {
    "ibrav",
    "A",
    "B",
    "C",
    "cosAB",
    "cosAC",
    "cosBC",
    *(f"celldm({index})" for index in range(1, 7)),
}


@dataclass(frozen=True)
class LatticeParameters:
    ibrav: int
    celldm: Tuple[float, float, float, float, float, float]
    source: str


def resolve_lattice_mode(config: Dict[str, Any]) -> str:
    mode = str(config.get("qe", {}).get("lattice_mode", LATTICE_MODE_IBRAV_ANGSTROM))
    if mode not in VALID_LATTICE_MODES:
        allowed = ", ".join(sorted(VALID_LATTICE_MODES))
        raise ValueError(f"[qe].lattice_mode = {mode!r} 不合法，允许值: {allowed}")
    return mode


def validate_no_manual_lattice_params(raw_config: Dict[str, Any]) -> None:
    conflicts: List[str] = []

    def walk(value: Any, path: str) -> None:
        if not isinstance(value, dict):
            return
        for key, child in value.items():
            child_path = f"{path}.{key}" if path else str(key)
            if key in MANUAL_LATTICE_KEYS:
                conflicts.append(child_path)
            walk(child, child_path)

    for root in ("qe", "qe_scf", "qe_nscf", "qe_eband", "qe_phonon"):
        walk(raw_config.get(root), root)
    if conflicts:
        joined = ", ".join(conflicts)
        raise ValueError(
            "不允许在 input.toml 中手动指定 QE 晶格参数；请删除这些 key，"
            f"由 lattice_mode 自动生成: {joined}"
        )


def write_system_lattice(
    handle,
    config: Dict[str, Any],
    struct_info: Dict[str, Any],
) -> None:
    mode = resolve_lattice_mode(config)
    lattice = np.asarray(struct_info["lattice"], dtype=float)
    if mode == LATTICE_MODE_CELL_PARAMETERS:
        handle.write("  ibrav = 0\n")
        return

    params = resolve_lattice_parameters(config, lattice)
    handle.write(f"  ibrav = {params.ibrav}\n")
    if mode == LATTICE_MODE_IBRAV_CELLDM:
        for index, value in _required_celldm(params.ibrav, params.celldm):
            handle.write(f"  celldm({index}) = {_format_float(value)}\n")
        return

    for key, value in _required_abc(params.ibrav, params.celldm):
        handle.write(f"  {key} = {_format_float(value)}\n")


def write_atomic_positions(handle, struct_info: Dict[str, Any], precision: int = 17) -> None:
    coord_type = str(struct_info["coord_type"]).lower()
    if coord_type.startswith("d"):
        handle.write("ATOMIC_POSITIONS {crystal}\n")
    else:
        handle.write("ATOMIC_POSITIONS {angstrom}\n")
    atom_index = 0
    width = precision + 4
    for element_index, element in enumerate(struct_info["elements"]):
        for _ in range(struct_info["counts"][element_index]):
            pos = struct_info["positions"][atom_index]
            handle.write(
                f"  {element:3} {pos[0]:{width}.{precision}f} "
                f"{pos[1]:{width}.{precision}f} {pos[2]:{width}.{precision}f}\n"
            )
            atom_index += 1
    handle.write("\n")


def write_cell_parameters_if_needed(
    handle,
    config: Dict[str, Any],
    struct_info: Dict[str, Any],
    precision: int = 17,
) -> None:
    if resolve_lattice_mode(config) != LATTICE_MODE_CELL_PARAMETERS:
        return
    width = precision + 4
    handle.write("CELL_PARAMETERS {angstrom}\n")
    for vec in struct_info["lattice"]:
        handle.write(
            f"  {vec[0]:{width}.{precision}f} {vec[1]:{width}.{precision}f} "
            f"{vec[2]:{width}.{precision}f}\n"
        )
    handle.write("\n")


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


def _required_celldm(ibrav: int, celldm: Sequence[float]) -> List[Tuple[int, float]]:
    indices = {
        1: (1,),
        2: (1,),
        3: (1,),
        -3: (1,),
        4: (1, 3),
        5: (1, 4),
        -5: (1, 4),
        6: (1, 3),
        7: (1, 3),
        8: (1, 2, 3),
        9: (1, 2, 3),
        -9: (1, 2, 3),
        91: (1, 2, 3),
        10: (1, 2, 3),
        11: (1, 2, 3),
        12: (1, 2, 3, 4),
        -12: (1, 2, 3, 5),
        13: (1, 2, 3, 4),
        -13: (1, 2, 3, 5),
        14: (1, 2, 3, 4, 5, 6),
    }.get(ibrav)
    if indices is None:
        raise ValueError(f"不支持的 ibrav: {ibrav}")
    return [(index, float(celldm[index - 1])) for index in indices]


def _required_abc(ibrav: int, celldm: Sequence[float]) -> List[Tuple[str, float]]:
    a, b, c, cosab, cosac, cosbc = _celldm2abc(ibrav, celldm)
    mapping = {
        1: (("A", a),),
        2: (("A", a),),
        3: (("A", a),),
        -3: (("A", a),),
        4: (("A", a), ("C", c)),
        5: (("A", a), ("cosAB", cosab)),
        -5: (("A", a), ("cosAB", cosab)),
        6: (("A", a), ("C", c)),
        7: (("A", a), ("C", c)),
        8: (("A", a), ("B", b), ("C", c)),
        9: (("A", a), ("B", b), ("C", c)),
        -9: (("A", a), ("B", b), ("C", c)),
        91: (("A", a), ("B", b), ("C", c)),
        10: (("A", a), ("B", b), ("C", c)),
        11: (("A", a), ("B", b), ("C", c)),
        12: (("A", a), ("B", b), ("C", c), ("cosAB", cosab)),
        -12: (("A", a), ("B", b), ("C", c), ("cosAC", cosac)),
        13: (("A", a), ("B", b), ("C", c), ("cosAB", cosab)),
        -13: (("A", a), ("B", b), ("C", c), ("cosAC", cosac)),
        14: (("A", a), ("B", b), ("C", c), ("cosAB", cosab), ("cosAC", cosac), ("cosBC", cosbc)),
    }.get(ibrav)
    if mapping is None:
        raise ValueError(f"不支持的 ibrav: {ibrav}")
    return list(mapping)


def _celldm2abc(ibrav: int, celldm: Sequence[float]) -> Tuple[float, float, float, float, float, float]:
    a = celldm[0] * BOHR_RADIUS_ANGS
    b = celldm[0] * celldm[1] * BOHR_RADIUS_ANGS
    c = celldm[0] * celldm[2] * BOHR_RADIUS_ANGS
    if ibrav in (14, 0):
        cosbc, cosac, cosab = celldm[3], celldm[4], celldm[5]
    elif ibrav in (-12, -13):
        cosab, cosac, cosbc = 0.0, celldm[4], 0.0
    elif ibrav in (-5, 5, 12, 13):
        cosab, cosac, cosbc = celldm[3], 0.0, 0.0
    else:
        cosab, cosac, cosbc = 0.0, 0.0, 0.0
    return float(a), float(b), float(c), float(cosab), float(cosac), float(cosbc)


def _format_float(value: float) -> str:
    return f"{value:.12g}"
