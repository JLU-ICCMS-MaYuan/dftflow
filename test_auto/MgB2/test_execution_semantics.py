import os
import pytest

from public.executables import resolve_qe_command, resolve_vasp_command
from public.job import execute_commands


def test_vasp_executable_path_is_treated_as_full_command():
    command = "mpirun -np 8 /home/mayuan/soft/vasp.6.3.2/bin/vasp_std"
    assert resolve_vasp_command(command) == command


def test_qe_executable_path_appends_binary_to_bin_command():
    command = "mpirun -np 8 ~/soft/qe-7.4.1/bin"
    resolved = resolve_qe_command(command, "pw.x")

    assert resolved.startswith("mpirun -np 8 ")
    assert resolved.endswith("/soft/qe-7.4.1/bin/pw.x")


def test_execute_commands_raises_when_expected_outputs_are_missing(tmp_path):
    work_dir = tmp_path / "job"
    work_dir.mkdir()

    with pytest.raises(RuntimeError, match="未检测到预期结果"):
        execute_commands(
            str(work_dir),
            ["true"],
            expected_outputs=["OUTCAR"],
        )


def test_execute_commands_accepts_updated_outputs(tmp_path):
    work_dir = tmp_path / "job"
    work_dir.mkdir()

    execute_commands(
        str(work_dir),
        ["touch OUTCAR"],
        expected_outputs=["OUTCAR"],
    )

    assert os.path.exists(work_dir / "OUTCAR")
