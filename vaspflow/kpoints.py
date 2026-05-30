#!/usr/bin/env python3
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


@dataclass(frozen=True)
class AutomaticKpoints:
    title: str
    style: str
    mesh: tuple[int, int, int]
    shift: tuple[float, float, float]


def read_automatic_kpoints(path: str | Path) -> AutomaticKpoints:
    """读取 VASP automatic mesh KPOINTS。"""
    kpoints_path = Path(path)
    lines = kpoints_path.read_text(encoding="utf-8").splitlines()
    if len(lines) < 4:
        raise ValueError(f"{kpoints_path} 不是完整的 automatic mesh KPOINTS")

    try:
        generation_mode = int(lines[1].split()[0])
    except (IndexError, ValueError) as exc:
        raise ValueError(f"{kpoints_path} 第二行不是 automatic mesh 标记") from exc
    if generation_mode != 0:
        raise ValueError(f"{kpoints_path} 不是 automatic mesh KPOINTS")

    style = lines[2].strip() or "Gamma"
    style_lower = style.lower()
    if not (style_lower.startswith("g") or style_lower.startswith("m")):
        raise ValueError(f"{kpoints_path} 第三行不是 Gamma 或 Monkhorst-Pack")

    try:
        mesh = tuple(int(value) for value in lines[3].split()[:3])
    except ValueError as exc:
        raise ValueError(f"{kpoints_path} 第四行不是三维整数网格") from exc
    if len(mesh) != 3 or any(value <= 0 for value in mesh):
        raise ValueError(f"{kpoints_path} 第四行不是有效的三维正整数网格")

    if len(lines) >= 5 and lines[4].split():
        try:
            shift = tuple(float(value) for value in lines[4].split()[:3])
        except ValueError as exc:
            raise ValueError(f"{kpoints_path} 第五行不是三维平移量") from exc
        if len(shift) != 3:
            raise ValueError(f"{kpoints_path} 第五行不是三维平移量")
    else:
        shift = (0.0, 0.0, 0.0)

    return AutomaticKpoints(lines[0].strip() or "Automatic mesh", style, mesh, shift)


def write_automatic_kpoints(path: str | Path, kpoints: AutomaticKpoints) -> None:
    """写出 VASP automatic mesh KPOINTS。"""
    kpoints_path = Path(path)
    shift = " ".join(_format_number(value) for value in kpoints.shift)
    kpoints_path.write_text(
        "\n".join(
            [
                kpoints.title,
                "0",
                kpoints.style,
                f"{kpoints.mesh[0]} {kpoints.mesh[1]} {kpoints.mesh[2]}",
                shift,
            ]
        )
        + "\n",
        encoding="utf-8",
    )


def derive_integer_kmesh_multiplier(scf_kmesh: float, target_kmesh: float) -> int:
    """按 kmesh 密度比推导用于网格点数的整数倍。"""
    scf_value = float(scf_kmesh)
    target_value = float(target_kmesh)
    if scf_value <= 0 or target_value <= 0:
        raise ValueError("kmesh 必须为正数")

    ratio = scf_value / target_value
    if ratio <= 1:
        return 1

    nearest = round(ratio)
    if math.isclose(ratio, nearest, rel_tol=1e-8, abs_tol=1e-8):
        return max(1, int(nearest))
    return max(1, math.ceil(ratio))


def expected_scaled_mesh(mesh: Sequence[int], multiplier: int) -> tuple[int, int, int]:
    if len(mesh) != 3:
        raise ValueError("mesh 必须是三维网格")
    return tuple(int(value) * multiplier for value in mesh)


def _format_number(value: float) -> str:
    if float(value).is_integer():
        return str(int(value))
    return f"{value:g}"
