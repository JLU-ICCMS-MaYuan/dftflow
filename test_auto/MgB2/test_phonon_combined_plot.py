"""联合声子图：新增产物、共享坐标与旧接口兼容。"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.figure import Figure
import numpy as np
from PIL import Image
import pytest

from qeflow.check.checks import check_phonon_plot
from qeflow.phonon.context import PhononStepContext
from qeflow.phonon import phonon_plot as module


@pytest.fixture
def archive(tmp_path):
    (tmp_path / "phband").mkdir()
    (tmp_path / "phdos").mkdir()
    inputs = {
        "POSCAR": "Si\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n1\nDirect\n0 0 0\n",
        "phband/Si.phband.freq": " &plot nbnd=3, nks=2 /\n0 0 0\n-20 10 30\n0.5 0 0\n-5 15 35\n",
        "phband/Si_band.labelinfo.dat": "GAMMA 1 0 0 0 0\nX 2 0.5 0.5 0 0\n",
        "phdos/Si.phdos.dat": "# freq TDOS Si\n-40 0 0\n0 1 1\n80 0 0\n",
        "input.json": json.dumps({"qe_phonon": {"work_dir": str(tmp_path), "prefix": "Si"}}),
    }
    for path, data in inputs.items():
        (tmp_path / path).write_text(data, encoding="utf-8")
    return PhononStepContext({}, str(tmp_path / "input.json"), str(tmp_path), "Si",
                             {"elements": ["Si"], "counts": [1]}, {})


def test_plot_adds_jpeg_preserving_existing_outputs_and_sources(archive):
    workspace = Path(archive.workspace)
    sources = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in workspace.rglob("*") if p.is_file()}
    module.plot_phonon_outputs(archive)
    for name in ["phband/phonon_band.png", "phband/phonon_band.csv", "phband/phonon_band_labels.csv",
                 "phdos/phonon_dos.png", "phdos/phonon_dos_atoms.csv", "phdos/phonon_dos_elements.csv"]:
        assert (workspace / name).stat().st_size > 0
    assert check_phonon_plot(str(workspace), "Si", "finished", "finished") == "finished"
    with Image.open(workspace / "phonon_band_dos.jpg") as image:
        assert image.format == "JPEG"
        assert image.width > image.height
        assert image.info["dpi"] == (300, 300)
        image.verify()
    assert all(hashlib.sha256(p.read_bytes()).hexdigest() == digest for p, digest in sources.items())
    assert not plt.get_fignums()


@pytest.mark.parametrize("band_min,dos_min", [(-20, -40), (-70, -10), (10, 20)])
def test_combined_axes_share_union_range_and_preserve_curves(tmp_path, monkeypatch, band_min, dos_min):
    band = [{"distance": 0., "mode_1_cm-1": band_min, "mode_2_cm-1": 30.},
            {"distance": 1., "mode_1_cm-1": 5., "mode_2_cm-1": 35.}]
    labels = [{"distance": 0., "label": "Γ"}, {"distance": 1., "label": "X"}]
    dos = [{"frequency_cm-1": dos_min, "TDOS": 0., "Si": 0.},
           {"frequency_cm-1": 80., "TDOS": 2., "Si": 1.}]
    seen = []
    savefig = Figure.savefig

    def inspect(fig, path, **kwargs):
        left, right = fig.axes
        assert left.get_shared_y_axes().joined(left, right)
        assert left.get_ylim() == right.get_ylim()
        lo, hi = left.get_ylim()
        assert lo < min(band_min, dos_min, 5.) and hi > 80.
        assert left.get_ylabel() == "Frequency (cm$^{-1}$)"
        assert right.get_ylabel() == ""
        assert not any(t.get_visible() for t in right.get_yticklabels())
        np.testing.assert_array_equal(left.lines[0].get_ydata(), [band_min, 5.])
        np.testing.assert_array_equal(left.lines[1].get_ydata(), [30., 35.])
        np.testing.assert_array_equal(right.lines[0].get_xdata(), [0., 2.])
        np.testing.assert_array_equal(right.lines[0].get_ydata(), [dos_min, 80.])
        np.testing.assert_array_equal(right.lines[1].get_xdata(), [0., 1.])
        assert [t.get_text() for t in left.get_xticklabels()] == ["Γ", "X"]
        assert right.get_position().width < left.get_position().width
        seen.append(True)
        return savefig(fig, path, **kwargs)

    monkeypatch.setattr(Figure, "savefig", inspect)
    module._plot_band_dos(str(tmp_path / "combined.jpg"), band, labels, dos, ["TDOS", "Si"])
    assert seen == [True]
    assert not plt.get_fignums()


def test_combined_save_failure_propagates_and_closes_figure(tmp_path, monkeypatch):
    def fail(*args, **kwargs):
        raise OSError("disk full")
    monkeypatch.setattr(Figure, "savefig", fail)
    with pytest.raises(OSError, match="disk full"):
        module._plot_band_dos(str(tmp_path / "combined.jpg"),
                              [{"distance": 0., "mode_1_cm-1": -1.}, {"distance": 1., "mode_1_cm-1": 2.}],
                              [{"label": "Γ", "distance": 0.}],
                              [{"frequency_cm-1": -1., "TDOS": 0.}, {"frequency_cm-1": 2., "TDOS": 1.}],
                              ["TDOS"])
    assert not plt.get_fignums()


def test_missing_archive_does_not_produce_combined_image(archive):
    workspace = Path(archive.workspace)
    (workspace / "phdos/Si.phdos.dat").unlink()
    with pytest.raises(FileNotFoundError):
        module.plot_phonon_outputs(archive)
    assert not (workspace / "phonon_band_dos.jpg").exists()


def test_legacy_plot_status_does_not_require_combined_image(tmp_path):
    from qeflow.phonon.archive import phonon_plot_files
    for name in phonon_plot_files():
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("legacy output")
    assert not (tmp_path / "phonon_band_dos.jpg").exists()
    assert check_phonon_plot(str(tmp_path), "Si", "finished", "finished") == "finished"


def test_existing_cli_outputs_combined_jpeg(archive):
    repository = Path(__file__).resolve().parents[2]
    environment = dict(os.environ, PYTHONPATH=str(repository))
    result = subprocess.run([sys.executable, "-m", "qeflow.phonon.process_phonon", "-c", archive.config_file, "--plot"],
                            cwd=archive.workspace, env=environment, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert (Path(archive.workspace) / "phonon_band_dos.jpg").exists()
