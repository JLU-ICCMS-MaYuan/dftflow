"""声子路径的坐标声明必须与标签使用的倒格子基底一致。"""

from pathlib import Path

import numpy as np
import pytest

from qeflow.input_deck.matdyn import write_matdyn_phband_input, write_matdyn_phdos_input
from qeflow.templates import get_qe_phonon_parameters_template


@pytest.mark.parametrize(
    "lattice,reciprocal_without_2pi",
    [
        (np.diag([4.0, 5.0, 7.0]), np.diag([0.25, 0.2, 1 / 7])),
        (
            [[4.0, 0.0, 0.0], [1.0, 5.0, 0.0], [0.0, 0.0, 7.0]],
            [[0.25, -0.05, 0.0], [0.0, 0.2, 0.0], [0.0, 0.0, 1 / 7]],
        ),
    ],
    ids=["orthorhombic", "oblique"],
)
@pytest.mark.parametrize("band_form", [True, False], ids=["band-form", "explicit-points"])
def test_non_cubic_band_input_uses_crystal_coordinates(
    tmp_path, lattice, reciprocal_without_2pi, band_form
):
    kpath = tmp_path / "KPATH.in"
    original = "0 0 0 G\n0 0.5 0 Y\n0 0.5 0 Y\n0 0.5 0.5 T\n0 0.5 0.5 T\n0.5 0.5 0.5 R\n"
    kpath.write_text(original, encoding="utf-8")
    config = get_qe_phonon_parameters_template()
    config["qe_phonon"]["phband"].update(kpath_points=7, q_in_band_form=band_form)

    output = Path(write_matdyn_phband_input(str(tmp_path), config, {"lattice": lattice}, "Si8P16"))
    namelist, points_text = output.read_text().split("/\n", 1)
    assert namelist.count("q_in_cryst_coord = .true.") == 1
    assert ("q_in_band_form = .true." in namelist) is band_form
    assert "flfrc = 'Si8P16.fc'" in namelist
    assert "flfrq = 'Si8P16.phband.freq'" in namelist

    lines = points_text.splitlines()
    assert lines[0] == "4"
    points = np.array([[float(v) for v in line.split()] for line in lines[1:]])
    fractional = np.array([[0, 0, 0], [0, 0.5, 0], [0, 0.5, 0.5], [0.5, 0.5, 0.5]])
    np.testing.assert_array_equal(points[:, :3], fractional)
    np.testing.assert_array_equal(points[:, 3], [7, 7, 7, 1])

    # 独立给出解析倒格子，避免用生产代码重复计算期望值。
    reciprocal = 2 * np.pi * np.array(reciprocal_without_2pi)
    cartesian = fractional @ reciprocal
    expected_distances = np.r_[0, np.cumsum(np.linalg.norm(np.diff(cartesian, axis=0), axis=1))]
    labels = [line.split() for line in (tmp_path / "Si8P16_band.labelinfo.dat").read_text().splitlines()]
    assert [row[0] for row in labels] == ["G", "Y", "T", "R"]
    assert [int(row[1]) for row in labels] == [1, 8, 15, 22]
    np.testing.assert_allclose([float(row[2]) for row in labels], expected_distances, atol=1e-10, rtol=0)
    np.testing.assert_array_equal([[float(v) for v in row[3:]] for row in labels], fractional)
    # 若漏掉声明，QE 将分数坐标当作 2π/alat 笛卡尔坐标，非立方路径不同。
    wrong_cartesian = fractional * (2 * np.pi / 4.0)
    assert not np.allclose(cartesian, wrong_cartesian)
    assert kpath.read_text() == original


def test_phdos_input_contract_is_unchanged(tmp_path):
    config = get_qe_phonon_parameters_template()
    config["qe_phonon"]["phdos"].update(qpoints=[6, 9, 18], ndos=500)
    output = Path(write_matdyn_phdos_input(str(tmp_path), config, "Si8P16"))
    assert output.read_text() == (
        "&input\n"
        "  asr = 'simple'\n"
        "  flfrc = 'Si8P16.fc'\n"
        "  fldos = 'Si8P16.phdos.dat'\n"
        "  dos = .true.\n"
        "  ndos = 500\n"
        "  nk1 = 6\n"
        "  nk2 = 9\n"
        "  nk3 = 18\n"
        "/\n"
    )
