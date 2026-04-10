import re
from dataclasses import dataclass
from typing import Sequence, Tuple


@dataclass(frozen=True)
class InteractionSide:
    element: str
    orbitals: Tuple[str, ...]


@dataclass(frozen=True)
class InteractionSpec:
    raw: str
    left: InteractionSide
    right: InteractionSide
    label: str
    filename_label: str


def parse_interaction_side(side_text: str) -> InteractionSide:
    if ":" not in side_text:
        raise ValueError(f"相互作用片段格式错误: {side_text}。应为 Element:orb1,orb2")
    element, orbitals_text = side_text.split(":", 1)
    element = element.strip()
    orbitals = tuple(orb.strip() for orb in orbitals_text.split(",") if orb.strip())
    if not element or not orbitals:
        raise ValueError(f"相互作用片段格式错误: {side_text}。元素和轨道都不能为空。")
    return InteractionSide(element=element, orbitals=orbitals)


def sanitize_label(text: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9]+", "-", text).strip("-")
    return safe or "interaction"


def parse_interaction_spec(spec_text: str) -> InteractionSpec:
    parts = spec_text.split("|")
    if len(parts) != 2:
        raise ValueError(
            f"interaction 格式错误: {spec_text}。应为 ElementA:orb1,orb2|ElementB:orb1,orb2"
        )
    left = parse_interaction_side(parts[0].strip())
    right = parse_interaction_side(parts[1].strip())
    label = f"{left.element}:{','.join(left.orbitals)} | {right.element}:{','.join(right.orbitals)}"
    filename_label = (
        f"{sanitize_label(left.element + '_' + '-'.join(left.orbitals))}__"
        f"{sanitize_label(right.element + '_' + '-'.join(right.orbitals))}"
    )
    return InteractionSpec(
        raw=spec_text,
        left=left,
        right=right,
        label=label,
        filename_label=filename_label,
    )


def extract_element(atom_label: str) -> str:
    match = re.match(r"([A-Za-z]+)", atom_label)
    return match.group(1) if match else atom_label


def orbital_matches(requested: str, orbital_label: str) -> bool:
    if requested == orbital_label:
        return True
    if "_" not in requested:
        return orbital_label.startswith(f"{requested}_") or orbital_label == requested
    return False


def side_matches_label(side: InteractionSide, basis_label: str) -> bool:
    if "_" not in basis_label:
        return False
    atom_label, orbital_label = basis_label.split("_", 1)
    if extract_element(atom_label) != side.element:
        return False
    return any(orbital_matches(orbital, orbital_label) for orbital in side.orbitals)


def match_basis_indices(labels: Sequence[str], side: InteractionSide):
    return [idx for idx, label in enumerate(labels) if side_matches_label(side, label)]


def orbital_pair_matches(interaction: InteractionSpec, left_label: str, right_label: str) -> bool:
    return side_matches_label(interaction.left, left_label) and side_matches_label(interaction.right, right_label)
