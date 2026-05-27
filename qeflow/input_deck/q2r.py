#!/usr/bin/env python3
import copy
import os
from typing import Any, Dict

from qeflow.input_deck.filenames import derive_filenames
from qeflow.input_deck.values import to_qe_value


def write_q2r_input(workspace: str, config: Dict[str, Any], prefix: str) -> str:
    q2r_cfg = copy.deepcopy(config["qe_phonon"]["q2r"])
    names = derive_filenames(prefix)
    path = os.path.join(workspace, "q2r.in")
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("&input\n")
        handle.write(f"  fildyn = {to_qe_value(names['fildyn'])}\n")
        handle.write(f"  flfrc = {to_qe_value(names['flfrc'])}\n")
        for key, value in q2r_cfg.items():
            if value is not None:
                handle.write(f"  {key} = {to_qe_value(value)}\n")
        handle.write("/\n")
    return path
