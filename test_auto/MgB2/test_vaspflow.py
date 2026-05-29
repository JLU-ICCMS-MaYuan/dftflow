"""VASP 模块测试 - API + CLI + 实际运行"""
import os
import shutil
import subprocess
import sys
import numpy as np
import pytest
from pathlib import Path


# ============================================================================
# mattersim_opt 测试
# ============================================================================

def _write_mattersim_config(path: Path):
    path.write_text(
        """
[mattersim]
executable_path = "python"

mattersim_source_path = "/tmp/mock_mattersim/src"

[slurm]
header = "#!/bin/bash"

[mattersim_opt]
model = "mattersim-v1.0.0-1m"
device = "cpu"

[mattersim_opt.stage1]
optimizer = "FIRE"
constrain_symmetry = true
fmax = 0.1
ionic_steps = 200

[mattersim_opt.stage2]
optimizer = "FIRE"
cell_filter = "FrechetCellFilter"
constrain_symmetry = true
fix_axis = false
fmax = 0.05
ionic_steps = 300
""".strip()
        + "\n",
        encoding="utf-8",
    )


def test_mattersim_opt_api(chdir_work):
    from mattersimflow.opt.mattersim_opt import MatterSimOptSetup

    config_path = Path("input_mattersim.toml")
    _write_mattersim_config(config_path)

    setup = MatterSimOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.run()

    assert (Path("mattersim_opt") / "POSCAR").exists()
    assert (Path("mattersim_opt") / "run_mattersim_opt.py").exists()
    assert (Path("mattersim_opt") / "run_mattersim.sh").exists()

    runner = (Path("mattersim_opt") / "run_mattersim_opt.py").read_text(encoding="utf-8")
    script = (Path("mattersim_opt") / "run_mattersim.sh").read_text(encoding="utf-8")
    assert "MatterSimCalculator" in runner
    assert "write(f\"{stage['name']}_relaxed.vasp\"" in runner
    assert "'name': 'stage1'" in runner
    assert "'name': 'stage2'" in runner
    assert "from ase.units import GPa" in runner
    assert "FrechetCellFilter" in runner
    assert "python run_mattersim_opt.py > mattersim_opt.log 2>&1" in script


def test_mattersim_opt_supports_dynamic_stage3(chdir_work):
    from mattersimflow.opt.mattersim_opt import MatterSimOptSetup

    config_path = Path("input_mattersim.toml")
    _write_mattersim_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8")
        + """
[mattersim_opt.stage3]
optimizer = "BFGS"
cell_filter = "ExpCellFilter"
constrain_symmetry = false
fix_axis = [true, false, false, true, true, true]
fmax = 0.02
ionic_steps = 500
""",
        encoding="utf-8",
    )

    setup = MatterSimOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.run()

    runner = (Path("mattersim_opt") / "run_mattersim_opt.py").read_text(encoding="utf-8")
    assert "'name': 'stage3'" in runner
    assert "'optimizer': 'BFGS'" in runner
    assert "'cell_filter': 'ExpCellFilter'" in runner
    assert "'ionic_steps': 500" in runner
    assert "[True, False, False, True, True, True]" in runner


def test_mattersim_opt_rejects_non_contiguous_stages(chdir_work):
    from mattersimflow.opt.mattersim_opt import MatterSimOptSetup

    config_path = Path("input_mattersim.toml")
    _write_mattersim_config(config_path)
    broken_text = config_path.read_text(encoding="utf-8").replace("[mattersim_opt.stage2]", "[mattersim_opt.stage3]")
    config_path.write_text(broken_text, encoding="utf-8")

    setup = MatterSimOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    with pytest.raises(ValueError, match="阶段编号必须连续"):
        setup.run()


def test_mattersim_opt_keeps_legacy_flat_config_compatibility(chdir_work):
    from mattersimflow.opt.mattersim_opt import MatterSimOptSetup

    config_path = Path("input_mattersim_legacy.toml")
    config_path.write_text(
        """
[mattersim]
executable_path = "python"

mattersim_source_path = "/tmp/mock_mattersim/src"

[slurm]
header = "#!/bin/bash"

[mattersim_opt]
model = "mattersim-v1.0.0-1m"
device = "cpu"
optimizer = "FIRE"
cell_filter = "FrechetCellFilter"
constrain_symmetry = true
fix_axis = false
stage1_fmax = 0.1
stage1_steps = 200
stage2_fmax = 0.05
stage2_steps = 300
""".strip()
        + "\n",
        encoding="utf-8",
    )

    setup = MatterSimOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.run()

    runner = (Path("mattersim_opt") / "run_mattersim_opt.py").read_text(encoding="utf-8")
    assert "'name': 'stage1'" in runner
    assert "'name': 'stage2'" in runner


def test_mattersim_opt_multiple_pressures_generate_multiple_cases(chdir_work):
    from mattersimflow.opt import mattersim_opt as module

    config_path = Path("input_mattersim.toml")
    _write_mattersim_config(config_path)
    original_argv = sys.argv[:]
    try:
        sys.argv = ["mattersim_opt", "-i", "MgB2.vasp", "-c", "input_mattersim.toml", "-p", "0", "10"]
        module.main()
    finally:
        sys.argv = original_argv

    assert (Path("0") / "mattersim_opt" / "run_mattersim_opt.py").exists()
    assert (Path("10") / "mattersim_opt" / "run_mattersim_opt.py").exists()
    assert Path("submit_all_mattersim_opt.sh").exists()


def test_mattersim_opt_submit_uses_run_mattersim_script(chdir_work, monkeypatch):
    from mattersimflow.opt import mattersim_opt as module

    config_path = Path("input_mattersim.toml")
    _write_mattersim_config(config_path)

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(sys, "argv", ["mattersim_opt", "-i", "MgB2.vasp", "-c", "input_mattersim.toml", "-s"])

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "mattersim_opt"
    assert captured["script_name"] == "run_mattersim.sh"


def test_mattersim_opt_uses_installed_mattersim_without_source_path(chdir_work, monkeypatch):
    from mattersimflow.opt.mattersim_opt import MatterSimOptSetup

    config_path = Path("input_mattersim_installed.toml")
    config_path.write_text(
        """
[mattersim]
executable_path = "python"

[slurm]
header = "#!/bin/bash"

[mattersim_opt]
model = "mattersim-v1.0.0-1m"
device = "cpu"

[mattersim_opt.stage1]
optimizer = "FIRE"
constrain_symmetry = true
fmax = 0.1
ionic_steps = 200
""".strip()
        + "\n",
        encoding="utf-8",
    )

    monkeypatch.setattr("mattersimflow.opt.mattersim_opt.importlib.util.find_spec", lambda name: object())

    setup = MatterSimOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.run()

    runner = (Path("mattersim_opt") / "run_mattersim_opt.py").read_text(encoding="utf-8")
    assert "sys.path.insert(0," not in runner


def test_mattersim_opt_requires_source_path_when_mattersim_not_installed(chdir_work, monkeypatch):
    from mattersimflow.opt.mattersim_opt import MatterSimOptSetup

    config_path = Path("input_mattersim_missing.toml")
    config_path.write_text(
        """
[mattersim]
executable_path = "python"

[slurm]
header = "#!/bin/bash"

[mattersim_opt]
model = "mattersim-v1.0.0-1m"
device = "cpu"

[mattersim_opt.stage1]
optimizer = "FIRE"
constrain_symmetry = true
fmax = 0.1
ionic_steps = 200
""".strip()
        + "\n",
        encoding="utf-8",
    )

    monkeypatch.setattr("mattersimflow.opt.mattersim_opt.importlib.util.find_spec", lambda name: None)

    setup = MatterSimOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    with pytest.raises(ValueError, match="无法直接 import mattersim"):
        setup.run()


# ============================================================================
# vasp_scf 测试
# ============================================================================

def test_vasp_scf_api(chdir_work):
    """API 测试：vasp_scf 文件生成"""
    from vaspflow.scf.vasp_scf import VaspSetup

    setup = VaspSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    # 验证文件生成
    assert (Path("vasp_scf") / "INCAR").exists()
    assert (Path("vasp_scf") / "POTCAR").exists()
    assert (Path("vasp_scf") / "KPOINTS").exists()
    assert (Path("vasp_scf") / "POSCAR").exists()
    assert (Path("vasp_scf") / "run_vasp.sh").exists()

    # 验证 INCAR 内容
    incar_content = (Path("vasp_scf") / "INCAR").read_text()
    assert "ENCUT" in incar_content
    assert "520" in incar_content


def test_vasp_scf_cli(chdir_work):
    """CLI 测试：vasp_scf 命令行"""
    result = subprocess.run(
        ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_scf") / "INCAR").exists()
    assert (Path("vasp_scf") / "POTCAR").exists()


def test_vasp_scf_submit_uses_run_vasp_script(chdir_work, monkeypatch):
    from vaspflow.scf import vasp_scf as module

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(sys, "argv", ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "-s"])

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "vasp_scf"
    assert captured["script_name"] == "run_vasp.sh"


def test_vasp_lobsterin_api(chdir_work):
    from argparse import Namespace
    from vaspflow.cohp.vasp_lobsterin import VaspLobsterinSetup

    for filename in ["WAVECAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR"]:
        Path(filename).write_text(f"{filename}\n", encoding="utf-8")

    setup = VaspLobsterinSetup(config_file="input_vasp.toml")
    args = Namespace(
        COHPEnergy=[-10.0, 5.0],
        species_custom=["Mg", "B"],
        zvalances=["Mg:3s,3p", "B:2s,2p"],
        d_limit=[1.0, 2.5],
        orbitalWise=False,
        kSpaceCOHP=False,
        mode=0,
        custom_basis=["Mg:mg.sto"],
        config="input_vasp.toml",
    )

    setup.prepare_inputs(args)

    work_dir = Path("Mg_B_1.0_2.5")
    assert (work_dir / "lobsterin").exists()
    assert (work_dir / "run_lobster.sh").exists()
    assert (work_dir / "CONTCAR").is_symlink()

    lobsterin_content = (work_dir / "lobsterin").read_text(encoding="utf-8")
    assert "COHPstartEnergy  -10.0" in lobsterin_content
    assert "gaussianSmearingWidth 0.05" in lobsterin_content
    assert "basisfunctions Mg 3s 3p" in lobsterin_content
    assert "customSTOforAtom Mg  mg.sto" in lobsterin_content
    assert "cohpGenerator from 1.0 to 2.5 type Mg type B\n" in lobsterin_content
    assert "orbitalWise" not in lobsterin_content
    assert "kSpaceCOHP" not in lobsterin_content

    run_script_content = (work_dir / "run_lobster.sh").read_text(encoding="utf-8")
    assert "/home/mayuan/bin/lobster-5.1.1" in run_script_content


def test_vasp_lobsterin_api_optional_cohp_suffixes(chdir_work):
    from argparse import Namespace
    from vaspflow.cohp.vasp_lobsterin import VaspLobsterinSetup

    for filename in ["WAVECAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR", "KPATH.in"]:
        Path(filename).write_text(f"{filename}\n", encoding="utf-8")

    setup = VaspLobsterinSetup(config_file="input_vasp.toml")
    args = Namespace(
        COHPEnergy=[-10.0, 5.0],
        species_custom=["Mg", "B"],
        zvalances=["Mg:3s,3p", "B:2s,2p"],
        d_limit=[1.0, 2.5],
        orbitalWise=True,
        kSpaceCOHP=True,
        mode=5,
        custom_basis=None,
        config="input_vasp.toml",
    )

    setup.prepare_inputs(args)

    lobsterin_content = (Path("Mg_B_1.0_2.5") / "lobsterin").read_text(encoding="utf-8")
    assert (
        "cohpGenerator from 1.0 to 2.5 type Mg type B orbitalWise kSpaceCOHP\n"
        in lobsterin_content
    )
    assert (Path("Mg_B_1.0_2.5") / "KPOINTS").exists()
    assert not (Path("Mg_B_1.0_2.5") / "KPOINTS").is_symlink()
    assert (Path("Mg_B_1.0_2.5") / "KPATH.in").exists()
    assert not (Path("Mg_B_1.0_2.5") / "KPATH.in").is_symlink()


def test_vasp_lobsterin_submit_uses_run_lobster_script(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_lobsterin as module

    for filename in ["WAVECAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR"]:
        Path(filename).write_text(f"{filename}\n", encoding="utf-8")

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "vasp_lobsterin",
            "-c",
            "input_vasp.toml",
            "-s",
            "--species_custom",
            "Mg",
            "B",
            "-z",
            "Mg:3s,3p",
            "B:2s,2p",
            "-d",
            "1.0",
            "2.5",
        ],
    )

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "Mg_B_1.0_2.5"
    assert captured["script_name"] == "run_lobster.sh"


def _write_minimal_kcohp_fixture(base_dir: Path):
    base_dir.mkdir(parents=True, exist_ok=True)
    (base_dir / "POSCAR").write_text(
        "\n".join(
            [
                "MgB2",
                "1.0",
                "1.0 0.0 0.0",
                "0.0 1.0 0.0",
                "0.0 0.0 1.0",
                "Mg B",
                "1 2",
                "Direct",
                "0.0 0.0 0.0",
                "0.5 0.5 0.5",
                "0.25 0.25 0.25",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (base_dir / "KPATH.in").write_text(
        "\n".join(
            [
                "K-Path Generated by VASPKIT.",
                "20",
                "Line-Mode",
                "Reciprocal",
                "0.0 0.0 0.0 GAMMA",
                "0.5 0.0 0.0 M",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (base_dir / "KPOINTS").write_text(
        "\n".join(
            [
                "0.030 1 1 1 1 0.030 2",
                "3",
                "Reciprocal lattice",
                "0.0 0.0 0.0 1",
                "0.0 0.0 0.0 0",
                "0.5 0.0 0.0 0",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (base_dir / "OUTCAR").write_text(" E-fermi : 5.0000 XC(G=0): 0.0000\n", encoding="utf-8")

    def write_band_file(path: Path, energies, values):
        labels = ["Mg1_3s", "B2_2s", "B3_2s"]
        lines = ["#COHP_munujk matrix Spins 1 kpoints 2"]
        for idx, energy in enumerate(energies, start=1):
            lines.append(
                f"COHP_munujk matrix for spin 1 eigenvalue {energy:.4f} (eV) "
                f"kpoint {idx} at 0 0 0 (frac) or 0 0 0 (cart)"
            )
            lines.append(
                "       basisfunction               Mg1_3s                B2_2s                B3_2s"
            )
            matrix = [
                [0.0, values[idx - 1][0], values[idx - 1][1]],
                [values[idx - 1][0], 0.0, 0.0],
                [values[idx - 1][1], 0.0, 0.0],
            ]
            for label, row in zip(labels, matrix):
                lines.append(
                    f"{label:>20} " + " ".join(f"{item:>20.8f}" for item in row)
                )
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    write_band_file(base_dir / "KspaceCOHPBand1.lobster", [4.0, 5.0], [(1.0, 2.0), (3.0, 4.0)])
    write_band_file(base_dir / "KspaceCOHPBand2.lobster", [6.0, 7.0], [(5.0, 6.0), (7.0, 8.0)])


def test_vasp_plot_kcohp_api_generates_dat_and_png(chdir_work):
    from vaspflow.cohp.vasp_plot_kcohp import VaspPlotKCohpSetup

    work_dir = Path("kcohp_case")
    _write_minimal_kcohp_fixture(work_dir)

    setup = VaspPlotKCohpSetup(
        directory=str(work_dir),
        interaction_texts=["Mg:3s|B:2s"],
        fermi=None,
        emin=-2.0,
        emax=3.0,
        vmax=2.0,
        output_prefix="kcohp_test",
    )
    dat_path, png_path = setup.run()

    assert Path(dat_path).exists()
    assert Path(png_path).exists()
    dat_content = Path(dat_path).read_text(encoding="utf-8")
    assert "Mg:3s | B:2s" in dat_content
    assert "0.0000000000" in dat_content
    assert "-1.0000000000" in dat_content
    assert "3.0000000000" in dat_content
    assert "15.0000000000" in dat_content


def test_vasp_plot_kcohp_cli_reads_outcar_fermi(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_plot_kcohp as module

    work_dir = Path("kcohp_case")
    _write_minimal_kcohp_fixture(work_dir)

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "vasp_plot_kcohp",
            "-d",
            str(work_dir),
            "--interaction",
            "Mg:3s|B:2s",
            "-o",
            "cli_kcohp",
        ],
    )

    module.main()
    dat_content = (work_dir / "cli_kcohp.dat").read_text(encoding="utf-8")
    assert "-1.0000000000" in dat_content
    assert "0.0000000000" in dat_content
    assert "1.0000000000" in dat_content
    assert "2.0000000000" in dat_content


def test_vasp_plot_kcohp_matches_shell_orbitals_across_atoms(chdir_work):
    from vaspflow.cohp.vasp_plot_kcohp import match_indices, parse_interaction_side

    labels = ["Mg1_3s", "Mg1_3p_x", "Mg1_3p_y", "B2_2p_x", "B3_2p_y"]
    side = parse_interaction_side("Mg:3p")
    assert match_indices(labels, side) == [1, 2]

    side_b = parse_interaction_side("B:2p")
    assert match_indices(labels, side_b) == [3, 4]


def test_vasp_process_eband_generates_csv_and_png(chdir_work):
    pytest.importorskip("matplotlib")
    from vaspflow.eband.vasp_process_eband import VaspProcessBandSetup

    work_dir = Path("vasp_eband")
    work_dir.mkdir()
    (work_dir / "FERMI_ENERGY").write_text("5.000000\n", encoding="utf-8")
    (work_dir / "KLABELS").write_text(
        "\n".join(
            [
                "K-Label Coordinate",
                "GAMMA 0.000",
                "M 1.000",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "BAND.dat").write_text(
        "\n".join(
            [
                "#K-Path(1/A) Energy-Level(eV)",
                "# NKPTS & NBANDS: 3   2",
                "# Band-Index    1",
                "0.00000 4.000000",
                "0.50000 5.000000",
                "1.00000 6.000000",
                "# Band-Index    2",
                "0.00000 5.500000",
                "0.50000 6.000000",
                "1.00000 6.500000",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "PBAND_Mg.dat").write_text(
        "\n".join(
            [
                "#K-Path Energy s py pz px dxy dyz dz2 dxz x2-y2 tot",
                "# NKPTS & NBANDS: 3   2",
                "# Band-Index    1",
                "0.00000 4.000000 0.1 0.2 0.3 0.4 0 0 0 0 0 1.0",
                "0.50000 5.000000 0.2 0.3 0.4 0.5 0 0 0 0 0 1.4",
                "1.00000 6.000000 0.3 0.4 0.5 0.6 0 0 0 0 0 1.8",
                "# Band-Index    2",
                "0.00000 5.500000 0.2 0.1 0.1 0.2 0 0 0 0 0 0.6",
                "0.50000 6.000000 0.3 0.2 0.2 0.3 0 0 0 0 0 1.0",
                "1.00000 6.500000 0.4 0.3 0.3 0.4 0 0 0 0 0 1.4",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "PBAND_B.dat").write_text(
        "\n".join(
            [
                "#K-Path Energy s py pz px dxy dyz dz2 dxz x2-y2 tot",
                "# NKPTS & NBANDS: 3   2",
                "# Band-Index    1",
                "0.00000 4.000000 0.5 0.1 0.1 0.1 0 0 0 0 0 0.8",
                "0.50000 5.000000 0.6 0.1 0.2 0.1 0 0 0 0 0 1.0",
                "1.00000 6.000000 0.7 0.2 0.2 0.2 0 0 0 0 0 1.3",
                "# Band-Index    2",
                "0.00000 5.500000 0.4 0.2 0.1 0.1 0 0 0 0 0 0.8",
                "0.50000 6.000000 0.5 0.2 0.2 0.2 0 0 0 0 0 1.1",
                "1.00000 6.500000 0.6 0.3 0.2 0.2 0 0 0 0 0 1.3",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    setup = VaspProcessBandSetup(
        directory=str(work_dir),
        element_specs=["Mg:s,p", "B:s"],
        skip_vaspkit=True,
        out_prefix="mgb2_band",
    )
    csv_path, png_path = setup.run()

    assert Path(csv_path).exists()
    assert Path(png_path).exists()
    csv_content = Path(csv_path).read_text(encoding="utf-8")
    assert "band_index,kpoint_index,kpath_dist,energy_ev,weight_Mg_s,weight_Mg_p,weight_B_s,label,tick_x" in csv_content
    assert "1,2,0.5,0.0,0.2,1.2,0.6,,0.5" in csv_content
    assert "1,1,0.0,-1.0,0.1,0.9,0.5,GAMMA,0.0" in csv_content


def test_vasp_process_eband_supports_fine_grained_orbitals(chdir_work):
    from vaspflow.eband.vasp_process_eband import VaspProcessBandSetup

    work_dir = Path("vasp_eband")
    work_dir.mkdir()
    (work_dir / "FERMI_ENERGY").write_text("5.000000\n", encoding="utf-8")
    (work_dir / "KLABELS").write_text("GAMMA 0.000\nM 1.000\n", encoding="utf-8")
    (work_dir / "BAND.dat").write_text(
        "\n".join(
            [
                "#K-Path(1/A) Energy-Level(eV)",
                "# NKPTS & NBANDS: 2   1",
                "# Band-Index    1",
                "0.00000 4.000000",
                "1.00000 5.000000",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "PBAND_Mg.dat").write_text(
        "\n".join(
            [
                "#K-Path Energy s py pz px dxy dyz dz2 dxz x2-y2 tot",
                "# NKPTS & NBANDS: 2   1",
                "# Band-Index    1",
                "0.00000 4.000000 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9 4.5",
                "1.00000 5.000000 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9 1.0 5.4",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    setup = VaspProcessBandSetup(
        directory=str(work_dir),
        element_specs=["Mg:px,dz2,dx2"],
        skip_vaspkit=True,
        out_prefix="mgb2_band_fine",
    )
    csv_path, _ = setup.run()

    csv_content = Path(csv_path).read_text(encoding="utf-8")
    assert "weight_Mg_px,weight_Mg_dz2,weight_Mg_dx2" in csv_content
    assert "1,1,0.0,-1.0,0.4,0.7,0.9,GAMMA,0.0" in csv_content


def test_replace_doscar_fermi_preserves_spacing_pattern(chdir_work):
    from vaspflow.eledos.vasp_process_eledos import replace_doscar_fermi

    doscar_path = Path("DOSCAR")
    original_line = "  -10.0   10.0    200   5.0000000000   1.0\n"
    doscar_path.write_text(
        "\n".join(["1", "2", "3", "4", "5", original_line.rstrip("\n")]) + "\n",
        encoding="utf-8",
    )

    replace_doscar_fermi(str(doscar_path), 2.5)

    new_line = doscar_path.read_text(encoding="utf-8").splitlines()[5]
    assert new_line == "  -10.0   10.0    200   2.5000000000   1.0"


def test_vasp_process_eband_syncs_doscar_and_runs_all_vaspkit_steps(chdir_work, monkeypatch):
    from vaspflow.eband import vasp_process_eband as module

    work_dir = Path("vasp_eband")
    work_dir.mkdir()
    (work_dir / "DOSCAR").write_text(
        "\n".join(
            [
                "1 1 1 1 1",
                "line2",
                "line3",
                "line4",
                "line5",
                "  -10.0   10.0    200   5.0000000000   1.0",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    scf_dir = Path("vasp_scf")
    scf_dir.mkdir()
    (scf_dir / "OUTCAR").write_text(" E-fermi : 2.5000 XC(G=0): 0.0000\n", encoding="utf-8")

    calls = []

    def fake_run_vaspkit(work_dir_arg, command_text):
        calls.append((work_dir_arg, command_text))
        return "", ""

    monkeypatch.setattr(module, "run_vaspkit", fake_run_vaspkit)

    setup = module.VaspProcessBandSetup(
        directory=str(work_dir),
        element_specs=["Mg:s"],
    )
    setup.prepare_band_files()

    assert calls == [
        (str(work_dir.resolve()), "21\n211\n"),
        (str(work_dir.resolve()), "21\n213\n"),
        (str(work_dir.resolve()), "21\n215\n"),
    ]
    assert "  -10.0   10.0    200   2.5000000000   1.0" in (work_dir / "DOSCAR").read_text(
        encoding="utf-8"
    )


def test_vasp_process_eband_accepts_small_band_energy_rounding_difference():
    from vaspflow.eband.vasp_process_eband import ensure_matching_blocks

    ref_blocks = [
        {
            "kpath": np.array([0.0, 0.5, 1.0], dtype=float),
            "energy": np.array([1.0, 2.0, 3.0], dtype=float),
        }
    ]
    test_blocks = [
        {
            "kpath": np.array([0.0, 0.5, 1.0], dtype=float),
            "energy": np.array([1.0, 2.0 + 1e-6, 3.0], dtype=float),
        }
    ]

    ensure_matching_blocks(ref_blocks, test_blocks, "PBAND_Mg.dat")


class _FakeCohpcar:
    def __init__(self):
        self.energies = np.array([-1.0, 0.0, 1.0], dtype=float)
        self.cohp_data = {
            "average": {
                "COHP": {"up": np.array([0.1, 0.2, 0.3], dtype=float)},
                "ICOHP": {"up": np.array([1.1, 1.2, 1.3], dtype=float)},
            },
            "bond_1": {},
            "bond_2": {},
            "bond_3": {},
        }
        self.orb_res_cohp = {
            "bond_1": {
                "Mg1_3s-B2_2s": {
                    "COHP": {"up": np.array([3.0, 6.0, 9.0], dtype=float)},
                    "ICOHP": {"up": np.array([30.0, 60.0, 90.0], dtype=float)},
                },
                "Mg1_3p_x-B2_2p_y": {
                    "COHP": {"up": np.array([1.0, 2.0, 3.0], dtype=float)},
                    "ICOHP": {"up": np.array([10.0, 20.0, 30.0], dtype=float)},
                },
                "Mg1_3p_y-B3_2p_x": {
                    "COHP": {"up": np.array([4.0, 5.0, 6.0], dtype=float)},
                    "ICOHP": {"up": np.array([40.0, 50.0, 60.0], dtype=float)},
                },
            }
        }


def test_vasp_plot_cohp_api_generates_outputs(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_plot_cohp as module

    Path("COHPCAR.lobster").write_text("fake\n", encoding="utf-8")
    monkeypatch.setattr(module, "load_cohpcar", lambda filename: _FakeCohpcar())

    setup = module.VaspPlotCohpSetup(
        directory=".",
        interaction_texts=["Mg:3s|B:2s", "Mg:3p|B:2p"],
        lower_energy=-1.0,
        upper_energy=1.0,
        emin=-1.0,
        emax=1.0,
    )
    outputs = setup.run()

    assert Path(outputs["proj_dat"]).exists()
    assert Path(outputs["average_dat"]).exists()
    assert Path(outputs["proj_png"]).exists()
    assert Path(outputs["average_png"]).exists()

    proj_dat = Path(outputs["proj_dat"]).read_text(encoding="utf-8")
    assert "Mg:3s | B:2s-COHP" in proj_dat
    assert "Mg:3p | B:2p-ICOHP" in proj_dat
    assert "1.000000" in proj_dat
    assert "2.333333" in proj_dat
    assert "23.333333" in proj_dat

    avg_dat = Path(outputs["average_dat"]).read_text(encoding="utf-8")
    assert "energy  COHP  ICOHP" in avg_dat


def test_vasp_plot_cohp_average_by_num_bonds(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_plot_cohp as module
    from vaspflow.cohp.orbital_interactions import parse_interaction_spec

    fake = _FakeCohpcar()
    total_cohp, total_icohp, icohp_at_ef = module.average_cohp_for_interaction(
        fake,
        len(fake.cohp_data) - 1,
        fake.energies,
        parse_interaction_spec("Mg:3p|B:2p"),
    )
    assert np.allclose(total_cohp, np.array([5.0, 7.0, 9.0]) / 3.0)
    assert np.allclose(total_icohp, np.array([50.0, 70.0, 90.0]) / 3.0)
    assert icohp_at_ef == 70.0 / 3.0


def test_vasp_plot_cohp_cli_uses_out_prefix(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_plot_cohp as module

    Path("COHPCAR.lobster").write_text("fake\n", encoding="utf-8")
    monkeypatch.setattr(module, "load_cohpcar", lambda filename: _FakeCohpcar())
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "vasp_plot_cohp",
            "-d",
            ".",
            "--interaction",
            "Mg:3s|B:2s",
            "--out-prefix",
            "mgb2",
        ],
    )

    module.main()
    assert Path("mgb2_proj_cohp.dat").exists()
    assert Path("mgb2_average_cohp.png").exists()


def test_vasp_plot_cohp_errors_when_orbitalwise_data_missing(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_plot_cohp as module

    class FakeNoOrbitalCohpcar:
        def __init__(self):
            self.orb_res_cohp = None
            self.energies = np.array([-1.0, 0.0, 1.0], dtype=float)
            self.cohp_data = {"average": {"COHP": {"up": np.array([0, 0, 0])}, "ICOHP": {"up": np.array([0, 0, 0])}}, "1": {}}

    Path("COHPCAR.lobster").write_text("fake\n", encoding="utf-8")
    monkeypatch.setattr(module, "load_cohpcar", lambda filename: FakeNoOrbitalCohpcar())

    setup = module.VaspPlotCohpSetup(directory=".", interaction_texts=["Mg:3s|B:2s"])
    with pytest.raises(ValueError, match="orbital-resolved COHP"):
        setup.run()


@pytest.mark.run
def test_vasp_scf_run(chdir_shared):
    """实际运行测试：vasp_scf --run"""
    result = subprocess.run(
        ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    print("STDOUT:", result.stdout)
    print("STDERR:", result.stderr)
    print("Return code:", result.returncode)

    assert result.returncode == 0
    assert (Path("vasp_scf") / "OUTCAR").exists()
    assert (Path("vasp_scf") / "CHGCAR").exists()
    assert (Path("vasp_scf") / "WAVECAR").exists()


# ============================================================================
# vasp_eband 测试
# ============================================================================

def test_vasp_eband_api(chdir_work):
    """API 测试：vasp_eband 文件生成"""
    # 创建假的 SCF 输出
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.eband.vasp_eband import VaspBandSetup

    setup = VaspBandSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_eband") / "INCAR").exists()
    incar_content = (Path("vasp_eband") / "INCAR").read_text()
    assert "ICHARG" in incar_content
    assert "11" in incar_content


def test_vasp_eband_cli(chdir_work):
    """CLI 测试：vasp_eband 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_eband", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_eband") / "INCAR").exists()


@pytest.mark.run
def test_vasp_eband_run(chdir_shared):
    """实际运行测试：vasp_eband --run（依赖 vasp_scf）"""
    # 确保 vasp_scf 已运行
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_eband", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_eband") / "OUTCAR").exists()


# ============================================================================
# vasp_eledos 测试
# ============================================================================

def test_vasp_eledos_api(chdir_work):
    """API 测试：vasp_eledos 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.eledos.vasp_eledos import VaspDosSetup

    setup = VaspDosSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_eledos") / "INCAR").exists()
    incar_content = (Path("vasp_eledos") / "INCAR").read_text()
    assert "ISMEAR" in incar_content


def test_vasp_eledos_cli(chdir_work):
    """CLI 测试：vasp_eledos 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_eledos", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_eledos") / "INCAR").exists()


def test_vasp_process_eledos_generates_csv_and_png(chdir_work, monkeypatch):
    pytest.importorskip("matplotlib")
    from vaspflow.eledos import vasp_process_eledos as module

    work_dir = Path("vasp_eledos")
    work_dir.mkdir()
    (work_dir / "POSCAR").write_text(
        "\n".join(
            [
                "MgB2",
                "1.0",
                "1.0 0.0 0.0",
                "0.0 1.0 0.0",
                "0.0 0.0 1.0",
                "Mg B",
                "1 2",
                "Direct",
                "0.0 0.0 0.0",
                "0.5 0.5 0.5",
                "0.25 0.25 0.25",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "DOSCAR").write_text(
        "\n".join(
            [
                "1 1 1 1 1",
                "line2",
                "line3",
                "line4",
                "line5",
                "  -10.0  10.0  200  5.0000000000  1.0",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    scf_dir = Path("vasp_scf")
    scf_dir.mkdir()
    (scf_dir / "OUTCAR").write_text(" E-fermi : 2.5000 XC(G=0): 0.0000\n", encoding="utf-8")

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = Path(cwd)
            self.returncode = 0

        def communicate(self, input=None):
            if input == "11\n113\n":
                (self.cwd / "PDOS_Mg.dat").write_text(
                    "\n".join(
                        [
                            "Energy s py pz px tot",
                            "-1.0 1.0 2.0 3.0 4.0 10.0",
                            "0.0 2.0 3.0 4.0 5.0 14.0",
                            "1.0 3.0 4.0 5.0 6.0 18.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
                (self.cwd / "PDOS_B.dat").write_text(
                    "\n".join(
                        [
                            "Energy s py pz px tot",
                            "-1.0 0.5 1.0 1.5 2.0 5.0",
                            "0.0 1.0 1.5 2.0 2.5 7.0",
                            "1.0 1.5 2.0 2.5 3.0 9.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
            elif input == "11\n111\n":
                (self.cwd / "TDOS.dat").write_text(
                    "\n".join(
                        [
                            "Energy DOS",
                            "-1.0 20.0",
                            "0.0 24.0",
                            "1.0 28.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
            return ("", "")

    monkeypatch.setattr(module.subprocess, "Popen", lambda *args, **kwargs: FakeProcess(kwargs.get("cwd")))

    setup = module.VaspProcessDosSetup(
        directory=str(work_dir),
        element_specs=["Mg:s,p", "B:s"],
        out_prefix="mgb2_dos",
    )
    merged_csv, at_ef_csv, plot_png = setup.run()

    assert Path(merged_csv).exists()
    assert Path(at_ef_csv).exists()
    assert Path(plot_png).exists()
    assert "2.5000000000" in (work_dir / "DOSCAR").read_text(encoding="utf-8")

    merged_content = Path(merged_csv).read_text(encoding="utf-8")
    assert "Energy(eV),Mg_s,Mg_p,Mg,B_s,B,TDOS" in merged_content
    assert "0.0,2.0,12.0,14.0,1.0,7.0,24.0" in merged_content

    ef_content = Path(at_ef_csv).read_text(encoding="utf-8")
    assert "Energy(eV),Mg_s,Mg_p,Mg,B_s,B,TDOS" in ef_content
    assert "0.0,2.0,12.0,14.0,1.0,7.0,24.0" in ef_content


def test_vasp_process_eledos_plot_uses_two_times_tdos_at_ef_as_ymax(chdir_work):
    from vaspflow.eledos import vasp_process_eledos as module

    energy = np.array([-1.0, 0.0, 1.0], dtype=float)
    tdos = np.array([100.0, 4.0, 120.0], dtype=float)

    assert module.determine_dos_plot_ymax(energy, tdos) == 8.0


def test_vasp_process_eledos_plot_falls_back_to_one_when_tdos_at_ef_is_non_positive(chdir_work):
    from vaspflow.eledos import vasp_process_eledos as module

    energy = np.array([-1.0, 0.0, 1.0], dtype=float)
    tdos = np.array([3.0, 0.0, 5.0], dtype=float)

    assert module.determine_dos_plot_ymax(energy, tdos) == 1.0


def test_vasp_process_eledos_supports_fine_grained_orbitals(chdir_work, monkeypatch):
    from vaspflow.eledos import vasp_process_eledos as module

    work_dir = Path("vasp_eledos")
    work_dir.mkdir()
    (work_dir / "POSCAR").write_text(
        "\n".join(
            [
                "MgB2",
                "1.0",
                "1.0 0.0 0.0",
                "0.0 1.0 0.0",
                "0.0 0.0 1.0",
                "Mg B",
                "1 2",
                "Direct",
                "0.0 0.0 0.0",
                "0.5 0.5 0.5",
                "0.25 0.25 0.25",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "DOSCAR").write_text(
        "\n".join(
            [
                "1 1 1 1 1",
                "line2",
                "line3",
                "line4",
                "line5",
                "  -10.0  10.0  200  5.0000000000  1.0",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    scf_dir = Path("vasp_scf")
    scf_dir.mkdir()
    (scf_dir / "OUTCAR").write_text(" E-fermi : 2.5000 XC(G=0): 0.0000\n", encoding="utf-8")

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = Path(cwd)
            self.returncode = 0

        def communicate(self, input=None):
            if input == "11\n113\n":
                (self.cwd / "PDOS_Mg.dat").write_text(
                    "\n".join(
                        [
                            "Energy s py pz px dxy dyz dz2 dxz dx2 tot",
                            "-1.0 1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 45.0",
                            "0.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 54.0",
                            "1.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 63.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
            elif input == "11\n111\n":
                (self.cwd / "TDOS.dat").write_text(
                    "\n".join(
                        [
                            "Energy DOS",
                            "-1.0 20.0",
                            "0.0 24.0",
                            "1.0 28.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
            return ("", "")

    monkeypatch.setattr(module.subprocess, "Popen", lambda *args, **kwargs: FakeProcess(kwargs.get("cwd")))

    setup = module.VaspProcessDosSetup(
        directory=str(work_dir),
        element_specs=["Mg:px,dz2,dx2"],
        out_prefix="mgb2_dos_fine",
    )
    merged_csv, at_ef_csv, _ = setup.run()

    merged_content = Path(merged_csv).read_text(encoding="utf-8")
    assert "Energy(eV),Mg_px,Mg_dz2,Mg_dx2,Mg,TDOS" in merged_content
    assert "0.0,5.0,8.0,10.0,54.0,24.0" in merged_content

    ef_content = Path(at_ef_csv).read_text(encoding="utf-8")
    assert "Energy(eV),Mg_px,Mg_dz2,Mg_dx2,Mg,TDOS" in ef_content
    assert "0.0,5.0,8.0,10.0,54.0,24.0" in ef_content


def test_vasp_process_eledos_supports_elements_total_mode(chdir_work, monkeypatch):
    from vaspflow.eledos import vasp_process_eledos as module

    work_dir = Path("vasp_eledos")
    work_dir.mkdir()
    (work_dir / "POSCAR").write_text(
        "\n".join(
            [
                "MgB2",
                "1.0",
                "1.0 0.0 0.0",
                "0.0 1.0 0.0",
                "0.0 0.0 1.0",
                "Mg B",
                "1 2",
                "Direct",
                "0.0 0.0 0.0",
                "0.5 0.5 0.5",
                "0.25 0.25 0.25",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "DOSCAR").write_text(
        "\n".join(
            [
                "1 1 1 1 1",
                "line2",
                "line3",
                "line4",
                "line5",
                "  -10.0  10.0  200  5.0000000000  1.0",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    scf_dir = Path("vasp_scf")
    scf_dir.mkdir()
    (scf_dir / "OUTCAR").write_text(" E-fermi : 2.5000 XC(G=0): 0.0000\n", encoding="utf-8")

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = Path(cwd)
            self.returncode = 0

        def communicate(self, input=None):
            if input == "11\n113\n":
                (self.cwd / "PDOS_Mg.dat").write_text(
                    "Energy s py pz px tot\n-1.0 1.0 2.0 3.0 4.0 10.0\n0.0 2.0 3.0 4.0 5.0 14.0\n1.0 3.0 4.0 5.0 6.0 18.0\n",
                    encoding="utf-8",
                )
                (self.cwd / "PDOS_B.dat").write_text(
                    "Energy s py pz px tot\n-1.0 0.5 1.0 1.5 2.0 5.0\n0.0 1.0 1.5 2.0 2.5 7.0\n1.0 1.5 2.0 2.5 3.0 9.0\n",
                    encoding="utf-8",
                )
            elif input == "11\n111\n":
                (self.cwd / "TDOS.dat").write_text(
                    "Energy DOS\n-1.0 20.0\n0.0 24.0\n1.0 28.0\n",
                    encoding="utf-8",
                )
            return ("", "")

    monkeypatch.setattr(module.subprocess, "Popen", lambda *args, **kwargs: FakeProcess(kwargs.get("cwd")))

    setup = module.VaspProcessDosSetup(
        directory=str(work_dir),
        total_elements=["Mg", "B"],
        out_prefix="mgb2_total",
    )
    merged_csv, at_ef_csv, _ = setup.run()

    merged_content = Path(merged_csv).read_text(encoding="utf-8")
    assert "Energy(eV),Mg,B,TDOS" in merged_content
    assert "0.0,14.0,7.0,24.0" in merged_content

    ef_content = Path(at_ef_csv).read_text(encoding="utf-8")
    assert "Energy(eV),Mg,B,TDOS" in ef_content
    assert "0.0,14.0,7.0,24.0" in ef_content


def test_vasp_idos_computed_generates_csv(chdir_work, monkeypatch):
    from vaspflow.eledos import vasp_idos_computed as module

    work_dir = Path("vasp_eledos")
    work_dir.mkdir()
    (work_dir / "POSCAR").write_text(
        "\n".join(
            [
                "MgB2",
                "1.0",
                "1.0 0.0 0.0",
                "0.0 1.0 0.0",
                "0.0 0.0 1.0",
                "Mg B",
                "1 2",
                "Direct",
                "0.0 0.0 0.0",
                "0.5 0.5 0.5",
                "0.25 0.25 0.25",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    (work_dir / "DOSCAR").write_text(
        "\n".join(
            [
                "1 1 1 1 1",
                "line2",
                "line3",
                "line4",
                "line5",
                "  -10.0  10.0  200  5.0000000000  1.0",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    scf_dir = Path("vasp_scf")
    scf_dir.mkdir()
    (scf_dir / "OUTCAR").write_text(" E-fermi : 2.5000 XC(G=0): 0.0000\n", encoding="utf-8")

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = Path(cwd)
            self.returncode = 0

        def communicate(self, input=None):
            if input == "11\n113\n":
                (self.cwd / "PDOS_Mg.dat").write_text(
                    "\n".join(
                        [
                            "Energy s py pz px tot",
                            "-1.0 1.0 2.0 3.0 4.0 10.0",
                            "0.0 2.0 3.0 4.0 5.0 14.0",
                            "1.0 3.0 4.0 5.0 6.0 18.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
                (self.cwd / "PDOS_B.dat").write_text(
                    "\n".join(
                        [
                            "Energy s py pz px tot",
                            "-1.0 0.5 1.0 1.5 2.0 5.0",
                            "0.0 1.0 1.5 2.0 2.5 7.0",
                            "1.0 1.5 2.0 2.5 3.0 9.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
            elif input == "11\n111\n":
                (self.cwd / "TDOS.dat").write_text(
                    "\n".join(
                        [
                            "Energy DOS",
                            "-1.0 20.0",
                            "0.0 24.0",
                            "1.0 28.0",
                        ]
                    )
                    + "\n",
                    encoding="utf-8",
                )
            return ("", "")

    monkeypatch.setattr(module, "generate_vaspkit_dos_files", lambda wd: (FakeProcess(wd).communicate("11\n113\n"), FakeProcess(wd).communicate("11\n111\n")))

    setup = module.VaspIdosComputedSetup(
        directory=str(work_dir),
        element_specs=["Mg:s,p", "B:s"],
        low=-0.5,
        up=0.5,
        out_prefix="mgb2_idos",
    )
    csv_path = setup.run()

    assert Path(csv_path).exists()
    assert "2.5000000000" in (work_dir / "DOSCAR").read_text(encoding="utf-8")
    csv_content = Path(csv_path).read_text(encoding="utf-8")
    assert "low_eV,up_eV,TDOS_IDOS,Mg_s_IDOS,Mg_p_IDOS,B_s_IDOS,sum_selected_PDOS_IDOS,difference_TDOS_minus_sum_selected" in csv_content
    assert "-0.5,0.5,24.0,2.0,12.0,1.0,15.0,9.0" in csv_content


@pytest.mark.run
def test_vasp_eledos_run(chdir_shared):
    """实际运行测试：vasp_eledos --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_eledos", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_eledos") / "OUTCAR").exists()


# ============================================================================
# vasp_fermisurface 测试
# ============================================================================

def test_vasp_fermisurface_api(chdir_work):
    """API 测试：vasp_fermisurface 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.fermisurface.vasp_fermisurface import VaspFermiSetup

    setup = VaspFermiSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_fermisurface") / "INCAR").exists()


def test_vasp_fermisurface_uses_vaspkit_261(chdir_work, monkeypatch):
    from vaspflow.fermisurface.vasp_fermisurface import VaspFermiSetup

    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    captured = {}

    class FakeProcess:
        def communicate(self, input=None):
            captured["input"] = input
            Path("vasp_fermisurface/KPOINTS").write_text("fake kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        captured["cmd"] = cmd
        captured["cwd"] = cwd
        captured["stdout"] = stdout
        captured["stderr"] = stderr
        return FakeProcess()

    monkeypatch.setattr("vaspflow.fermisurface.vasp_fermisurface.subprocess.Popen", fake_popen)

    setup = VaspFermiSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert captured["cmd"] == ["vaspkit"]
    assert captured["cwd"].endswith("vasp_fermisurface")
    assert captured["input"] == "261\n0.04\n"
    assert captured["stdout"] is subprocess.DEVNULL
    assert captured["stderr"] is subprocess.DEVNULL
    assert (Path("vasp_fermisurface") / "KPOINTS").exists()


def test_vasp_fermisurface_cli(chdir_work):
    """CLI 测试：vasp_fermisurface 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_fermi", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_fermisurface") / "INCAR").exists()


@pytest.mark.run
def test_vasp_fermisurface_run(chdir_shared):
    """实际运行测试：vasp_fermisurface --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_fermi", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_fermisurface") / "OUTCAR").exists()


# ============================================================================
# vasp_cohp_uniform / vasp_cohp_fatband 测试
# ============================================================================

def test_vasp_cohp_uniform_api(chdir_work):
    """API 测试：vasp_cohp_uniform 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp.vasp_cohp_uniform import VaspCohpUniformSetup

    setup = VaspCohpUniformSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_cohp_uniform") / "INCAR").exists()
    incar_content = (Path("vasp_cohp_uniform") / "INCAR").read_text()
    assert "ISYM" in incar_content


def test_vasp_cohp_uniform_cli(chdir_work, monkeypatch):
    """CLI 测试：vasp_cohp_uniform 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp import vasp_cohp_uniform as module

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = cwd

        def communicate(self, input=None):
            Path(self.cwd, "KPOINTS").write_text("fake uniform kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        return FakeProcess(cwd)

    monkeypatch.setattr("vaspflow.cohp.cohp_common.subprocess.Popen", fake_popen)
    monkeypatch.setattr(sys, "argv", ["vasp_cohp_uniform", "-i", "MgB2.vasp", "-c", "input_vasp.toml"])

    module.main()
    assert (Path("vasp_cohp_uniform") / "INCAR").exists()


@pytest.mark.run
def test_vasp_cohp_uniform_run(chdir_shared):
    """实际运行测试：vasp_cohp_uniform --run"""
    if shutil.which("vasp_cohp_uniform") is None:
        pytest.skip("当前环境未安装 vasp_cohp_uniform CLI。")

    if not (Path("vasp_scf") / "CHGCAR").exists():
        scf_result = subprocess.run(
            ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
            capture_output=True,
            text=True,
            timeout=600,
        )
        if scf_result.returncode != 0:
            pytest.skip("当前环境无法完成前置 vasp_scf 真实计算，跳过 cohp_uniform --run 测试。")

    result = subprocess.run(
        ["vasp_cohp_uniform", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_cohp_uniform") / "OUTCAR").exists()


def test_vasp_cohp_fatband_api(chdir_work, monkeypatch):
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp.vasp_cohp_fatband import VaspCohpFatbandSetup

    captured = []

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = cwd

        def communicate(self, input=None):
            captured.append((self.cwd, input))
            if input == "303\n":
                Path(self.cwd, "KPATH.in").write_text("fake kpath\n", encoding="utf-8")
            else:
                Path(self.cwd, "KPOINTS").write_text("fake fatband kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        assert cmd == ["vaspkit"]
        return FakeProcess(cwd)

    monkeypatch.setattr("vaspflow.cohp.cohp_common.subprocess.Popen", fake_popen)

    setup = VaspCohpFatbandSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_cohp_fatband") / "INCAR").exists()
    assert (Path("vasp_cohp_fatband") / "KPATH.in").exists()
    assert (Path("vasp_cohp_fatband") / "KPOINTS").exists()
    assert captured[0][1] == "303\n"
    assert captured[1][1] == "251\n1\n0.03\n0.03\n"


def test_vasp_cohp_fatband_cli(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_cohp_fatband as module

    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = cwd

        def communicate(self, input=None):
            if input == "303\n":
                Path(self.cwd, "KPATH.in").write_text("fake kpath\n", encoding="utf-8")
            else:
                Path(self.cwd, "KPOINTS").write_text("fake fatband kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        return FakeProcess(cwd)

    monkeypatch.setattr("vaspflow.cohp.cohp_common.subprocess.Popen", fake_popen)

    monkeypatch.setattr(sys, "argv", ["vasp_cohp_fatband", "-i", "MgB2.vasp", "-c", "input_vasp.toml"])

    module.main()
    assert (Path("vasp_cohp_fatband") / "INCAR").exists()


# ============================================================================
# vasp_locpot 测试
# ============================================================================

def test_vasp_locpot_api(chdir_work):
    """API 测试：vasp_locpot 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.locpot.vasp_locpot import VaspLocpotSetup

    setup = VaspLocpotSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_locpot") / "INCAR").exists()
    incar_content = (Path("vasp_locpot") / "INCAR").read_text()
    assert "LVHAR" in incar_content
    assert "NGX" in incar_content


def test_vasp_locpot_cli(chdir_work):
    """CLI 测试：vasp_locpot 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_locpot", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_locpot") / "INCAR").exists()


@pytest.mark.run
def test_vasp_locpot_run(chdir_shared):
    """实际运行测试：vasp_locpot --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_locpot", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_locpot") / "OUTCAR").exists()


# ============================================================================
# vasp_charge 测试
# ============================================================================

def test_vasp_charge_api(chdir_work):
    """API 测试：vasp_charge 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.charge.vasp_charge import VaspChargeSetup

    setup = VaspChargeSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    # 目录名包含能量范围
    charge_dirs = list(Path(".").glob("vasp_charge*"))
    assert len(charge_dirs) > 0

    charge_dir = charge_dirs[0]
    assert (charge_dir / "INCAR").exists()
    incar_content = (charge_dir / "INCAR").read_text()
    assert "LPARD" in incar_content
    assert "EINT" in incar_content


def test_vasp_charge_cli(chdir_work):
    """CLI 测试：vasp_charge 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_charge", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    charge_dirs = list(Path(".").glob("vasp_charge*"))
    assert len(charge_dirs) > 0


@pytest.mark.run
def test_vasp_charge_run(chdir_shared):
    """实际运行测试：vasp_charge --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_charge", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    charge_dirs = list(Path(".").glob("vasp_charge*"))
    assert len(charge_dirs) > 0
    assert (charge_dirs[0] / "OUTCAR").exists()


# ============================================================================
# vasp_elf 测试
# ============================================================================

def test_vasp_elf_api(chdir_work):
    """API 测试：vasp_elf 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.elf.vasp_elf import VaspElfSetup

    setup = VaspElfSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_elf") / "INCAR").exists()
    incar_content = (Path("vasp_elf") / "INCAR").read_text()
    assert "LELF" in incar_content


def test_vasp_elf_cli(chdir_work):
    """CLI 测试：vasp_elf 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_elf", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_elf") / "INCAR").exists()


@pytest.mark.run
def test_vasp_elf_run(chdir_shared):
    """实际运行测试：vasp_elf --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_elf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_elf") / "OUTCAR").exists()


# ============================================================================
# vasp_opt 测试（不运行 --run，太耗时）
# ============================================================================

def test_vasp_opt_rv4_api(chdir_work):
    """API 测试：vasp_opt rv4 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    Path("KPOINTS").write_text("external kpoints\n", encoding="utf-8")
    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rv4")

    assert (Path("vasp_opt") / "INCAR_1").exists()
    assert (Path("vasp_opt") / "INCAR_2").exists()
    assert (Path("vasp_opt") / "INCAR_3").exists()
    assert (Path("vasp_opt") / "INCAR_4").exists()
    assert (Path("vasp_opt") / "fouropt.sh").exists()
    assert not (Path("vasp_opt") / "KPOINTS").exists()

    incar_1 = (Path("vasp_opt") / "INCAR_1").read_text(encoding="utf-8")
    incar_2 = (Path("vasp_opt") / "INCAR_2").read_text(encoding="utf-8")
    incar_3 = (Path("vasp_opt") / "INCAR_3").read_text(encoding="utf-8")
    incar_4 = (Path("vasp_opt") / "INCAR_4").read_text(encoding="utf-8")

    assert "KSPACING = 0.8" in incar_1
    assert "KSPACING = 0.6" in incar_2
    assert "KSPACING = 0.4" in incar_3
    assert f"KSPACING = {0.04 * 2 * 3.1415926}" in incar_4


def test_vasp_opt_rv1_api(chdir_work):
    """API 测试：vasp_opt rv1 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    Path("KPOINTS").write_text("external kpoints\n", encoding="utf-8")
    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rv1")

    assert (Path("vasp_opt") / "INCAR").exists()
    assert (Path("vasp_opt") / "oneopt.sh").exists()
    assert not (Path("vasp_opt") / "KPOINTS").exists()

    incar_content = (Path("vasp_opt") / "INCAR").read_text(encoding="utf-8")
    assert f"KSPACING = {0.04 * 2 * 3.1415926}" in incar_content


def test_vasp_opt_rvf_api(chdir_work):
    """API 测试：vasp_opt rvf 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    Path("KPOINTS").write_text("external kpoints\n", encoding="utf-8")
    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rvf")

    assert (Path("vasp_opt") / "INCAR").exists()
    assert (Path("vasp_opt") / "fopt.sh").exists()
    assert not (Path("vasp_opt") / "KPOINTS").exists()

    incar_content = (Path("vasp_opt") / "INCAR").read_text(encoding="utf-8")
    assert f"KSPACING = {0.04 * 2 * 3.1415926}" in incar_content


def test_vasp_opt_prefers_explicit_kspacing_over_kmesh(chdir_work):
    from pathlib import Path

    config_path = Path("input_vasp_opt_kspacing.toml")
    content = Path("input_vasp.toml").read_text(encoding="utf-8").replace(
        "[vasp_opt]\nkmesh = 0.04",
        "[vasp_opt]\nkmesh = 0.04\nKSPACING = 0.23",
    )
    config_path.write_text(content, encoding="utf-8")

    from vaspflow.opt.vasp_opt import VaspOptSetup

    setup = VaspOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.run(mode="rv1")

    incar_content = (Path("vasp_opt") / "INCAR").read_text(encoding="utf-8")
    assert "KSPACING = 0.23" in incar_content


def test_vasp_opt_cli(chdir_work):
    """CLI 测试：vasp_opt 命令行"""
    result = subprocess.run(
        ["vasp_opt", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--mode", "rv4"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_opt") / "INCAR_1").exists()


def test_vasp_opt_pressure_override_uses_gpa_input_and_writes_kbar(chdir_work):
    result = subprocess.run(
        ["vasp_opt", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--mode", "rv1", "-p", "1.5"],
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert (Path("1.5") / "vasp_opt" / "INCAR").exists()
    incar_content = (Path("1.5") / "vasp_opt" / "INCAR").read_text(encoding="utf-8")
    assert "PSTRESS  = 15.0" in incar_content


def test_vasp_opt_multiple_pressures_generate_multiple_cases(chdir_work):
    result = subprocess.run(
        ["vasp_opt", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--mode", "rv1", "-p", "0", "1.5", "3"],
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert (Path("0") / "vasp_opt" / "INCAR").exists()
    assert (Path("1.5") / "vasp_opt" / "INCAR").exists()
    assert (Path("3") / "vasp_opt" / "INCAR").exists()
    assert Path("submit_all_vasp_opt.sh").exists()

    incar_0 = (Path("0") / "vasp_opt" / "INCAR").read_text(encoding="utf-8")
    incar_15 = (Path("1.5") / "vasp_opt" / "INCAR").read_text(encoding="utf-8")
    incar_3 = (Path("3") / "vasp_opt" / "INCAR").read_text(encoding="utf-8")
    assert "PSTRESS  = 0.0" in incar_0
    assert "PSTRESS  = 15.0" in incar_15
    assert "PSTRESS  = 30.0" in incar_3

    submit_script = Path("submit_all_vasp_opt.sh").read_text(encoding="utf-8")
    assert 'cd "' in submit_script
    assert "bash oneopt.sh" not in submit_script
    assert "bash fouropt.sh" not in submit_script
    assert "/home/mayuan/soft/vasp.6.3.2/bin/vasp_std" in submit_script
