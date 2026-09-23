import json
import shutil
from pathlib import Path

import numpy as np
import pytest
import tomllib


def test_load_scflow_config_resolves_paths_and_defaults(tmp_path):
    from scflow.config import load_scflow_config

    config_file = tmp_path / "input.toml"
    config_file.write_text(
        """
[scflow]
work_dir = "qe_epc"
results_dir = "results"
mu = 0.13
""",
        encoding="utf-8",
    )

    config = load_scflow_config(config_file)

    assert config.work_dir == (tmp_path / "qe_epc").resolve()
    assert config.results_dir == (tmp_path / "results").resolve()
    assert config.smoothing_thz == 3.0
    assert config.resolution == 1000
    assert config.gaussian_sigma == 5.0
    assert config.mu == 0.13
    assert config.integration == "sum"


def test_simpson_compatibility_integrates_with_current_scipy():
    from scflow.compat import simpson

    x = np.linspace(0.0, 1.0, 101)
    assert simpson(x**2, x=x) == pytest.approx(1.0 / 3.0)


def test_scflow_console_script_is_registered():
    pyproject = tomllib.loads((Path(__file__).parents[2] / "pyproject.toml").read_text(encoding="utf-8"))
    assert pyproject["project"]["scripts"]["scflow"] == "scflow.main:main"


def test_scflow_cli_processes_qe_archive_and_writes_tc_outputs(tmp_path):
    from scflow.main import main

    archive = Path(__file__).parents[2] / "external" / "a2f" / "examples-public" / "La2H20"
    config_file = tmp_path / "input.toml"
    results_dir = tmp_path / "results"
    config_file.write_text(
        f"""
[scflow]
work_dir = {str(archive)!r}
results_dir = {str(results_dir)!r}
resolution = 32
gaussian_sigma = 1.0
""",
        encoding="utf-8",
    )

    main(["-c", str(config_file)])

    result = json.loads((results_dir / "result.json").read_text(encoding="utf-8"))
    assert result["Tc, K"]["Eliashberg"] >= 0
    assert result["Tc, K"]["Allen-Dynes"]["a2f"]["gamma"] >= 0
    assert list(results_dir.glob("direct_s*.csv"))
    assert list(results_dir.glob("a2f_s*.csv"))


def test_scflow_cli_accepts_archive_without_ph_out(tmp_path):
    from scflow.main import main

    source = Path(__file__).parents[2] / "external" / "a2f" / "examples-public" / "La2H20"
    archive = tmp_path / "qe_epc"
    archive.mkdir()
    for path in source.glob("*.dyn*.elph*"):
        shutil.copy2(path, archive / path.name)

    config_file = tmp_path / "input.toml"
    results_dir = tmp_path / "results"
    config_file.write_text(
        f"""
[scflow]
work_dir = {str(archive)!r}
results_dir = {str(results_dir)!r}
resolution = 8
gaussian_sigma = 1.0
""",
        encoding="utf-8",
    )

    main(["-c", str(config_file)])

    result = json.loads((results_dir / "result.json").read_text(encoding="utf-8"))
    assert result["Hc2, T"] is None
