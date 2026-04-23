import os
import re
import shutil
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
    from qeflow.qe_input.fileio import load_json

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")

    run_standalone_split(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, mode="split_assign_index")

    assert (workdir / "q_01" / "scf.in").exists()
    assert (workdir / "q_01" / "split_ph.in").exists()
    assert (workdir / "q_01" / "run_split.sh").exists()
    assert "start_q = 1" in (workdir / "q_01" / "split_ph.in").read_text(encoding="utf-8")
    q_map = load_json(str(workdir / "q_index_map.json"))
    assert q_map["mode"] == "split_assign_index"
    assert q_map["qpoints"][0]["directory"] == "q_01"


def test_qe_ph_split_assign_coords_writes_single_q_coordinate(chdir_work):
    from qeflow.phonon.workflow import run_standalone_split
    from qeflow.qe_input.fileio import load_json

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")

    run_standalone_split(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, mode="split_assign_coords")

    content = (workdir / "q_01" / "split_ph.in").read_text(encoding="utf-8")
    assert "ldisp = .false." in content
    assert "0.000000000000000E+00" in content
    q_map = load_json(str(workdir / "q_index_map.json"))
    assert q_map["mode"] == "split_assign_coords"
    assert q_map["qpoints"][0]["directory"] == "q_01"


def test_qe_q2r_and_matdyn_generate_inputs(chdir_work, monkeypatch):
    from qeflow.phonon.workflow import run_standalone_matdyn, run_standalone_q2r

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

    run_standalone_q2r(str(workdir / "input.toml"), None, run_calc=False, submit_job=False)
    run_standalone_matdyn(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, target="phband")
    run_standalone_matdyn(str(workdir / "input.toml"), None, run_calc=False, submit_job=False, target="phdos")

    assert (workdir / "q2r.in").exists()
    assert (workdir / "matdyn_phband.in").exists()
    assert (workdir / "matdyn_phdos.in").exists()


def test_output_checks_and_running_detection(chdir_work, monkeypatch):
    from qeflow.qe_check.checks import check_ph, check_q2r, check_relax, check_scf

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
    from qeflow.phonon import workflow as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False, mode="nosplit")

    workdir = Path("qe_phonon")
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
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
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
            (cwd_path / "Mg1B2.phband.freq").write_text("band\n", encoding="utf-8")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            (cwd_path / "Mg1B2.phdos.dat").write_text("dos\n", encoding="utf-8")

    monkeypatch.setattr(module, "_run_shell", fake_run_shell)
    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False, mode="nosplit")

    assert executed == [
        "bash run_scf.sh",
        "bash run_ph.sh",
        "bash run_q2r.sh",
        "bash run_matdyn_phband.sh",
        "bash run_matdyn_phdos.sh",
    ]
    assert (workdir / "Mg1B2.phdos.dat").exists()


def test_qe_phonon_nosplit_run_continues_from_ph_and_finishes(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
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
            (cwd_path / "Mg1B2.phband.freq").write_text("band\n", encoding="utf-8")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            (cwd_path / "Mg1B2.phdos.dat").write_text("dos\n", encoding="utf-8")

    monkeypatch.setattr(module, "_run_shell", fake_run_shell)
    module.run_phonon_workflow(str(workdir / "input.toml"), None, run_calc=True, submit_job=False, mode="nosplit")

    assert executed == [
        "bash run_ph.sh",
        "bash run_q2r.sh",
        "bash run_matdyn_phband.sh",
        "bash run_matdyn_phdos.sh",
    ]


def test_qe_phonon_nosplit_run_continues_from_q2r_and_finishes(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")
    _write_job_done(workdir / "ph.out")
    (workdir / "Mg1B2.dyn0").write_text("dyn0\n", encoding="utf-8")
    _write_job_done(workdir / "q2r.out")
    (workdir / "Mg1B2.fc").write_text("fc\n", encoding="utf-8")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    executed = []

    def fake_run_shell(command, cwd):
        executed.append(command)
        cwd_path = Path(cwd)
        if command == "bash run_matdyn_phband.sh":
            _write_job_done(cwd_path / "matdyn_phband.out")
            (cwd_path / "Mg1B2.phband.freq").write_text("band\n", encoding="utf-8")
        elif command == "bash run_matdyn_phdos.sh":
            _write_job_done(cwd_path / "matdyn_phdos.out")
            (cwd_path / "Mg1B2.phdos.dat").write_text("dos\n", encoding="utf-8")

    monkeypatch.setattr(module, "_run_shell", fake_run_shell)
    module.run_phonon_workflow(str(workdir / "input.toml"), None, run_calc=True, submit_job=False, mode="nosplit")

    assert executed == [
        "bash run_matdyn_phband.sh",
        "bash run_matdyn_phdos.sh",
    ]


def test_qe_phonon_nosplit_submit_writes_remaining_chain_script(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    submitted = []
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_submit_script(workdir, script_name):
        submitted.append((os.path.basename(workdir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True, mode="nosplit")

    assert submitted == [("qe_phonon", "submit_phonon_workflow.sh")]
    script = Path("qe_phonon/submit_phonon_workflow.sh").read_text(encoding="utf-8")
    assert "bash run_scf.sh" in script
    assert "bash run_ph.sh" in script
    assert "bash run_q2r.sh" in script
    assert "bash run_matdyn_phband.sh" in script
    assert "bash run_matdyn_phdos.sh" in script


def test_qe_phonon_split_prepares_all_root_inputs(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))
    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=False, mode="split_assign_index")

    workdir = Path("qe_phonon")
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


def test_qe_phonon_split_submit_uses_four_stage_progression(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    submitted = []
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_submit_script(workdir, script_name):
        submitted.append((os.path.basename(workdir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True, mode="split_assign_index")

    assert submitted == [("qe_phonon", "submit_pre_stage.sh")]
    pre_script = Path("qe_phonon/submit_pre_stage.sh").read_text(encoding="utf-8")
    assert "bash run_scf.sh" in pre_script
    assert "bash run_ph.sh" in pre_script
    assert "bash run_q2r.sh" not in pre_script

    workdir = Path("qe_phonon")
    _write_job_done(workdir / "scf.out")
    (workdir / "Mg1B2.dyn0").write_text("dyn0\n", encoding="utf-8")
    _prepare_xml_fixture(Path("."))
    module.materialize_split_directories(str(workdir), module.create_context("input_qe.toml", "MgB2.vasp").config, "Mg1B2", "split_assign_index")
    submitted.clear()

    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True, mode="split_assign_index")
    assert submitted == [("qe_phonon", "submit_split_stage.sh")]
    split_script = Path("qe_phonon/submit_split_stage.sh").read_text(encoding="utf-8")
    assert "q_01" in split_script
    assert "run_split.sh" in split_script
    assert "run_q2r.sh" not in split_script

    for q_dir in sorted(path for path in workdir.glob("q_*") if path.is_dir()):
        content = (q_dir / "split_ph.in").read_text(encoding="utf-8")
        match = re.search(r"start_q\s*=\s*(\d+)", content)
        assert match is not None
        index = int(match.group(1))
        _write_split_merge_outputs(q_dir, "Mg1B2", index)
    submitted.clear()

    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True, mode="split_assign_index")
    assert submitted == [("qe_phonon", "submit_merge_stage.sh")]
    status = module.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "unstarted"
    merge_script = Path("qe_phonon/submit_merge_stage.sh").read_text(encoding="utf-8")
    assert "bash run_merge.sh" in merge_script
    assert "run_q2r.sh" not in merge_script

    assert module.run_merge_stage(module.create_context("input_qe.toml", "MgB2.vasp")) is True
    submitted.clear()

    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=False, submit_job=True, mode="split_assign_index")
    assert submitted == [("qe_phonon", "submit_post_stage.sh")]
    post_script = Path("qe_phonon/submit_post_stage.sh").read_text(encoding="utf-8")
    assert "bash run_q2r.sh" in post_script
    assert "bash run_matdyn_phband.sh" in post_script
    assert "bash run_matdyn_phdos.sh" in post_script
    assert "run_merge.sh" not in post_script


def test_qe_phonon_split_run_merges_then_postprocesses_after_all_q_finished(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _write_job_done(workdir / "scf.out")
    _prepare_xml_fixture(Path("."))
    _write_kpath(workdir / "KPATH.in")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: None)
    module.materialize_split_directories(str(workdir), module.create_context(str(workdir / "input.toml"), None).config, "Mg1B2", "split_assign_index")
    q_map = module._load_q_index_entries(str(workdir))
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
    assert (workdir / "split_ph.in.1").exists()
    assert (workdir / "_ph0" / "Mg1B2.q_1" / "dvscf").exists()


def test_qe_phonon_split_run_materializes_split_inputs_after_dyn0(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: _write_kpath(Path(target_dir) / "KPATH.in"))

    def fake_run_shell(command, cwd):
        if command == "bash run_scf.sh":
            _write_job_done(Path(cwd) / "scf.out")

    def fake_dyn0(workspace, ph_command, dyn0_path, output_path):
        Path(dyn0_path).write_text("dyn0\n", encoding="utf-8")
        Path(output_path).write_text("partial\n", encoding="utf-8")
        _prepare_xml_fixture(Path("."))

    monkeypatch.setattr(module, "_run_shell", fake_run_shell)
    monkeypatch.setattr(module, "run_ph_to_dyn0", fake_dyn0)
    module.run_phonon_workflow("input_qe.toml", "MgB2.vasp", run_calc=True, submit_job=False, mode="split_assign_index")

    assert (workdir / "q_01" / "scf.in").exists()
    assert (workdir / "q_01" / "split_ph.in").exists()
    assert (workdir / "q_01" / "run_split.sh").exists()


def test_qe_merge_collects_outputs_by_q_index_map(chdir_work, capsys):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")
    module.materialize_split_directories(str(workdir), module.create_context(str(workdir / "input.toml"), None).config, "Mg1B2", "split_assign_index")

    for entry in module._load_q_index_entries(str(workdir)):
        _write_split_merge_outputs(workdir / str(entry["directory"]), "Mg1B2", int(entry["index"]))

    result = module.run_standalone_merge(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    assert result == 0
    assert (workdir / "Mg1B2.dyn1").exists()
    assert (workdir / "split_ph.in.1").exists()
    assert (workdir / "split_ph.out.1").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "control_ph.xml").exists()
    assert (workdir / "_ph0" / "Mg1B2.q_1" / "dvscf").exists()
    assert "Mg1B2.dyn1" in capsys.readouterr().out
    status = module.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "finished"


def test_qe_merge_coords_accepts_plain_dyn_and_root_ph0_dvscf(chdir_work, capsys):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")
    module.materialize_split_directories(
        str(workdir),
        module.create_context(str(workdir / "input.toml"), None).config,
        "Mg1B2",
        "split_assign_coords",
    )

    for entry in module._load_q_index_entries(str(workdir)):
        _write_split_merge_outputs_coords(workdir / str(entry["directory"]), "Mg1B2", int(entry["index"]))

    result = module.run_standalone_merge(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    assert result == 0
    assert (workdir / "Mg1B2.dyn1").exists()
    assert (workdir / "_ph0" / "dvscf").exists()
    assert not (workdir / "_ph0" / "Mg1B2.q_1").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "patterns.1.xml").exists()
    assert "Mg1B2.dyn1" in capsys.readouterr().out
    status = module.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "finished"


def test_qe_merge_index_accepts_first_q_root_dvscf_special_case(chdir_work):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")
    module.materialize_split_directories(
        str(workdir),
        module.create_context(str(workdir / "input.toml"), None).config,
        "Mg1B2",
        "split_assign_index",
    )

    for entry in module._load_q_index_entries(str(workdir)):
        q_dir = workdir / str(entry["directory"])
        index = int(entry["index"])
        _write_split_merge_outputs(q_dir, "Mg1B2", index)
        if index == 1:
            shutil.rmtree(q_dir / "_ph0" / "Mg1B2.q_1")
            (q_dir / "_ph0" / "dvscf").write_text("dvscf\n", encoding="utf-8")

    result = module.run_standalone_merge(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    assert result == 0
    assert (workdir / "_ph0" / "dvscf").exists()
    assert not (workdir / "_ph0" / "Mg1B2.q_1").exists()
    assert (workdir / "_ph0" / "Mg1B2.q_2" / "dvscf").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "patterns.1.xml").exists()
    assert (workdir / "_ph0" / "Mg1B2.phsave" / "patterns.2.xml").exists()
    status = module.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "finished"


def test_qe_merge_reports_all_missing_items_without_early_exit(chdir_work, capsys):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")
    module.materialize_split_directories(str(workdir), module.create_context(str(workdir / "input.toml"), None).config, "Mg1B2", "split_assign_index")

    q_map = module._load_q_index_entries(str(workdir))
    first = q_map[0]
    first_dir = workdir / str(first["directory"])
    _write_job_done(first_dir / "scf.out")
    _write_job_done(first_dir / "split_ph.out")

    result = module.run_standalone_merge(str(workdir / "input.toml"), None, run_calc=True, submit_job=False)

    captured = capsys.readouterr().out
    assert result == 1
    assert "缺少 split_ph.in" not in captured
    assert "缺少候选源文件" in captured
    assert "未完成，禁止 merge" in captured
    status = module.refresh_workflow_status(str(workdir), "Mg1B2")
    assert status["merge"] == "unstarted"


def test_qe_ph_split_submit_merges_locally_then_submits_postprocess(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")
    _write_kpath(workdir / "KPATH.in")
    monkeypatch.setattr("qeflow.qe_input.builders.run_vaspkit_for_kpath", lambda target_dir: None)
    module.materialize_split_directories(str(workdir), module.create_context(str(workdir / "input.toml"), None).config, "Mg1B2", "split_assign_index")
    for entry in module._load_q_index_entries(str(workdir)):
        _write_split_merge_outputs(workdir / str(entry["directory"]), "Mg1B2", int(entry["index"]))

    submitted = []

    def fake_submit_script(work_dir, script_name):
        submitted.append((os.path.basename(work_dir), script_name))

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    result = module.run_standalone_split(str(workdir / "input.toml"), None, run_calc=False, submit_job=True, mode="split_assign_index")

    assert result == 0
    assert submitted == [("qe_phonon", "submit_post_stage.sh")]
    assert (workdir / "split_ph.out.1").exists()


def test_qe_ph_split_submit_stops_when_local_merge_fails(chdir_work, monkeypatch):
    from qeflow.phonon import workflow as module

    workdir = Path("qe_phonon")
    workdir.mkdir()
    Path("MgB2.vasp").replace(workdir / "POSCAR")
    Path("input_qe.toml").replace(workdir / "input.toml")
    _prepare_xml_fixture(Path("."))
    _write_job_done(workdir / "scf.out")
    module.materialize_split_directories(str(workdir), module.create_context(str(workdir / "input.toml"), None).config, "Mg1B2", "split_assign_index")
    for entry in module._load_q_index_entries(str(workdir)):
        q_dir = workdir / str(entry["directory"])
        _write_job_done(q_dir / "scf.out")
        _write_job_done(q_dir / "split_ph.out")

    submitted = []
    monkeypatch.setattr(module, "submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    result = module.run_standalone_split(str(workdir / "input.toml"), None, run_calc=False, submit_job=True, mode="split_assign_index")

    assert result == 1
    assert submitted == []
