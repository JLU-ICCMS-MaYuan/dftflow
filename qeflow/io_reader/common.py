#!/usr/bin/env python3
import os
import re
from typing import Iterable, List, Sequence


ERROR_MARKERS = (
    "%%%%%%%%%%%%%%",
    "error in routine",
    "error:",
    "stopping ...",
    "convergence not achieved",
)


def is_float(text: str) -> bool:
    try:
        float(text)
        return True
    except ValueError:
        return False


def read_text_if_exists(path: str) -> str:
    if not os.path.exists(path):
        return ""
    with open(path, "r", encoding="utf-8", errors="ignore") as handle:
        return handle.read()


def has_error_marker(text: str, extra_markers: Iterable[str] = ()) -> bool:
    lowered = text.lower()
    markers = tuple(marker.lower() for marker in ERROR_MARKERS) + tuple(
        marker.lower() for marker in extra_markers
    )
    return any(marker in lowered for marker in markers)


def find_first_file(directory: str, patterns: Sequence[str]) -> str:
    for pattern in patterns:
        matches = [name for name in os.listdir(directory) if re.fullmatch(pattern, name)]
        if matches:
            return os.path.join(directory, sorted(matches)[0])
    return ""


def float_values(line: str) -> List[float]:
    return [float(item) for item in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[Ee][-+]?\d+)?", line)]

