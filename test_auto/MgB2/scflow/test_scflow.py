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


def test_native_qe_files_are_discovered_without_matdyn_outputs(tmp_path):
    from scflow.qe_outputs import Folder

    epc = tmp_path / "elph_dir"
    epc.mkdir()
    for name in ("elph.inp_lambda.1", "elph.inp_lambda.2", "elph.inp_lambda.1.bak"):
        (epc / name).touch()
    for name in ("Si8P16.dyn0", "Si8P16.dyn1", "Si8P16.dyn2", "matdyn.out",
                 "split_ph.out.1", "split_ph.out.2", "phonodos.out"):
        (tmp_path / name).touch()
    (tmp_path / "old.dyn3").mkdir()

    folder = Folder(str(tmp_path))
    assert {Path(p).name for p in folder.dyn_paths} == {"Si8P16.dyn1", "Si8P16.dyn2"}
    assert {Path(p).name for p in folder.dyn_elph_paths} == {"elph.inp_lambda.1", "elph.inp_lambda.2"}
    assert {Path(p).name for p in folder.ph_outs_paths} == {"split_ph.out.1", "split_ph.out.2"}


@pytest.mark.parametrize("smoothing", [0, 3])
def test_exact_zero_modes_with_zero_lambda_have_zero_contribution(smoothing):
    from types import SimpleNamespace
    from scflow.constants import k_Ry_THz
    from scflow.sc_direct import a2f_direct, lambdas_direct

    freqs = np.array([0., 4., 8.])
    epc = SimpleNamespace(sqr_freqs=(freqs / k_Ry_THz)**2,
                          lambdas=np.array([0., .4, .6]), gammas=np.array([1., 2., 3.]), DOS=10.)
    qpoints = [{"DynElph": epc, "weight": 1.}]
    with np.errstate(all="raise"):
        spectrum = a2f_direct(qpoints, smoothing)
        coupling = lambdas_direct(qpoints, smoothing)
    for result in (spectrum, coupling):
        for values in result.values():
            assert np.isfinite(values).all()
    assert spectrum["freqs, THz"][0] == 0
    assert spectrum["a2f (gamma), THz"][0] == 0
    assert coupling["lambdas (gamma)"][0] == 0
    assert coupling["lambdas (lambda)"][1] == pytest.approx(.4 * np.exp(-(smoothing / 4)**2))
    assert epc.gammas[0] == 1.  # Raw arrays are never repaired or overwritten.


def test_exact_zero_mode_with_nonzero_lambda_is_rejected():
    from types import SimpleNamespace
    from scflow.sc_direct import lambdas_direct

    epc = SimpleNamespace(sqr_freqs=np.array([0., 1e-6]),
                          lambdas=np.array([.1, .5]), gammas=np.array([1., 2.]), DOS=10.)
    with pytest.raises(ValueError, match="zero-frequency"):
        lambdas_direct([{"DynElph": epc, "weight": 1.}], 3)


def test_system_calculation_state_is_not_shared_between_broadenings():
    from scflow.system import System

    first = System([], [])
    second = System([], [])
    first.direct["sentinel"] = np.array([1.])
    first.a2f["sentinel"] = np.array([2.])
    assert "sentinel" not in second.direct
    assert "sentinel" not in second.a2f


def test_tc_formula_does_not_compute_prefactor_in_float16():
    import math
    from scflow.tc import tc_mcm_formula

    wlog = np.array([311.], dtype=np.float16)
    coupling = np.array([.41650399565696716], dtype=np.float32)
    lam = float(coupling[0])
    expected = 311. / 1.2 * math.exp(-1.04 * (1 + lam) / (lam - .1 * (1 + .62 * lam)))
    assert tc_mcm_formula(wlog, coupling, .1)[0] == pytest.approx(expected, rel=1e-12)


def test_eliashberg_result_explicitly_reports_search_floor():
    from scflow.main import eliashberg_solver_metadata
    from types import SimpleNamespace

    floor = eliashberg_solver_metadata(SimpleNamespace(tc_e=.001, dim=24))
    assert floor["status"] == "resolution_limit"
    assert floor["matrix_size"] == 25
    assert eliashberg_solver_metadata(SimpleNamespace(tc_e=2.0, dim=24))["status"] == "resolved"
