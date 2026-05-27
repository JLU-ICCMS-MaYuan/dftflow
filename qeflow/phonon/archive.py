#!/usr/bin/env python3
import os
import shlex
import shutil
from typing import List, Sequence

from qeflow.phonon.context import PhononStepContext
from qeflow.input_deck.filenames import derive_filenames


PHBAND_DIR = "phband"
PHDOS_DIR = "phdos"


def phband_archive_files(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [
        os.path.join(PHBAND_DIR, names["flfrq"]),
        os.path.join(PHBAND_DIR, f"{prefix}_band.labelinfo.dat"),
    ]


def phdos_archive_files(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [os.path.join(PHDOS_DIR, names["fldos"])]


def phonon_plot_files() -> List[str]:
    return [
        os.path.join(PHBAND_DIR, "phonon_band.csv"),
        os.path.join(PHBAND_DIR, "phonon_band_labels.csv"),
        os.path.join(PHBAND_DIR, "phonon_band.png"),
        os.path.join(PHDOS_DIR, "phonon_dos_atoms.csv"),
        os.path.join(PHDOS_DIR, "phonon_dos_elements.csv"),
        os.path.join(PHDOS_DIR, "phonon_dos.png"),
    ]


def collect_phband_outputs(context: PhononStepContext) -> None:
    names = derive_filenames(context.prefix)
    _copy_required(
        context.workspace,
        PHBAND_DIR,
        [
            names["flfrq"],
            f"{context.prefix}_band.labelinfo.dat",
        ],
    )


def collect_phdos_outputs(context: PhononStepContext) -> None:
    names = derive_filenames(context.prefix)
    _copy_required(context.workspace, PHDOS_DIR, [names["fldos"]])


def build_collect_phband_commands(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [
        f"mkdir -p {shlex.quote(PHBAND_DIR)}",
        f"cp -f {shlex.quote(names['flfrq'])} {shlex.quote(PHBAND_DIR)}/",
        f"cp -f {shlex.quote(prefix + '_band.labelinfo.dat')} {shlex.quote(PHBAND_DIR)}/",
    ]


def build_collect_phdos_commands(prefix: str) -> List[str]:
    names = derive_filenames(prefix)
    return [
        f"mkdir -p {shlex.quote(PHDOS_DIR)}",
        f"cp -f {shlex.quote(names['fldos'])} {shlex.quote(PHDOS_DIR)}/",
    ]


def root_phband_outputs_exist(context: PhononStepContext) -> bool:
    names = derive_filenames(context.prefix)
    return os.path.exists(os.path.join(context.workspace, names["flfrq"])) and os.path.exists(
        os.path.join(context.workspace, f"{context.prefix}_band.labelinfo.dat")
    )


def root_phdos_outputs_exist(context: PhononStepContext) -> bool:
    return os.path.exists(os.path.join(context.workspace, derive_filenames(context.prefix)["fldos"]))


def _copy_required(workspace: str, target_dir_name: str, filenames: Sequence[str]) -> None:
    target_dir = os.path.join(workspace, target_dir_name)
    os.makedirs(target_dir, exist_ok=True)
    missing = [filename for filename in filenames if not os.path.exists(os.path.join(workspace, filename))]
    if missing:
        raise FileNotFoundError("缺少待归档文件: " + ", ".join(missing))
    for filename in filenames:
        shutil.copy2(os.path.join(workspace, filename), os.path.join(target_dir, filename))
