#!/usr/bin/env python3
import os
from typing import List

from public.job import write_job_script


def write_root_run_script(
    workspace: str,
    script_name: str,
    slurm_header: str,
    commands: List[str],
    announce: bool = True,
) -> str:
    path = os.path.join(workspace, script_name)
    write_job_script(path, slurm_header, commands, announce=announce)
    return path


def write_split_run_script(q_dir: str, slurm_header: str, pw_cmd: str, ph_cmd: str, announce: bool = True) -> str:
    path = os.path.join(q_dir, "run_split.sh")
    write_job_script(
        path,
        slurm_header,
        [
            f"{pw_cmd} < scf.in > scf.out",
            f"{ph_cmd} < split_ph.in > split_ph.out",
        ],
        announce=announce,
    )
    return path
