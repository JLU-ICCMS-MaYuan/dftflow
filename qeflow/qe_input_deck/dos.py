#!/usr/bin/env python3
import os
from typing import Any


def write_eledos_inputs(setup: Any) -> None:
    eledos_cfg = setup.cli_config.get("eledos", {})
    outdir_val = setup.control_outdir
    if eledos_cfg.get("outdir"):
        outdir_val = str(eledos_cfg.get("outdir")).strip("'").strip('"')
    fildos = eledos_cfg.get("fildos") or f"{setup.prefix}.tdos"
    filpdos = eledos_cfg.get("filpdos") or f"{setup.prefix}.pdos"
    filproj = eledos_cfg.get("filproj") or f"{setup.prefix}.proj"
    with open(os.path.join(setup.work_dir, "eletdos.in"), "w", encoding="utf-8") as handle:
        handle.write("&dos\n")
        handle.write(f"   prefix = '{setup.prefix}',\n")
        handle.write(f"   outdir = '{outdir_val}',\n")
        handle.write(f"   fildos = '{fildos}',\n")
        handle.write(f"   DeltaE = {eledos_cfg.get('DeltaE')},\n")
        handle.write(f"   emin = {eledos_cfg.get('emin')},\n")
        handle.write(f"   emax = {eledos_cfg.get('emax')}\n")
        handle.write("/\n")
    with open(os.path.join(setup.work_dir, "elepdos.in"), "w", encoding="utf-8") as handle:
        handle.write("&projwfc\n")
        handle.write(f"   prefix = '{setup.prefix}',\n")
        handle.write(f"   outdir = '{outdir_val}',\n")
        handle.write(f"   filpdos = '{filpdos}',\n")
        handle.write(f"   filproj = '{filproj}',\n")
        handle.write(f"   ngauss = {eledos_cfg.get('ngauss')},\n")
        handle.write(f"   DeltaE = {eledos_cfg.get('DeltaE')},\n")
        handle.write(f"   degauss = {eledos_cfg.get('degauss')},\n")
        handle.write(f"   emin = {eledos_cfg.get('emin')},\n")
        handle.write(f"   emax = {eledos_cfg.get('emax')}\n")
        handle.write("/\n")
