#!/usr/bin/env python3
import os
import re
from dataclasses import dataclass
from typing import List, Optional

import numpy as np

from qeflow.qe_io_reader.common import has_error_marker, read_text_if_exists


@dataclass(frozen=True)
class PwOutputData:
    path: str
    finished: bool
    has_error: bool
    fermi_energy_ev: Optional[float]
    total_energy_ry: Optional[float]
    number_of_atoms: Optional[int]
    number_of_kpoints: Optional[int]
    cell_angstrom: Optional[np.ndarray]


def read_pw_output(path: str) -> PwOutputData:
    text = read_text_if_exists(path)
    return PwOutputData(
        path=path,
        finished="JOB DONE" in text,
        has_error=bool(text) and has_error_marker(text),
        fermi_energy_ev=_first_float(text, r"the Fermi energy is\s+([-+]?\d+(?:\.\d+)?)\s+ev"),
        total_energy_ry=_first_float(text, r"!\s+total energy\s+=\s+([-+]?\d+(?:\.\d+)?)\s+Ry"),
        number_of_atoms=_first_int(text, r"number of atoms/cell\s*=\s*(\d+)"),
        number_of_kpoints=_first_int(text, r"number of k points\s*=\s*(\d+)"),
        cell_angstrom=_read_cell_angstrom(text),
    )


def read_scf_output(path: str) -> PwOutputData:
    return read_pw_output(path)


def read_nscf_output(path: str) -> PwOutputData:
    return read_pw_output(path)


def read_eband_output(path: str) -> PwOutputData:
    return read_pw_output(path)


@dataclass
class ScfOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return read_text_if_exists(self.path)

    def is_finished(self) -> bool:
        return read_pw_output(self.path).finished

    def has_error(self) -> bool:
        return read_pw_output(self.path).has_error


@dataclass
class RelaxOut(ScfOut):
    def is_finished(self) -> bool:
        text = self.text()
        return "JOB DONE" in text and "End final coordinates" in text

    def has_error(self) -> bool:
        text = self.text()
        return bool(text) and has_error_marker(text, extra_markers=("bfgs failed",))


def _first_float(text: str, pattern: str) -> Optional[float]:
    match = re.search(pattern, text, flags=re.IGNORECASE)
    return float(match.group(1)) if match else None


def _first_int(text: str, pattern: str) -> Optional[int]:
    match = re.search(pattern, text, flags=re.IGNORECASE)
    return int(match.group(1)) if match else None


def _read_cell_angstrom(text: str) -> Optional[np.ndarray]:
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if "CELL_PARAMETERS" not in line.upper():
            continue
        rows: List[List[float]] = []
        for offset in range(1, 4):
            if index + offset >= len(lines):
                return None
            parts = lines[index + offset].split()
            if len(parts) < 3:
                return None
            try:
                rows.append([float(parts[0]), float(parts[1]), float(parts[2])])
            except ValueError:
                return None
        return np.array(rows, dtype=float)
    return None

