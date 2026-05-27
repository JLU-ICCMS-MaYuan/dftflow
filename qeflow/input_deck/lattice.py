#!/usr/bin/env python3
from typing import Any, Dict, List, Sequence, Tuple

import numpy as np

from qeflow.crystal.lattice import BOHR_RADIUS_ANGS, resolve_lattice_parameters


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
