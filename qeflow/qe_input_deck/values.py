#!/usr/bin/env python3
import re
from typing import Any


def to_qe_value(value: Any) -> str:
    if isinstance(value, bool):
        return ".true." if value else ".false."
    if isinstance(value, (int, float)):
        return str(value)
    text = str(value)
    if text.startswith("'") or text.startswith('"'):
        return text
    if text.lower() in {".true.", ".false."}:
        return text.lower()
    if text.lower() in {"true", "false"}:
        return ".true." if text.lower() == "true" else ".false."
    if re.fullmatch(r"[+-]?(\d+(\.\d*)?|\.\d+)([deDE][+-]?\d+)?", text):
        return text
    return f"'{text}'"

