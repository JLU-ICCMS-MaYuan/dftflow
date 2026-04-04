#!/usr/bin/env python3
import copy
import os
from typing import Any, Dict, Iterable

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


def filter_config_by_allowed_keys(input_config: Dict[str, Any], allowed_keys: Iterable[str]) -> Dict[str, Any]:
    filtered: Dict[str, Any] = {}
    allowed = set(allowed_keys)
    for key, value in input_config.items():
        if key in allowed:
            filtered[key] = copy.deepcopy(value)
    return filtered


def build_cli_config(template: Dict[str, Any], input_config: Dict[str, Any], allowed_keys: Iterable[str]) -> Dict[str, Any]:
    filtered = filter_config_by_allowed_keys(input_config, allowed_keys)
    return merge_parameter_template(template, filtered)


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
