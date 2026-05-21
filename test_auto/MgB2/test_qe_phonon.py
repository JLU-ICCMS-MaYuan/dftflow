import os
import csv
import re
import shutil
import sys
import tomllib
from pathlib import Path


MINIMAL_QGRID_XML = """<root>
  <output>
    <basis_set>
      <reciprocal_lattice>
        <b1>1.0 0.0 0.0</b1>
        <b2>0.0 1.0 0.0</b2>
        <b3>0.0 0.0 1.0</b3>
      </reciprocal_lattice>
    </basis_set>
    <symmetries>
      <symmetry>
        <rotation>1 0 0 0 1 0 0 0 1</rotation>
      </symmetry>
      <symmetry>
        <rotation>-1 0 0 0 -1 0 0 0 -1</rotation>
      </symmetry>
    </symmetries>
  </output>
  <time_reversal>true</time_reversal>
</root>
"""


def _write_kpath(path: Path) -> None:
    path.write_text("0.0 0.0 0.0 G\n0.5 0.0 0.0 X\n", encoding="utf-8")


def _prepare_xml_fixture(root: Path, prefix: str = "Mg1B2") -> None:
    save_dir = root / "qe_phonon" / "out" / f"{prefix}.save"
    save_dir.mkdir(parents=True, exist_ok=True)
    (save_dir / "data-file-schema.xml").write_text(MINIMAL_QGRID_XML, encoding="utf-8")


def _write_dyn0_fixture(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "   2   2   2",
                "   4",
                "   0.000000000000000E+00   0.000000000000000E+00   0.000000000000000E+00",
                "   0.000000000000000E+00   0.000000000000000E+00   5.000000000000000E-01",
                "   0.000000000000000E+00   5.000000000000000E-01   0.000000000000000E+00",
                "   0.000000000000000E+00   5.000000000000000E-01   5.000000000000000E-01",
            ]
        )
        + "\n",
        encoding="utf-8",
    )


def _write_job_done(path: Path) -> None:
    path.write_text("...\nJOB DONE.\n", encoding="utf-8")


def _write_split_merge_outputs(q_dir: Path, prefix: str, index: int) -> None:
    _write_job_done(q_dir / "scf.out")
    _write_job_done(q_dir / "split_ph.out")
    (q_dir / f"{prefix}.dyn{index}").write_text(
        "freq ( 1) = 1.0000\nfreq ( 2) = 2.5000\nfreq ( 3) = -0.5000\n",
        encoding="utf-8",
    )
    phsave_dir = q_dir / "_ph0" / f"{prefix}.phsave"
    phsave_dir.mkdir(parents=True, exist_ok=True)
    (phsave_dir / "control_ph.xml").write_text("phsave\n", encoding="utf-8")
    qph_dir = q_dir / "_ph0" / f"{prefix}.q_{index}"
    qph_dir.mkdir(parents=True, exist_ok=True)
    (qph_dir / "dvscf").write_text("dvscf\n", encoding="utf-8")
    (phsave_dir / f"patterns.{index}.xml").write_text(
        f"<?xml version='1.0' encoding='UTF-8'?><ROOT><QPOINT_NUMBER>{index}</QPOINT_NUMBER></ROOT>",
        encoding="utf-8",
    )


def _write_split_merge_outputs_coords(q_dir: Path, prefix: str, index: int, with_qdir: bool = False) -> None:
    _write_job_done(q_dir / "scf.out")
    _write_job_done(q_dir / "split_ph.out")
    (q_dir / f"{prefix}.dyn").write_text(
        "freq ( 1) = 1.0000\nfreq ( 2) = 2.5000\nfreq ( 3) = -0.5000\n",
        encoding="utf-8",
    )
    phsave_dir = q_dir / "_ph0" / f"{prefix}.phsave"
    phsave_dir.mkdir(parents=True, exist_ok=True)
    (phsave_dir / "control_ph.xml").write_text("phsave\n", encoding="utf-8")
    (phsave_dir / "patterns.1.xml").write_text(
        "<?xml version='1.0' encoding='UTF-8'?><ROOT><QPOINT_NUMBER>1</QPOINT_NUMBER></ROOT>",
        encoding="utf-8",
    )
    if with_qdir:
        qph_dir = q_dir / "_ph0" / f"{prefix}.q_{index}"
        qph_dir.mkdir(parents=True, exist_ok=True)
        (qph_dir / "dvscf").write_text("dvscf\n", encoding="utf-8")
    else:
        (q_dir / "_ph0").mkdir(parents=True, exist_ok=True)
        (q_dir / "_ph0" / "dvscf").write_text("dvscf\n", encoding="utf-8")


def _write_phband_fixture(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                " &plot nbnd=   9, nks=   2 /",
                "            0.000000  0.000000  0.000000",
                "    3.3356    6.6712   10.0068   13.3424   16.6780   20.0136",
                "   23.3492   26.6848   30.0204",
                "            0.500000  0.000000  0.000000",
                "    5.0034    8.3390   11.6746   15.0102   18.3458   21.6814",
                "   25.0170   28.3526   31.6882",
            ]
        )
        + "\n",
        encoding="utf-8",
    )


def _write_phband_legacy_fixture(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "freq (    1) = 0.100 [THz] = 3.3356 [cm-1]",
                "freq (    2) = 0.200 [THz] = 6.6712 [cm-1]",
                "freq (    3) = 0.300 [THz] = 10.0068 [cm-1]",
                "freq (    4) = 0.400 [THz] = 13.3424 [cm-1]",
                "freq (    5) = 0.500 [THz] = 16.6780 [cm-1]",
                "freq (    6) = 0.600 [THz] = 20.0136 [cm-1]",
                "freq (    7) = 0.700 [THz] = 23.3492 [cm-1]",
                "freq (    8) = 0.800 [THz] = 26.6848 [cm-1]",
                "freq (    9) = 0.900 [THz] = 30.0204 [cm-1]",
            ]
        )
        + "\n",
        encoding="utf-8",
    )


def _write_labelinfo_fixture(path: Path) -> None:
    path.write_text(
        "GAMMA 1 0.000000 0.000000 0.000000 0.000000\nX 2 0.500000 0.500000 0.000000 0.000000\n",
        encoding="utf-8",
    )


def _write_phdos_fixture(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "# freq TDOS Mg1 B1 B2",
                "0.0 0.1 0.01 0.02 0.03",
                "10.0 0.4 0.10 0.20 0.30",
                "20.0 0.2 0.05 0.04 0.03",
            ]
        )
        + "\n",
        encoding="utf-8",
    )


def _write_phonon_postprocess_fixtures(cwd_path: Path) -> None:
    _write_job_done(cwd_path / "matdyn_phband.out")
    _write_job_done(cwd_path / "matdyn_phdos.out")
    _write_phband_fixture(cwd_path / "Mg1B2.phband.freq")
    _write_labelinfo_fixture(cwd_path / "Mg1B2_band.labelinfo.dat")
    _write_phdos_fixture(cwd_path / "Mg1B2.phdos.dat")


def test_qe_phonon_template_override_priority(chdir_work):
    from public.config_templates import load_config_file
    from qeflow.templates import apply_qe_config_overrides, get_qe_phonon_parameters_template

    config_path = Path("tmp_qe_phonon_template.toml")
    config_path.write_text(
        """
pp_dir = "/pseudo"

[qe]
executable_path = "mpirun -np 8 /path/to/qe/bin"

[qe_phonon]
work_dir = "qe_phonon_custom"

[qe_phonon.scf]
ecutwfc = 60.0

[qe_phonon.ph]
qpoints = [2, 3, 4]
""".strip(),
        encoding="utf-8",
    )

    merged = apply_qe_config_overrides(
        get_qe_phonon_parameters_template(),
        load_config_file(str(config_path)),
        ("qe", "slurm", "pp_dir", "qe_phonon"),
    )

    assert merged["pp_dir"] == "/pseudo"
    assert merged["qe_phonon"]["work_dir"] == "qe_phonon_custom"
    assert merged["qe_phonon"]["scf"]["ecutwfc"] == 60.0
    assert merged["qe_phonon"]["ph"]["qpoints"] == [2, 3, 4]


def test_qe_phonon_template_default_work_dir_is_deferred(chdir_work):
    from qeflow.templates import get_qe_phonon_parameters_template

    template = get_qe_phonon_parameters_template()
    assert template["qe_phonon"]["work_dir"] is None


def test_qe_phonon_cli_registration_uses_short_split_names(chdir_work):
    scripts = tomllib.loads(Path("/home/mayuan/code/dftflow/pyproject.toml").read_text(encoding="utf-8"))["project"]["scripts"]

    assert "qe_phonon_nosplit" in scripts
    assert scripts["qe_phonon_nosplit"] == "qeflow.phonon.qe_phonon_nosplit:main"
    assert "qe_phonon_split_coords" in scripts
    assert scripts["qe_phonon_split_coords"] == "qeflow.phonon.qe_phonon_split_assign_coords:main"
    assert "qe_phonon_split_index" in scripts
    assert scripts["qe_phonon_split_index"] == "qeflow.phonon.qe_phonon_split_assign_index:main"
    assert "qe_ph_split_assign_coords" not in scripts
    assert "qe_ph_split_assign_index" not in scripts
    assert "qe_phonon_split_assign_coords" not in scripts
    assert "qe_phonon_split_assign_index" not in scripts
    assert scripts["qe_process_phonon"] == "qeflow.phonon.process_phonon:main"
    assert scripts["qe_split_submit"] == "qeflow.phonon.qe_split_submit:main"


def test_parse_dyn0_qpoints_uses_ph_generated_order(chdir_work):
    from qeflow.phonon._shared import parse_dyn0_qpoints

    dyn0_path = Path("Mg1B2.dyn0")
    dyn0_path.write_text(
        "\n".join(
            [
                "   2   2   2",
                "   4",
                "   0.000000000000000E+00   0.000000000000000E+00   0.000000000000000E+00",
                "   0.000000000000000E+00   0.000000000000000E+00   5.000000000000000E-01",
                "   0.000000000000000E+00   5.000000000000000E-01   0.000000000000000E+00",
                "   0.000000000000000E+00   5.000000000000000E-01   5.000000000000000E-01",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    qpoints = parse_dyn0_qpoints(str(dyn0_path))
    assert len(qpoints) == 4
    assert qpoints[0]["index"] == 1
    assert qpoints[1]["q_crystal"] == [0.0, 0.0, 0.5]


def test_qe_phonon_import_boundaries():
    phonon_root = Path("qeflow/phonon")
    assert "from qeflow.phonon.workflow import" not in (phonon_root / "ph_step.py").read_text(encoding="utf-8")
    assert "from qeflow.phonon.workflow import" not in (phonon_root / "split_prepare.py").read_text(encoding="utf-8")
    assert "from qeflow.phonon.workflow import" not in (phonon_root / "qe_merge.py").read_text(encoding="utf-8")
    assert "from qeflow.phonon.workflow import" not in (phonon_root / "qe_q2r.py").read_text(encoding="utf-8")
    assert "from qeflow.phonon.workflow import" not in (phonon_root / "qe_matdyn_phband.py").read_text(encoding="utf-8")
    assert "from qeflow.phonon.workflow import" not in (phonon_root / "qe_matdyn_phdos.py").read_text(encoding="utf-8")
    assert "ph_step" in (phonon_root / "qe_phonon_nosplit.py").read_text(encoding="utf-8")
    assert "split_prepare" in (phonon_root / "qe_phonon_split_assign_index.py").read_text(encoding="utf-8")
    assert "split_prepare" in (phonon_root / "qe_phonon_split_assign_coords.py").read_text(encoding="utf-8")


def test_ph_step_generates_ph_input_and_script(chdir_work):
    from qeflow.phonon.ph_step import run_ph_nosplit_step

    run_ph_nosplit_step("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False, dyn0_only=False)

    workdir = Path("qe_phonon")
    assert (workdir / "scf.in").exists()
    assert (workdir / "ph.in").exists()
    assert (workdir / "run_ph.sh").exists()
    assert "fildyn = 'Mg1B2.dyn'" in (workdir / "ph.in").read_text(encoding="utf-8")
    scf_text = (workdir / "scf.in").read_text(encoding="utf-8")
    assert "Mg   24.3050 mg_pbe_v1.4.uspp" in scf_text
    assert "B    10.8100 b_pbe_v1.4.uspp.F.UPF" in scf_text


def test_ph_step_dyn0_run_uses_background_stop(chdir_work, monkeypatch):
    from qeflow.phonon import ph_step as module

    captured = {}

    def fake_run_ph_to_dyn0(workspace, ph_command, dyn0_path, output_path):
        captured["workspace"] = workspace
        captured["ph_command"] = ph_command
        Path(dyn0_path).write_text("dyn0\n", encoding="utf-8")
        Path(output_path).write_text("partial\n", encoding="utf-8")

    monkeypatch.setattr(module, "run_ph_to_dyn0", fake_run_ph_to_dyn0)

    module.run_ph_nosplit_step("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False, dyn0_only=True)

    assert captured["workspace"].endswith("qe_phonon")
    assert Path("qe_phonon/Mg1B2.dyn0").exists()


def test_split_prepare_index_materializes_split_dirs(chdir_work):
    from qeflow.phonon import _shared
    from qeflow.phonon.split_prepare import prepare_split_assign_index_step
    from qeflow.qe_input.fileio import load_json

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")

    prepare_split_assign_index_step(_shared.create_context(str(workdir / "input.toml"), None))

    assert (workdir / "01" / "scf.in").exists()
    assert (workdir / "01" / "split_ph.in").exists()
    assert (workdir / "01" / "run_split.sh").exists()
    assert "start_q = 1" in (workdir / "01" / "split_ph.in").read_text(encoding="utf-8")
    q_map = load_json(str(workdir / "q_index_map.json"))
    assert q_map["mode"] == "split_assign_index"
    assert q_map["qpoints"][0]["directory"] == "01"


def test_split_prepare_coords_writes_single_q_coordinate(chdir_work):
    from qeflow.phonon import _shared
    from qeflow.phonon.split_prepare import prepare_split_assign_coords_step
    from qeflow.qe_input.fileio import load_json

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")

    prepare_split_assign_coords_step(_shared.create_context(str(workdir / "input.toml"), None))

    content = (workdir / "01" / "split_ph.in").read_text(encoding="utf-8")
    assert "ldisp = .false." in content
    assert "0.000000000000000E+00" in content
    q_map = load_json(str(workdir / "q_index_map.json"))
    assert q_map["mode"] == "split_assign_coords"
    assert q_map["qpoints"][0]["directory"] == "01"


def test_qe_q2r_and_matdyn_generate_inputs(chdir_work, monkeypatch):
    from qeflow.phonon.qe_matdyn_phband import run_matdyn_phband_step
    from qeflow.phonon.qe_matdyn_phdos import run_matdyn_phdos_step
    from qeflow.phonon.qe_q2r import run_q2r_step

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_kpath(workdir / "KPATH.in")
    (workdir / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
    monkeypatch.setattr(
        "qeflow.qe_input.builders.run_vaspkit_for_kpath",
        lambda target_dir: None,
    )

    run_q2r_step(str(workdir / "input.toml"), None, run_calc=False, submit_job=False)
    run_matdyn_phband_step(str(workdir / "input.toml"), None, run_calc=False, submit_job=False)
    run_matdyn_phdos_step(str(workdir / "input.toml"), None, run_calc=False, submit_job=False)

    assert (workdir / "q2r.in").exists()
    assert (workdir / "matdyn_phband.in").exists()
    assert (workdir / "matdyn_phdos.in").exists()


def test_output_checks_use_three_states(chdir_work):
    from qeflow.qe_check.checks import check_ph, check_q2r, check_relax, check_scf

    workdir = Path("qe_phonon")
    workdir.mkdir()
    _write_job_done(workdir / "scf.out")
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "q2r.out")
    (workdir / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
    (Path("vc-relax.out")).write_text("End final coordinates\nJOB DONE.\n", encoding="utf-8")

    assert check_scf(str(workdir)) == "finished"
    assert check_relax(str(Path("."))) == "finished"
    assert check_ph(str(workdir), "Mg1B2")["dyn0"] == "finished"
    assert check_q2r(str(workdir), "Mg1B2") == "finished"

    empty_dir = workdir / "empty"
    empty_dir.mkdir()
    assert check_scf(str(empty_dir)) == "unstarted"


def test_qe_check_cli_reports_supported_status(chdir_work, monkeypatch, capsys):
    from qeflow.qe_check import cli as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")

    monkeypatch.setattr(sys, "argv", ["qe_check", "-c", str(workdir / "input.toml"), "-scf"])
    module.main()
    assert capsys.readouterr().out.strip() == "finished"

    monkeypatch.setattr(sys, "argv", ["qe_check", "-c", str(workdir / "input.toml"), "-eph"])
    module.main()
    assert "暂未支持" in capsys.readouterr().out


def test_qe_phonon_nosplit_prepares_all_root_inputs(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_nosplit as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    module.run_phonon_nosplit_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)

    workdir = Path("qe_phonon_nosplit")
    for filename in (
        "scf.in",
        "ph.in",
        "q2r.in",
        "matdyn_phband.in",
        "matdyn_phdos.in",
        "run_scf.sh",
        "run_ph.sh",
        "run_q2r.sh",
        "run_matdyn_phband.sh",
        "run_matdyn_phdos.sh",
    ):
        assert (workdir / filename).exists()


def test_qe_phonon_nosplit_run_pushes_full_chain_from_scf(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_phonon_nosplit as module

    workdir = Path("qe_phonon_nosplit")
    executed = []
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_run_shell(command, cwd):
        executed.append(command)
        cwd_path = Path(cwd)
        if command == "bash run_scf.sh":
            _write_job_done(cwd_path / "scf.out")
        elif command == "bash run_ph.sh":
            _write_job_done(cwd_path / "ph.out")
            (cwd_path / "Mg1B2.dyn0").write_text("dyn0\n", encoding="utf-8")
        elif command == "bash run_q2r.sh":
            _write_job_done(cwd_path / "q2r.out")
            (cwd_path / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
        elif command == "bash run_matdyn_phband.sh":
            _write_job_done(cwd_path / "matdyn_phband.out")
            _write_phband_fixture(cwd_path / "Mg1B2.phband.freq")
            _write_labelinfo_fixture(cwd_path / "Mg1B2_band.labelinfo.dat")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            _write_phdos_fixture(cwd_path / "Mg1B2.phdos.dat")

    monkeypatch.setattr(_shared, "run_shell", fake_run_shell)
    module.run_phonon_nosplit_workflow("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False)

    assert executed == [
        "bash run_scf.sh",
        "bash run_ph.sh",
        "bash run_q2r.sh",
        "bash run_matdyn_phband.sh",
        "bash run_matdyn_phdos.sh",
    ]
    assert (workdir / "Mg1B2.phdos.dat").exists()


def test_qe_phonon_nosplit_run_continues_from_ph_and_finishes(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_phonon_nosplit as module

    workdir = Path("qe_phonon_nosplit")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    executed = []

    def fake_run_shell(command, cwd):
        executed.append(command)
        cwd_path = Path(cwd)
        if command == "bash run_ph.sh":
            _write_job_done(cwd_path / "ph.out")
            (cwd_path / "Mg1B2.dyn0").write_text("dyn0\n", encoding="utf-8")
        elif command == "bash run_q2r.sh":
            _write_job_done(cwd_path / "q2r.out")
            (cwd_path / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
        elif command == "bash run_matdyn_phband.sh":
            _write_job_done(cwd_path / "matdyn_phband.out")
            _write_phband_fixture(cwd_path / "Mg1B2.phband.freq")
            _write_labelinfo_fixture(cwd_path / "Mg1B2_band.labelinfo.dat")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            _write_phdos_fixture(cwd_path / "Mg1B2.phdos.dat")

    monkeypatch.setattr(_shared, "run_shell", fake_run_shell)
    module.run_phonon_nosplit_workflow(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    assert executed == [
        "bash run_ph.sh",
        "bash run_q2r.sh",
        "bash run_matdyn_phband.sh",
        "bash run_matdyn_phdos.sh",
    ]


def test_qe_phonon_nosplit_run_continues_from_q2r_and_finishes(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_phonon_nosplit as module

    workdir = Path("qe_phonon_nosplit")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")
    _write_job_done(workdir / "ph.out")
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "q2r.out")
    (workdir / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    executed = []

    def fake_run_shell(command, cwd):
        executed.append(command)
        cwd_path = Path(cwd)
        if command == "bash run_matdyn_phband.sh":
            _write_job_done(cwd_path / "matdyn_phband.out")
            _write_phband_fixture(cwd_path / "Mg1B2.phband.freq")
            _write_labelinfo_fixture(cwd_path / "Mg1B2_band.labelinfo.dat")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            _write_phdos_fixture(cwd_path / "Mg1B2.phdos.dat")

    monkeypatch.setattr(_shared, "run_shell", fake_run_shell)
    module.run_phonon_nosplit_workflow(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    assert executed == [
        "bash run_matdyn_phband.sh",
        "bash run_matdyn_phdos.sh",
    ]


def test_qe_process_phonon_script_archives_after_each_matdyn_step(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import process_phonon as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))
    context = _shared.create_context("input_qe.toml", "MgB2.vasp")

    script_path = Path(module.prepare_process_phonon_step(context))
    script = script_path.read_text(encoding="utf-8")

    assert script.index("bash run_q2r.sh") < script.index("bash run_matdyn_phband.sh")
    assert script.index("bash run_matdyn_phband.sh") < script.index("--collect phband")
    assert script.index("--collect phband") < script.index("bash run_matdyn_phdos.sh")
    assert script.index("bash run_matdyn_phdos.sh") < script.index("--collect phdos")
    assert script.index("--collect phdos") < script.index("--plot")


def test_qe_process_phonon_collects_plots_and_marks_status(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import process_phonon as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))
    context = _shared.create_context("input_qe.toml", "MgB2.vasp")
    workdir = Path(context.workspace)
    _write_job_done(workdir / "matdyn_phband.out")
    _write_job_done(workdir / "matdyn_phdos.out")
    _write_phband_fixture(workdir / "Mg1B2.phband.freq")
    _write_labelinfo_fixture(workdir / "Mg1B2_band.labelinfo.dat")
    _write_phdos_fixture(workdir / "Mg1B2.phdos.dat")

    module.collect_phband_outputs(context)
    module.collect_phdos_outputs(context)

    status = _shared.refresh_workflow_status(context.workspace, context.prefix)
    assert status["matdyn_phband"] == "finished"
    assert status["matdyn_phdos"] == "finished"
    assert status["phonon_plot"] == "unstarted"

    module.plot_phonon_outputs(context)
    status = _shared.refresh_workflow_status(context.workspace, context.prefix)

    assert status["phonon_plot"] == "finished"
    band_rows = list(csv.DictReader((workdir / "phband" / "phonon_band.csv").open(encoding="utf-8")))
    label_rows = list(csv.DictReader((workdir / "phband" / "phonon_band_labels.csv").open(encoding="utf-8")))
    atom_rows = list(csv.DictReader((workdir / "phdos" / "phonon_dos_atoms.csv").open(encoding="utf-8")))
    element_rows = list(csv.DictReader((workdir / "phdos" / "phonon_dos_elements.csv").open(encoding="utf-8")))

    assert band_rows[0]["mode_9_cm-1"] == "30.0204"
    assert label_rows[0]["label"] == "Γ"
    assert list(atom_rows[0].keys()) == ["frequency_cm-1", "TDOS", "Mg_1", "B_1", "B_2"]
    assert float(element_rows[1]["B"]) == 0.5
    assert (workdir / "phband" / "phonon_band.png").exists()
    assert (workdir / "phdos" / "phonon_dos.png").exists()


def test_qe_process_phonon_keeps_legacy_freq_line_compatibility(chdir_work):
    from qeflow.phonon import process_phonon as module

    freq_path = Path("legacy.phband.freq")
    _write_phband_legacy_fixture(freq_path)

    rows = module._read_band_frequencies(str(freq_path), 9)

    assert rows == [[3.3356, 6.6712, 10.0068, 13.3424, 16.6780, 20.0136, 23.3492, 26.6848, 30.0204]]


def test_qe_process_phonon_collect_uses_current_split_index_workspace(chdir_work, monkeypatch):
    from qeflow.phonon import process_phonon as module

    workdir = Path("qe_phonon_split_index")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    (workdir / "q_index_map.json").write_text('{"mode":"split_assign_index","qpoints":[]}', encoding="utf-8")
    _write_job_done(workdir / "matdyn_phband.out")
    _write_phband_fixture(workdir / "Mg1B2.phband.freq")
    _write_labelinfo_fixture(workdir / "Mg1B2_band.labelinfo.dat")

    monkeypatch.chdir(workdir)

    result = module.run_process_phonon("input.toml", None, False, False, "phband", False)

    assert result == 0
    assert Path("phband/Mg1B2.phband.freq").exists()
    assert Path("phband/Mg1B2_band.labelinfo.dat").exists()
    assert not Path("qe_phonon/phband/Mg1B2.phband.freq").exists()


def test_qe_process_phonon_run_executes_q2r_matdyn_collect_and_plot(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import process_phonon as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))
    executed = []

    def fake_run_shell(command, cwd):
        executed.append(command)
        cwd_path = Path(cwd)
        if command == "bash run_q2r.sh":
            _write_job_done(cwd_path / "q2r.out")
            (cwd_path / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
        elif command == "bash run_matdyn_phband.sh":
            _write_job_done(cwd_path / "matdyn_phband.out")
            _write_phband_fixture(cwd_path / "Mg1B2.phband.freq")
            _write_labelinfo_fixture(cwd_path / "Mg1B2_band.labelinfo.dat")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            _write_phdos_fixture(cwd_path / "Mg1B2.phdos.dat")

    monkeypatch.setattr(_shared, "run_shell", fake_run_shell)

    result = module.run_process_phonon("input_qe.toml", "MgB2.vasp", True, False, False, False)

    assert result == 0
    assert executed == ["bash run_q2r.sh", "bash run_matdyn_phband.sh", "bash run_matdyn_phdos.sh"]
    assert Path("qe_phonon/phband/Mg1B2.phband.freq").exists()
    assert Path("qe_phonon/phdos/Mg1B2.phdos.dat").exists()
    assert Path("qe_phonon/phband/phonon_band.csv").exists()
    assert Path("qe_phonon/phdos/phonon_dos_elements.csv").exists()


def test_qe_phonon_nosplit_submit_writes_remaining_chain_script(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_nosplit as module

    submitted = []
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_submit_script(workdir, script_name):
        submitted.append((os.path.basename(workdir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    module.run_phonon_nosplit_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True)

    assert submitted == [("qe_phonon_nosplit", "submit_phonon_workflow.sh")]
    script = Path("qe_phonon_nosplit/submit_phonon_workflow.sh").read_text(encoding="utf-8")
    assert "bash run_scf.sh" in script
    assert "bash run_ph.sh" in script
    assert "bash submit_process_phonon.sh" in script
    process_script = Path("qe_phonon_nosplit/submit_process_phonon.sh").read_text(encoding="utf-8")
    assert "bash run_q2r.sh" in process_script
    assert "bash run_matdyn_phband.sh" in process_script
    assert "--collect phband" in process_script
    assert "bash run_matdyn_phdos.sh" in process_script
    assert "--collect phdos" in process_script
    assert "--plot" in process_script


def test_qe_phonon_split_prepares_all_root_inputs(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_split_assign_index as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))
    module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)

    workdir = Path("qe_phonon_split_index")
    for filename in (
        "scf.in",
        "ph.in",
        "q2r.in",
        "matdyn_phband.in",
        "matdyn_phdos.in",
        "run_scf.sh",
        "run_ph.sh",
        "run_q2r.sh",
        "run_matdyn_phband.sh",
        "run_matdyn_phdos.sh",
    ):
        assert (workdir / filename).exists()


def test_qe_phonon_split_index_uses_short_default_workdir_when_not_configured(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_split_assign_index as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)

    assert Path("qe_phonon_split_index/scf.in").exists()
    assert Path("qe_phonon_split_index/run_ph.sh").exists()


def test_qe_phonon_split_coords_uses_short_default_workdir_when_not_configured(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_split_assign_coords as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    module.run_phonon_split_assign_coords_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)

    assert Path("qe_phonon_split_coords/scf.in").exists()
    assert Path("qe_phonon_split_coords/run_ph.sh").exists()


def test_qe_phonon_split_submit_uses_four_stage_progression(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_split_submit
    from qeflow.phonon import split_prepare as split_module
    from qeflow.phonon import qe_phonon_split_assign_index as module

    submitted = []
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_submit_script(workdir, script_name):
        submitted.append((os.path.basename(workdir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    monkeypatch.setattr(_shared, "submit_script", fake_submit_script)
    monkeypatch.setattr(qe_split_submit, "submit_script", fake_submit_script)
    module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True)

    assert submitted == [("qe_phonon_split_index", "submit_pre_and_split_stage.sh")]
    pre_script = Path("qe_phonon_split_index/submit_pre_and_split_stage.sh").read_text(encoding="utf-8")
    assert "1/11" in pre_script
    assert "2/11" in pre_script
    assert "3/11" in pre_script
    assert "[qeflow] 1/11" in pre_script
    assert "[qeflow] 完成: 1/11-3/11" in pre_script
    assert "DFTFlow" not in pre_script
    assert "qe_split_submit" in pre_script

    workdir = Path("qe_phonon_split_index")
    _write_job_done(workdir / "scf.out")
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _prepare_xml_fixture(Path("."))
    split_module.prepare_split_assign_index_step(
        _shared.create_context("input_qe.toml", "MgB2.vasp", workflow_name="qe_phonon_split_index")
    )
    submitted.clear()

    module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True)
    assert submitted
    assert all(script_name == "run_split.sh" for _, script_name in submitted)
    assert {workdir for workdir, _ in submitted} == {"01", "02", "03", "04"}

    for q_dir in sorted(path for path in workdir.glob("[0-9][0-9]") if path.is_dir()):
        content = (q_dir / "split_ph.in").read_text(encoding="utf-8")
        match = re.search(r"start_q\s*=\s*(\d+)", content)
        assert match is not None
        index = int(match.group(1))
        _write_split_merge_outputs(q_dir, "Mg1B2", index)
    submitted.clear()

    module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True)
    assert submitted == [("qe_phonon_split_index", "submit_merge_and_process_phonon.sh")]
    status = _shared.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "unstarted"
    assert not (workdir / "submit_merge_stage.sh").exists()
    assert not (workdir / "run_merge.sh").exists()
    post_script = Path("qe_phonon_split_index/submit_merge_and_process_phonon.sh").read_text(encoding="utf-8")
    assert "qe_merge" in post_script
    assert "4/11" in post_script
    assert "11/11" in post_script
    assert "[qeflow] 4/11 merge:" in post_script
    assert "[qeflow] 完成: 4/11-11/11" in post_script
    assert "DFTFlow" not in post_script
    assert "submit_process_phonon" not in post_script


def test_qe_phonon_split_coords_submit_dispatches_each_q_task_directly(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_split_submit
    from qeflow.phonon import split_prepare as split_module
    from qeflow.phonon import qe_phonon_split_assign_coords as module

    submitted = []
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_submit_script(workdir, script_name):
        submitted.append((os.path.basename(workdir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    monkeypatch.setattr(_shared, "submit_script", fake_submit_script)
    monkeypatch.setattr(qe_split_submit, "submit_script", fake_submit_script)
    module.run_phonon_split_assign_coords_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)

    workdir = Path("qe_phonon_split_coords")
    _write_job_done(workdir / "scf.out")
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _prepare_xml_fixture(Path("."))
    split_module.prepare_split_assign_coords_step(
        _shared.create_context("input_qe.toml", "MgB2.vasp", workflow_name="qe_phonon_split_coords")
    )

    module.run_phonon_split_assign_coords_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True)

    assert submitted
    assert all(script_name == "run_split.sh" for _, script_name in submitted)
    assert {workdir_name for workdir_name, _ in submitted} == {"01", "02", "03", "04"}


def test_qe_phonon_split_run_merges_then_postprocesses_after_all_q_finished(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import split_prepare as split_module
    from qeflow.phonon import qe_phonon_split_assign_index as module

    workdir = Path("qe_phonon_split_index")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_kpath(workdir / "KPATH.in")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: None)
    split_module.prepare_split_assign_index_step(
        _shared.create_context(str(workdir / "input.toml"), None, workflow_name="qe_phonon_split_index")
    )
    q_map = _shared.load_q_index_entries(str(workdir))
    for entry in q_map:
        q_dir = workdir / str(entry["directory"])
        _write_split_merge_outputs(q_dir, "Mg1B2", int(entry["index"]))
    commands = []

    def fake_run_shell(command, cwd):
        commands.append((command, os.path.basename(cwd)))
        if command == "bash run_q2r.sh":
            _write_job_done(Path(cwd) / "q2r.out")
            (Path(cwd) / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
        if command == "bash run_matdyn_phband.sh":
            _write_job_done(Path(cwd) / "matdyn_phband.out")
            _write_phband_fixture(Path(cwd) / "Mg1B2.phband.freq")
            _write_labelinfo_fixture(Path(cwd) / "Mg1B2_band.labelinfo.dat")
        if command == "bash run_matdyn_phdos.sh":
            _write_job_done(Path(cwd) / "matdyn_phdos.out")
            _write_phdos_fixture(Path(cwd) / "Mg1B2.phdos.dat")

    monkeypatch.setattr(_shared, "run_shell", fake_run_shell)
    module.run_phonon_split_assign_index_workflow(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    assert commands == [
        ("bash run_q2r.sh", "qe_phonon_split_index"),
        ("bash run_matdyn_phband.sh", "qe_phonon_split_index"),
        ("bash run_matdyn_phdos.sh", "qe_phonon_split_index"),
    ]
    assert (workdir / "split_ph.in.1").exists()
    assert (workdir / "_ph0" / "Mg1B2.q_1" / "dvscf").exists()


def test_qe_phonon_split_run_materializes_split_inputs_after_dyn0(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_phonon_split_assign_index as module

    workdir = Path("qe_phonon_split_index")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_run_shell(command, cwd):
        if command == "bash run_scf.sh":
            _write_job_done(Path(cwd) / "scf.out")

    def fake_dyn0(workspace, ph_command, dyn0_path, output_path):
        _write_dyn0_fixture(Path(dyn0_path))
        Path(output_path).write_text("partial\n", encoding="utf-8")
        _prepare_xml_fixture(Path("."))

    monkeypatch.setattr(_shared, "run_shell", fake_run_shell)
    monkeypatch.setattr(module, "split_run_shell", fake_run_shell)
    monkeypatch.setattr(module, "run_ph_to_dyn0", fake_dyn0)
    module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False)

    assert (workdir / "01" / "scf.in").exists()
    assert (workdir / "01" / "split_ph.in").exists()
    assert (workdir / "01" / "run_split.sh").exists()


def test_qe_phonon_nosplit_uses_short_default_workdir_when_not_configured(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_nosplit as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    module.run_phonon_nosplit_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)

    assert Path("qe_phonon_nosplit/scf.in").exists()
    assert Path("qe_phonon_nosplit/run_ph.sh").exists()


def test_create_context_keeps_case_config_as_only_truth_source(chdir_work):
    from qeflow.phonon import _shared

    context = _shared.create_context("input_qe.toml", "MgB2.vasp", workflow_name="qe_phonon_split_index")

    assert Path(context.workspace, "POSCAR").exists()
    assert not Path(context.workspace, "input.toml").exists()
    assert context.config_file == str(Path("input_qe.toml").resolve())


def test_qe_phonon_explicit_workdir_overrides_all_default_workdirs(chdir_work, monkeypatch):
    from qeflow.phonon import qe_phonon_nosplit as nosplit_module
    from qeflow.phonon import qe_phonon_split_assign_coords as coords_module
    from qeflow.phonon import qe_phonon_split_assign_index as index_module

    input_path = Path("input_qe.toml")
    input_path.write_text(
        input_path.read_text(encoding="utf-8").replace("[qe_phonon]\n", '[qe_phonon]\nwork_dir = "custom_ph"\n', 1),
        encoding="utf-8",
    )
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    nosplit_module.run_phonon_nosplit_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)
    Path("custom_ph").rename("custom_ph_nosplit_done")

    coords_module.run_phonon_split_assign_coords_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)
    assert Path("custom_ph/scf.in").exists()
    shutil.rmtree("custom_ph")

    index_module.run_phonon_split_assign_index_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False)
    assert Path("custom_ph/scf.in").exists()
    assert Path("custom_ph_nosplit_done/scf.in").exists()


def test_qe_merge_collects_outputs_by_q_index_map(chdir_work, capsys):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_merge as module
    from qeflow.phonon import split_prepare as split_module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    split_module.prepare_split_assign_index_step(_shared.create_context(str(workdir / "input.toml"), None))

    for entry in _shared.load_q_index_entries(str(workdir)):
        _write_split_merge_outputs(workdir / str(entry["directory"]), "Mg1B2", int(entry["index"]))

    result = module.run_merge_step(str(workdir / "input.toml"), None)

    assert result == 0
    assert (workdir / "Mg1B2.dyn1").exists()
    assert (workdir / "split_ph.in.1").exists()
    assert (workdir / "split_ph.out.1").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "control_ph.xml").exists()
    assert (workdir / "_ph0" / "Mg1B2.q_1" / "dvscf").exists()
    assert "Mg1B2.dyn1" in capsys.readouterr().out
    status = _shared.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "finished"


def test_qe_merge_coords_accepts_plain_dyn_and_root_ph0_dvscf(chdir_work, capsys):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_merge as module
    from qeflow.phonon import split_prepare as split_module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    split_module.prepare_split_assign_coords_step(_shared.create_context(str(workdir / "input.toml"), None))

    for entry in _shared.load_q_index_entries(str(workdir)):
        _write_split_merge_outputs_coords(workdir / str(entry["directory"]), "Mg1B2", int(entry["index"]))

    result = module.run_merge_step(str(workdir / "input.toml"), None)

    assert result == 0
    assert (workdir / "Mg1B2.dyn1").exists()
    assert (workdir / "_ph0" / "dvscf").exists()
    assert not (workdir / "_ph0" / "Mg1B2.q_1").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "patterns.1.xml").exists()
    assert "Mg1B2.dyn1" in capsys.readouterr().out
    status = _shared.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "finished"


def test_qe_merge_index_accepts_first_q_root_dvscf_special_case(chdir_work):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_merge as module
    from qeflow.phonon import split_prepare as split_module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    split_module.prepare_split_assign_index_step(_shared.create_context(str(workdir / "input.toml"), None))

    for entry in _shared.load_q_index_entries(str(workdir)):
        q_dir = workdir / str(entry["directory"])
        index = int(entry["index"])
        _write_split_merge_outputs(q_dir, "Mg1B2", index)
        if index == 1:
            shutil.rmtree(q_dir / "_ph0" / "Mg1B2.q_1")
            (q_dir / "_ph0" / "dvscf").write_text("dvscf\n", encoding="utf-8")

    result = module.run_merge_step(str(workdir / "input.toml"), None)

    assert result == 0
    assert (workdir / "_ph0" / "dvscf").exists()
    assert not (workdir / "_ph0" / "Mg1B2.q_1").exists()
    assert (workdir / "_ph0" / "Mg1B2.q_2" / "dvscf").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "patterns.1.xml").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "patterns.2.xml").exists()
    status = _shared.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "finished"


def test_qe_merge_reports_all_missing_items_without_early_exit(chdir_work, capsys):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_merge as module
    from qeflow.phonon import split_prepare as split_module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    split_module.prepare_split_assign_index_step(_shared.create_context(str(workdir / "input.toml"), None))

    q_map = _shared.load_q_index_entries(str(workdir))
    first = q_map[0]
    first_dir = workdir / str(first["directory"])
    _write_job_done(first_dir / "scf.out")
    _write_job_done(first_dir / "split_ph.out")

    result = module.run_merge_step(str(workdir / "input.toml"), None)

    captured = capsys.readouterr().out
    assert result == 1
    assert "缺少 split_ph.in" not in captured
    assert "缺少候选源文件" in captured
    assert "未完成，禁止 merge" in captured
    status = _shared.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "unstarted"


def test_qe_phonon_split_submit_merges_locally_then_submits_postprocess(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_phonon_split_assign_index as module
    from qeflow.phonon import split_prepare

    workdir = Path("qe_phonon_split_index")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    _write_kpath(workdir / "KPATH.in")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: None)
    split_prepare.prepare_split_assign_index_step(
        _shared.create_context(str(workdir / "input.toml"), None, workflow_name="qe_phonon_split_index")
    )
    for entry in _shared.load_q_index_entries(str(workdir)):
        _write_split_merge_outputs(workdir / str(entry["directory"]), "Mg1B2", int(entry["index"]))

    submitted = []

    def fake_submit_script(work_dir, script_name):
        submitted.append((os.path.basename(work_dir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    result = module.run_phonon_split_assign_index_workflow(str(workdir / "input.toml"), None, run_calc=False, submit_job=True)

    assert result == 0
    assert submitted == [("qe_phonon_split_index", "submit_merge_and_process_phonon.sh")]
    assert not (workdir / "split_ph.out.1").exists()
    script = (workdir / "submit_merge_and_process_phonon.sh").read_text(encoding="utf-8")
    assert "qe_merge" in script
    assert "qe_process_phonon" in script
    assert "11/11" in script


def test_qe_phonon_split_submit_defers_merge_errors_to_slurm_script(chdir_work, monkeypatch):
    from qeflow.phonon import _shared
    from qeflow.phonon import qe_phonon_split_assign_index as module
    from qeflow.phonon import split_prepare

    workdir = Path("qe_phonon_split_index")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    split_prepare.prepare_split_assign_index_step(
        _shared.create_context(str(workdir / "input.toml"), None, workflow_name="qe_phonon_split_index")
    )
    for entry in _shared.load_q_index_entries(str(workdir)):
        q_dir = workdir / str(entry["directory"])
        _write_job_done(q_dir / "scf.out")
        _write_job_done(q_dir / "split_ph.out")

    submitted = []
    monkeypatch.setattr(module, "submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))
    result = module.run_phonon_split_assign_index_workflow(str(workdir / "input.toml"), None, run_calc=False, submit_job=True)

    assert result == 0
    assert [(os.path.basename(work_dir), script_name) for work_dir, script_name in submitted] == [
        ("qe_phonon_split_index", "submit_merge_and_process_phonon.sh")
    ]


def test_refresh_workflow_status_uses_ph_tasks_as_only_split_source(chdir_work):
    from qeflow.phonon import _shared
    from qeflow.phonon import split_prepare as split_module
    from qeflow.qe_input.fileio import load_json

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_dyn0_fixture(workdir / "Mg1B2.dyn0")
    _write_job_done(workdir / "scf.out")
    split_module.prepare_split_assign_index_step(_shared.create_context(str(workdir / "input.toml"), None))

    entries = _shared.load_q_index_entries(str(workdir))
    first_q_dir = workdir / str(entries[0]["directory"])
    _write_job_done(first_q_dir / "scf.out")
    _write_job_done(first_q_dir / "split_ph.out")

    status = _shared.refresh_workflow_status(str(workdir), "Mg1B2")
    payload = load_json(str(workdir / "workflow_status.json"))

    assert "tasks" not in status
    assert "tasks" not in payload
    assert status["ph"]["tasks"] == {"01": "finished", "02": "unstarted", "03": "unstarted", "04": "unstarted"}
    assert payload["ph"]["tasks"] == status["ph"]["tasks"]


def test_split_task_helpers_and_print_status_read_ph_tasks_only(chdir_work, capsys):
    from qeflow.phonon import _shared

    workflow = {
        "scf": "finished",
        "ph": {
            "state": "unstarted",
            "dyn0": "finished",
            "tasks": {
                "01": "finished",
                "02": "unstarted",
            },
        },
        "merge": "unstarted",
        "q2r": "unstarted",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
    }

    assert _shared.all_split_tasks_finished(workflow) is False
    assert _shared.split_task_needs_run(workflow) is True

    _shared.print_status("Mg1B2", workflow)
    output = capsys.readouterr().out
    assert "[qeflow] 下一步:" in output
    assert "unstarted" not in output


def test_split_resume_point_uses_first_non_finished_step(chdir_work, capsys):
    from qeflow.phonon import _shared

    workflow = {
        "scf": "finished",
        "ph": {
            "state": "finished",
            "dyn0": "finished",
            "tasks": {
                "01": "finished",
                "02": "finished",
                "03": "finished",
                "04": "finished",
            },
        },
        "merge": "finished",
        "q2r": "unstarted",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
        "phonon_plot": "unstarted",
    }

    _shared.print_resume_point(workflow)
    output = capsys.readouterr().out

    assert "下一步: 从 5/11 运行 q2r 开始" in output
    assert "[qeflow] 下一步:" in output


def test_write_split_stage_script_and_submit_use_ph_tasks_states(chdir_work, monkeypatch):
    from qeflow.phonon import _shared

    workdir = Path("qe_phonon")
    workdir.mkdir()
    for q_name in ("01", "02", "03"):
        q_dir = workdir / q_name
        q_dir.mkdir()
        (q_dir / "run_split.sh").write_text("#!/bin/bash\n", encoding="utf-8")
    (workdir / "q_index_map.json").write_text(
        '{"mode":"split_assign_index","qpoints":[{"directory":"01","index":1},{"directory":"02","index":2},{"directory":"03","index":3}]}',
        encoding="utf-8",
    )

    context = _shared.create_context("input_qe.toml", "MgB2.vasp")
    context.workspace = str(workdir)
    context.config["slurm"]["header"] = ""
    workflow = {
        "ph": {
            "tasks": {
                "01": "finished",
                "02": "unstarted",
                "03": "error",
            }
        }
    }

    script = _shared.write_split_stage_script(context, workflow)
    content = Path(script).read_text(encoding="utf-8")
    assert "cd 03" in content
    assert "cd 01" not in content
    assert "cd 02" in content

    submitted = []
    monkeypatch.setattr(_shared, "submit_script", lambda cwd, script_name: submitted.append((os.path.basename(cwd), script_name)))
    assert _shared.submit_split_stage_tasks(context, workflow) == 2
    assert submitted == [("02", "run_split.sh"), ("03", "run_split.sh")]
