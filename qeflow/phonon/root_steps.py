#!/usr/bin/env python3
from qeflow.phonon.context import PhononStepContext
from qeflow.input_deck.pw import write_phonon_scf_input
from qeflow.input_deck.scripts import write_root_run_script


def prepare_root_scf_step(context: PhononStepContext) -> str:
    write_phonon_scf_input(context.workspace, context.config, context.struct_info, context.prefix)
    return write_root_run_script(
        context.workspace,
        "run_scf.sh",
        str(context.config.get("slurm", {}).get("header", "")),
        [f"{context.commands['pw']} < scf.in > scf.out"],
        announce=False,
    )
