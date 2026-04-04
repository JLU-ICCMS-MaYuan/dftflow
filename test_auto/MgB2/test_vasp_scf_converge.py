import subprocess
from pathlib import Path

import pytest


def write_config(path: Path, content: str) -> None:
    path.write_text(content.strip() + "\n", encoding="utf-8")


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

    assert (Path("ENCUT") / "400" / "vasp_scf" / "INCAR").exists()
    assert (Path("ENCUT") / "450" / "vasp_scf" / "INCAR").exists()
    assert (Path("ENCUT") / "500" / "vasp_scf" / "INCAR").exists()


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

    assert (Path("kmesh") / "0.02" / "vasp_scf" / "KPOINTS").exists()
    assert (Path("kmesh") / "0.03" / "vasp_scf" / "KPOINTS").exists()
    assert (Path("kmesh") / "0.04" / "vasp_scf" / "KPOINTS").exists()


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
    assert (Path("ENCUT") / "400" / "vasp_scf" / "INCAR").exists()
