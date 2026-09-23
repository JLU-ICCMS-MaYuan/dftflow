"""Explicit real-archive acceptance; does not run during ordinary pytest discovery."""
import argparse
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys

import numpy as np

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))
from scflow.qe_outputs import Dyn, DynElph, Folder
from scflow.utils import compare_q_points


def sha256(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n")


def assert_finite(value):
    if isinstance(value, dict):
        for item in value.values():
            assert_finite(item)
    elif isinstance(value, list):
        for item in value:
            assert_finite(item)
    elif isinstance(value, (int, float)):
        assert math.isfinite(value), value


def validate(archive, output):
    output.mkdir(parents=True, exist_ok=False)
    folder = Folder(str(archive))
    inputs = [Path(p) for p in folder.dyn_paths + folder.dyn_elph_paths + folder.ph_outs_paths]
    inputs.append(archive / "lambda.in")
    assert float((archive / "lambda.in").read_text().splitlines()[-1]) == .16
    before = {str(p): sha256(p) for p in inputs}
    epc = [DynElph(p) for p in folder.dyn_elph_paths]
    dyn = [Dyn(p) for p in folder.dyn_paths]
    assert len(epc) == len(dyn) == 16
    assert len({e.q_point for e in epc}) == 16
    expected = np.arange(1, 11) * .005
    for e in epc:
        np.testing.assert_allclose(e.headers, expected, rtol=0, atol=1e-12)
        assert len(e.sqr_freqs) == 72
        assert not (e.sqr_freqs < 0).any()
        assert sum(compare_q_points(e.q_point, d.q_point) for d in dyn) == 1
    assert sum(d.weight for d in dyn) == 36
    assert sum(int(np.count_nonzero(e.sqr_freqs == 0)) for e in epc) == 2
    write_json(output / "source_hashes.json", before)

    env = dict(os.environ, PYTHONPATH=str(REPO), MPLBACKEND="Agg",
               OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1", MKL_NUM_THREADS="1")
    rows, executions = [], []
    for mu in (.1, .16):
        work = output / f"mu{mu:.2f}"
        work.mkdir()
        results = work / "results"
        config = work / "input.toml"
        config.write_text(
            f'[scflow]\nwork_dir = "{archive}"\nresults_dir = "{results}"\n'
            f'mu = {mu}\nsmoothing_thz = 3.0\nresolution = 1000\n'
            'gaussian_sigma = 5.0\nintegration = "sum"\n'
        )
        command = [sys.executable, "-m", "scflow.main", "-c", str(config)]
        with (work / "run.log").open("w") as log:
            process = subprocess.run(command, cwd=REPO, env=env, stdout=log,
                                     stderr=subprocess.STDOUT, timeout=900)
        executions.append(dict(command=command, returncode=process.returncode))
        write_json(output / "executions.json", executions)
        assert process.returncode == 0, f"See {work / 'run.log'}"
        log = (work / "run.log").read_text()
        assert "RuntimeWarning" not in log and "Traceback" not in log, log[-3000:]
        result_paths = sorted(results.glob("*/result.json"))
        assert len(result_paths) == 10
        assert list(results.glob("*.cif")) and list(results.glob("*.vasp"))
        summaries = list(results.glob("summary_*.csv"))
        assert len(summaries) == 1
        assert len(list(csv.DictReader(summaries[0].open()))) == 10
        assert len(list(results.glob("summary_*.pdf"))) == 1
        for p in result_paths:
            result = json.loads(p.read_text(), parse_constant=lambda v: (_ for _ in ()).throw(ValueError(v)))
            assert_finite(result)
            assert result["formula"] == "SiP2"
            assert result["mu"] == mu
            expected_status = "resolution_limit" if result["Tc, K"]["Eliashberg"] <= .001 else "resolved"
            assert result["eliashberg_solver"]["status"] == expected_status
            modes = result["zero_frequency_handling"]["modes"]
            assert len(modes) == 2 and all(m["lambda"] == 0 for m in modes)
            assert {m["mode_1based"] for m in modes} == {1, 2}
            assert all(m["gamma_GHz"] > 0 for m in modes)
            assert len(list(p.parent.glob("*.pdf"))) == 2
            for artifact in p.parent.glob("*.pdf"):
                assert artifact.read_bytes().startswith(b"%PDF") and artifact.stat().st_size > 1000
            for route in ("direct", "a2f"):
                tables = list(p.parent.glob(f"{route}_*.csv"))
                assert len(tables) == 1
                table = np.genfromtxt(tables[0], delimiter=",", skip_header=1)
                assert np.isfinite(table).all()
                assert len(table) == (1152 if route == "direct" else 2050)
                if route == "direct":
                    assert (table[:, 0] == 0).sum() == 2
            a2f_path = next(p.parent.glob("a2f_*.csv"))
            spectrum = list(csv.DictReader(a2f_path.open()))
            last = spectrum[-1]
            lam = float(last["lambda (gamma)"])
            wlog = float(last["wlog (gamma), K"])
            tc = wlog / 1.2 * math.exp(-1.04 * (1 + lam) / (lam - mu * (1 + .62 * lam)))
            np.testing.assert_allclose(float(last["Tc_McM (gamma), K"]), tc, rtol=3e-6, atol=1e-7)
            assert abs(result["Tc, K"]["McMillan"]["a2f"]["gamma"] - tc) < .00051
            w2 = float(last["w2 (gamma), K"])
            f1 = (1 + (lam / (2.46 * (1 + 3.8 * mu)))**1.5)**(1 / 3)
            f2 = 1 - lam**2 * (1 - w2 / wlog) / (lam**2 + 3.312 * (1 + 6.3 * mu)**2)
            np.testing.assert_allclose(float(last["Tc_AD (gamma), K"]), f1 * f2 * tc, rtol=3e-6, atol=1e-7)
            rows.append(dict(mu=mu, broadening_ry=result["Broadening, Ry"], lambda_a2f=lam,
                             wlog_a2f_k=wlog, tc_mcm_k=result["Tc, K"]["McMillan"]["a2f"]["gamma"],
                             tc_ad_k=result["Tc, K"]["Allen-Dynes"]["a2f"]["gamma"],
                             tc_eliashberg_k=result["Tc, K"]["Eliashberg"],
                             eliashberg_status=expected_status, excluded_zero_modes=len(modes)))
        print(f"Verified mu={mu}: 10 broadenings, CLI/JSON/CSV/PDF/structure/formula checks passed", flush=True)
    assert before == {str(p): sha256(p) for p in inputs}
    source_code = {str(p.relative_to(REPO)): sha256(p) for p in (REPO / "scflow").glob("*.py")}
    write_json(output / "validation.json", dict(
        passed=True, archive=str(archive), input_files=len(inputs), results=len(rows),
        input_hashes_unchanged=True, q_points=16, total_weight=36, modes_per_q=72,
        source_code_sha256=source_code, executions=executions))
    rows.sort(key=lambda row: (row["mu"], row["broadening_ry"]))
    with (output / "results.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    validate(args.archive.resolve(), args.output.resolve())
