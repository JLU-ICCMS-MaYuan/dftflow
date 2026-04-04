#!/usr/bin/env python3
import copy
import os
from typing import Any, Dict

try:
    import tomllib  # Python 3.11+
except ImportError:
    tomllib = None
    try:
        import toml  # type: ignore
    except ImportError:
        toml = None


def load_config_file(path: str) -> Dict[str, Any]:
    if not os.path.exists(path):
        raise FileNotFoundError(f"找不到配置文件: {path}")

    if path.endswith(".json"):
        import json

        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)

    if tomllib:
        with open(path, "rb") as f:
            return tomllib.load(f)
    if toml:
        return toml.load(path)
    raise ImportError("缺少 tomllib/toml 模块，无法读取配置文件。")


def merge_parameter_template(template: Dict[str, Any], overrides: Dict[str, Any]) -> Dict[str, Any]:
    merged = copy.deepcopy(template)
    return _merge_dict_in_place(merged, overrides)


def _merge_dict_in_place(base: Dict[str, Any], overrides: Dict[str, Any]) -> Dict[str, Any]:
    for key, value in overrides.items():
        if (
            key in base
            and isinstance(base[key], dict)
            and isinstance(value, dict)
        ):
            _merge_dict_in_place(base[key], value)
        elif value is not None:
            base[key] = copy.deepcopy(value)
    return base
