import subprocess
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
