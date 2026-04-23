import os
import subprocess
import sys
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


def _write_job_done(path: Path) -> None:
    path.write_text("...\nJOB DONE.\n", encoding="utf-8")


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
        ("qe", "slurm", "pseudo_map", "pp_dir", "qe_phonon"),
    )

    assert merged["pp_dir"] == "/pseudo"
    assert merged["qe_phonon"]["work_dir"] == "qe_phonon_custom"
    assert merged["qe_phonon"]["scf"]["ecutwfc"] == 60.0
    assert merged["qe_phonon"]["ph"]["qpoints"] == [2, 3, 4]


def test_qgrid_generates_gamma_first_and_weights(chdir_work):
    from qeflow.phonon.qgrid import generate_irreducible_qpoints

    xml_path = Path("data-file-schema.xml")
    xml_path.write_text(MINIMAL_QGRID_XML, encoding="utf-8")
    qpoints = generate_irreducible_qpoints(str(xml_path), 2, 2, 1)

    assert len(qpoints) == 4
    assert qpoints[0]["q_cart"] == [0.0, 0.0, 0.0]
    assert abs(sum(item["weight"] for item in qpoints) - 1.0) < 1.0e-12


def test_qe_ph_nosplit_generates_ph_input_and_script(chdir_work):
    from qeflow.phonon.workflow import run_standalone_ph

    run_standalone_ph("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False, dyn0_only=False)

    workdir = Path("qe_phonon")
    assert (workdir / "scf.in").exists()
    assert (workdir / "ph.in").exists()
    assert (workdir / "run_ph.sh").exists()
    assert "fildyn = 'Mg1B2.dyn'" in (workdir / "ph.in").read_text(encoding="utf-8")


def test_qe_ph_nosplit_dyn0_run_uses_background_stop(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    captured = {}

    def fake_run_ph_to_dyn0(workspace, ph_command, dyn0_path, output_path):
        captured["workspace"] = workspace
        captured["ph_command"] = ph_command
        Path(dyn0_path).write_text("dyn0\n", encoding="utf-8")
        Path(output_path).write_text("partial\n", encoding="utf-8")

    monkeypatch.setattr(module, "run_ph_to_dyn0", fake_run_ph_to_dyn0)

    module.run_standalone_ph("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False, dyn0_only=True)

    assert captured["workspace"].endswith("qe_phonon")
    assert Path("qe_phonon/Mg1B2.dyn0").exists()


def test_qe_ph_split_assign_index_materializes_split_dirs(chdir_work):
    from qeflow.phonon.workflow import run_standalone_split

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")

    run_standalone_split(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, mode="split_assign_index")

    assert (workdir / "q_00" / "scf.in").exists()
    assert (workdir / "q_00" / "split_ph.in").exists()
    assert (workdir / "q_00" / "run_split.sh").exists()
    assert "start_q = 1" in (workdir / "q_00" / "split_ph.in").read_text(encoding="utf-8")


def test_qe_ph_split_assign_coords_writes_single_q_coordinate(chdir_work):
    from qeflow.phonon.workflow import run_standalone_split

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")

    run_standalone_split(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, mode="split_assign_coords")

    content = (workdir / "q_00" / "split_ph.in").read_text(encoding="utf-8")
    assert "ldisp = .false." in content
    assert "0.000000000000000E+00" in content


def test_qe_q2r_and_matdyn_generate_inputs(chdir_work, monkeypatch):
    from qeflow.phonon.workflow import run_standalone_matdyn, run_standalone_q2r

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_kpath(workdir / "KPATH.in")
    (workdir / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
    monkeypatch.setattr(
        "qeflow.phonon.builders.run_vaspkit_for_kpath",
        lambda target_dir: None,
    )

    run_standalone_q2r(str(workdir / "input.toml"), None, run_calc=False, submit_job=False)
    run_standalone_matdyn(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, target="phband")
    run_standalone_matdyn(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, target="phdos")

    assert (workdir / "q2r.in").exists()
    assert (workdir / "matdyn_phband.in").exists()
    assert (workdir / "matdyn_phdos.in").exists()


def test_output_checks_and_running_detection(chdir_work, monkeypatch):
    from qeflow.phonon.checks import check_ph, check_q2r, check_relax, check_scf

    workdir = Path("qe_phonon")
    workdir.mkdir()
    _write_job_done(workdir / "scf.out")
    (workdir / "Mg1B2.dyn0").write_text("dyn0\n", encoding="utf-8")
    _write_job_done(workdir / "q2r.out")
    (workdir / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
    (Path("vc-relax.out")).write_text("End final coordinates\nJOB DONE.\n", encoding="utf-8")

    assert check_scf(str(workdir)) == "finished"
    assert check_relax(str(Path("."))) == "finished"
    assert check_ph(str(workdir), "Mg1B2")["dyn0"] == "finished"
    assert check_q2r(str(workdir), "Mg1B2") == "finished"

    def fake_run(*args, **kwargs):
        class Result:
            returncode = 0
            stdout = f"123 debug R node 00:01 {(workdir / 'running_only').resolve()}\n"
        return Result()

    monkeypatch.setattr(subprocess, "run", fake_run)
    (workdir / "running_only").mkdir()
    assert check_scf(str(workdir / "running_only")) == "running"


def test_qe_check_cli_reports_supported_status(chdir_work, monkeypatch, capsys):
    from qeflow.phonon import qe_check as module

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


def test_qe_phonon_split_workflow_advances_stage_by_stage(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    submitted = []

    def fake_submit_script(workdir, script_name):
        submitted.append((os.path.basename(workdir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)

    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True, mode="split_assign_index")
    assert submitted == [("qe_phonon", "run_scf.sh")]

    workdir = Path("qe_phonon")
    _write_job_done(workdir / "scf.out")
    _prepare_xml_fixture(Path("."))
    submitted.clear()

    def fake_dyn0(workspace, ph_command, dyn0_path, output_path):
        Path(dyn0_path).write_text("dyn0\n", encoding="utf-8")
        Path(output_path).write_text("partial\n", encoding="utf-8")

    monkeypatch.setattr(module, "run_ph_to_dyn0", fake_dyn0)
    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False, mode="split_assign_index")
    assert (workdir / "Mg1B2.dyn0").exists()

    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False, mode="split_assign_index")
    assert (workdir / "q_00" / "run_split.sh").exists()


def test_qe_phonon_split_workflow_runs_postprocess_after_all_q_finished(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")
    _prepare_xml_fixture(Path("."))
    _write_kpath(workdir / "KPATH.in")
    monkeypatch.setattr("qeflow.phonon.builders.run_vaspkit_for_kpath", lambda target_dir: None)
    module.materialize_split_directories(str(workdir), module.create_context(str(workdir / "input.toml"), None).config, "Mg1B2", "split_assign_index")
    for q_dir in sorted(path for path in workdir.glob("q_*") if path.is_dir()):
        _write_job_done(q_dir / "scf.out")
        _write_job_done(q_dir / "split_ph.out")
    commands = []

    def fake_run_shell(command, cwd):
        commands.append((command, os.path.basename(cwd)))
        if command == "bash run_q2r.sh":
            _write_job_done(Path(cwd) / "q2r.out")
            (Path(cwd) / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
        if command == "bash run_matdyn_phband.sh":
            _write_job_done(Path(cwd) / "matdyn_phband.out")
            (Path(cwd) / "Mg1B2.phband.freq").write_text("band\n", encoding="utf-8")
        if command == "bash run_matdyn_phdos.sh":
            _write_job_done(Path(cwd) / "matdyn_phdos.out")
            (Path(cwd) / "Mg1B2.phdos.dat").write_text("dos\n", encoding="utf-8")

    monkeypatch.setattr(module, "_run_shell", fake_run_shell)
    module.run_phonon_workflow(str(workdir / "input.toml"), None, run_calc=True, submit_job=False, mode="split_assign_index")

    assert commands == [
        ("bash run_q2r.sh", "qe_phonon"),
        ("bash run_matdyn_phband.sh", "qe_phonon"),
        ("bash run_matdyn_phdos.sh", "qe_phonon"),
    ]
