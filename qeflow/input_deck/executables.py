#!/usr/bin/env python3
from typing import Any, Dict

from public.executables import resolve_qe_command


def build_qe_commands(config: Dict[str, Any]) -> Dict[str, str]:
    base = config.get("qe", {}).get("executable_path")
    return {
        "pw": resolve_qe_command(base, "pw.x"),
        "ph": resolve_qe_command(base, "ph.x"),
        "q2r": resolve_qe_command(base, "q2r.x"),
        "matdyn": resolve_qe_command(base, "matdyn.x"),
    }

