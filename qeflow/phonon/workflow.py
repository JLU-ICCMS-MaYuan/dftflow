#!/usr/bin/env python3
from typing import Optional

from qeflow.phonon._shared import create_context, load_q_index_entries, refresh_workflow_status
from qeflow.phonon.qe_matdyn_phband import run_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import run_matdyn_phdos_step
from qeflow.phonon.qe_ph_nosplit import run_ph_nosplit_step, run_ph_to_dyn0
from qeflow.phonon.qe_phonon_nosplit import run_phonon_nosplit_workflow
from qeflow.phonon.qe_phonon_split_assign_coords import run_phonon_split_assign_coords_workflow
from qeflow.phonon.qe_phonon_split_assign_index import run_phonon_split_assign_index_workflow
from qeflow.phonon.qe_q2r import run_q2r_step
from qeflow.phonon.split_prepare import prepare_split_assign_coords_step, prepare_split_assign_index_step


def run_standalone_ph(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, dyn0_only: bool) -> int:
    return run_ph_nosplit_step(config_file, struct_file, run_calc, submit_job, dyn0_only)


def run_standalone_q2r(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool) -> int:
    return run_q2r_step(config_file, struct_file, run_calc, submit_job)


def run_standalone_matdyn(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, target: str) -> int:
    if target == "phband":
        return run_matdyn_phband_step(config_file, struct_file, run_calc, submit_job)
    return run_matdyn_phdos_step(config_file, struct_file, run_calc, submit_job)


def materialize_split_directories(workspace: str, config: dict, prefix: str, mode: str) -> int:
    config_file = str(config.get("__config_file__", "")).strip()
    if not config_file:
        raise ValueError("缺少配置真源路径，无法物化 split q 点目录。")
    context = create_context(config_file, None)
    context.config = config
    context.prefix = prefix
    if mode == "split_assign_coords":
        return prepare_split_assign_coords_step(context)
    return prepare_split_assign_index_step(context)


def _load_q_index_entries(workspace: str):
    return load_q_index_entries(workspace)


def run_phonon_workflow(config_file: str, struct_file: Optional[str], run_calc: bool, submit_job: bool, mode: str) -> int:
    if mode == "nosplit":
        return run_phonon_nosplit_workflow(config_file, struct_file, run_calc, submit_job)
    if mode == "split_assign_coords":
        return run_phonon_split_assign_coords_workflow(config_file, struct_file, run_calc, submit_job)
    return run_phonon_split_assign_index_workflow(config_file, struct_file, run_calc, submit_job)
