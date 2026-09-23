"""Configuration loading for the scflow QE post-processing CLI."""

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Union

from public.config_templates import load_config_file


@dataclass(frozen=True)
class ScflowConfig:
    config_file: Path
    work_dir: Path
    results_dir: Path
    smoothing_thz: float = 3.0
    resolution: int = 1000
    gaussian_sigma: float = 5.0
    mu: float = 0.1
    structure_tolerance: float = 0.2
    integration: str = "sum"


def _as_number(section: Dict[str, Any], key: str, default: Any, converter):
    value = section.get(key, default)
    try:
        return converter(value)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"[scflow].{key} 必须是数值，当前值为: {value!r}") from exc


def load_scflow_config(path: Union[str, Path]) -> ScflowConfig:
    config_file = Path(path).expanduser().resolve()
    raw = load_config_file(str(config_file))
    section = raw.get("scflow", {})
    if not isinstance(section, dict):
        raise ValueError("[scflow] 必须是 TOML 表")

    base_dir = config_file.parent
    work_dir = Path(section.get("work_dir", ".")).expanduser()
    if not work_dir.is_absolute():
        work_dir = base_dir / work_dir
    work_dir = work_dir.resolve()

    results_value = section.get("results_dir")
    results_dir = work_dir / "results" if results_value is None else Path(results_value).expanduser()
    if not results_dir.is_absolute():
        results_dir = base_dir / results_dir
    results_dir = results_dir.resolve()

    integration = section.get("integration", "sum")
    if integration not in {"sum", "simps"}:
        raise ValueError("[scflow].integration 只能是 'sum' 或 'simps'")

    resolution = _as_number(section, "resolution", 1000, int)
    if resolution < 0:
        raise ValueError("[scflow].resolution 不能小于 0")

    return ScflowConfig(
        config_file=config_file,
        work_dir=work_dir,
        results_dir=results_dir,
        smoothing_thz=_as_number(section, "smoothing_thz", 3.0, float),
        resolution=resolution,
        gaussian_sigma=_as_number(section, "gaussian_sigma", 5.0, float),
        mu=_as_number(section, "mu", 0.1, float),
        structure_tolerance=_as_number(section, "structure_tolerance", 0.2, float),
        integration=integration,
    )
