import subprocess
import sys
from pathlib import Path

import pytest


def write_config(path: Path, content: str) -> None:
    path.write_text(content.strip() + "\n", encoding="utf-8")


def write_outcar(path: Path, energy: float) -> None:
    path.write_text(
        (
            " free  energy   TOTEN  =      -10.0000 eV\n"
            f"  energy  without entropy=     {energy: .8f}  energy(sigma->0) = {energy: .8f}\n"
        ),
        encoding="utf-8",
    )


def test_vasp_scf_converge_api_explicit_values(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.setup()

    assert (Path("ENCUT") / "400" / "INCAR").exists()
    assert (Path("ENCUT") / "450" / "INCAR").exists()
    assert (Path("ENCUT") / "500" / "INCAR").exists()


def test_vasp_scf_converge_api_range_values(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = [0.02, 0.04, 0.01]
ENCUT = 520
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    assert setup.scan_values == [0.02, 0.03, 0.04]
    setup.setup()

    assert (Path("kmesh") / "0.02" / "KPOINTS").exists()
    assert (Path("kmesh") / "0.03" / "KPOINTS").exists()
    assert (Path("kmesh") / "0.04" / "KPOINTS").exists()


def test_vasp_scf_converge_rejects_multiple_lists(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = [0.02, 0.04, 0.01]
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    with pytest.raises(ValueError, match="仅允许一个列表参数"):
        VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")


def test_vasp_scf_converge_cli(chdir_work):
    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    result = subprocess.run(
        ["vasp_scf_converge", "-i", "MgB2.vasp", "-c", str(config_path)],
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert (Path("ENCUT") / "400" / "INCAR").exists()


def test_vasp_scf_converge_plot_generates_png(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    for value, energy in [(400, -40.0), (450, -41.0), (500, -42.0)]:
        case_dir = Path(setup._build_case_dir(value))
        case_dir.mkdir(parents=True, exist_ok=True)
        poscar_path = case_dir / "POSCAR"
        poscar_path.write_text(Path("MgB2.vasp").read_text(encoding="utf-8"), encoding="utf-8")
        write_outcar(case_dir / "OUTCAR", energy)

    output_path = setup.plot()
    assert Path(output_path).exists()
    assert Path(output_path).name == "converge_ENCUT.png"


def test_vasp_scf_converge_plot_warns_and_skips_missing_outcar(chdir_work, capsys):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    for value, energy in [(400, -40.0), (500, -42.0)]:
        case_dir = Path(setup._build_case_dir(value))
        case_dir.mkdir(parents=True, exist_ok=True)
        (case_dir / "POSCAR").write_text(Path("MgB2.vasp").read_text(encoding="utf-8"), encoding="utf-8")
        write_outcar(case_dir / "OUTCAR", energy)

    output_path = setup.plot()
    captured = capsys.readouterr()
    assert "警告: 跳过" in captured.out
    assert Path(output_path).exists()


def test_vasp_scf_converge_collect_plot_data_uses_highest_accuracy_reference(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    for value, energy in [(400, -20.0000), (450, -20.0015), (500, -20.0020)]:
        case_dir = Path(setup._build_case_dir(value))
        case_dir.mkdir(parents=True, exist_ok=True)
        (case_dir / "POSCAR").write_text(Path("MgB2.vasp").read_text(encoding="utf-8"), encoding="utf-8")
        write_outcar(case_dir / "OUTCAR", energy)

    plot_data = setup._collect_plot_data()

    assert plot_data["reference_point"]["x"] == 500
    assert pytest.approx(plot_data["reference_energy"], rel=0, abs=1e-12) == (-20.0020 / 3.0)
    delta_by_x = {point["x"]: point["delta_mev_per_atom"] for point in plot_data["points"]}
    assert pytest.approx(delta_by_x[500], rel=0, abs=1e-12) == 0.0
    assert pytest.approx(delta_by_x[450], rel=0, abs=1e-9) == ((-20.0015 / 3.0) - (-20.0020 / 3.0)) * 1000.0


def test_vasp_scf_converge_collect_plot_data_marks_first_converged_point(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    for value, energy in [(400, -19.9980), (450, -20.001998), (500, -20.0020)]:
        case_dir = Path(setup._build_case_dir(value))
        case_dir.mkdir(parents=True, exist_ok=True)
        (case_dir / "POSCAR").write_text(Path("MgB2.vasp").read_text(encoding="utf-8"), encoding="utf-8")
        write_outcar(case_dir / "OUTCAR", energy)

    plot_data = setup._collect_plot_data()
    assert plot_data["first_converged_point"] is not None
    assert plot_data["first_converged_point"]["x"] == 450
    assert abs(plot_data["first_converged_point"]["delta_mev_per_atom"]) <= 1.0


def test_vasp_scf_converge_creates_root_submit_script(chdir_work):
    from vaspflow.scf.vasp_scf_converge import VaspScfConvergeSetup

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        '''
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[slurm]
header = """
#!/bin/bash
#SBATCH --job-name=test
"""

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        ''',
    )

    setup = VaspScfConvergeSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.setup()

    submit_script = Path("ENCUT") / "submit_all.sh"
    assert submit_script.exists()
    content = submit_script.read_text(encoding="utf-8")
    assert 'cd "400"' in content
    assert 'cd "450"' in content
    assert 'cd "500"' in content
    assert content.count('sbatch "run_vasp.sh"') == 3
    assert content.count('cd ..') == 3


def test_vasp_scf_converge_submit_uses_root_submit_script(chdir_work, monkeypatch):
    from vaspflow.scf import vasp_scf_converge as module

    config_path = Path("input_converge.toml")
    write_config(
        config_path,
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[vasp_scf_converge]
kmesh = 0.04
ENCUT = [400, 450, 500]
ISMEAR = 0
SIGMA = 0.05
        """,
    )

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(sys, "argv", ["vasp_scf_converge", "-i", "MgB2.vasp", "-c", str(config_path), "-s"])

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "ENCUT"
    assert captured["script_name"] == "submit_all.sh"
