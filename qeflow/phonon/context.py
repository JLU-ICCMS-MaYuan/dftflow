#!/usr/bin/env python3
import os
from dataclasses import dataclass
from typing import Dict, Optional

from qeflow.phonon.workspace import resolve_workspace
from qeflow.input_deck.executables import build_qe_commands
from qeflow.input_deck.prefix import resolve_prefix
from qeflow.crystal.poscar import parse_poscar


@dataclass
class PhononStepContext:
    config: Dict[str, object]
    config_file: str
    workspace: str
    prefix: str
    struct_info: Dict[str, object]
    commands: Dict[str, str]


def create_context(
    config_file: str,
    struct_file: Optional[str],
    workflow_name: Optional[str] = None,
    prepare_inputs: bool = True,
) -> PhononStepContext:
    config, _root_dir, workspace = resolve_workspace(
        config_file,
        struct_file,
        prepare_inputs=prepare_inputs,
        workflow_name=workflow_name,
    )
    config["__config_file__"] = os.path.abspath(config_file)
    struct_info = parse_poscar(os.path.join(workspace, "POSCAR"))
    prefix = resolve_prefix(config, struct_info)
    commands = build_qe_commands(config)
    return PhononStepContext(
        config=config,
        config_file=os.path.abspath(config_file),
        workspace=workspace,
        prefix=prefix,
        struct_info=struct_info,
        commands=commands,
    )
