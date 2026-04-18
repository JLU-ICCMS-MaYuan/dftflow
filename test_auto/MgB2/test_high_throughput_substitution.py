import io
import json
import sys
from contextlib import redirect_stdout
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))


PLACEHOLDER_POSCAR = """XAXBH7-P63mmc
1.0
        4.1799998283         0.0000000000         0.0000000000
       -2.0899999142         3.6199860392         0.0000000000
        0.0000000000         0.0000000000         6.5120000839
   XA   XB    H
    2    2   14
Direct
     0.333333343         0.666666687         0.250000000
     0.666666627         0.333333313         0.750000000
     0.000000000         0.000000000         0.500000000
     0.000000000         0.000000000         0.000000000
     0.000000000         0.000000000         0.250000000
     0.000000000         0.000000000         0.750000000
     0.806500018         0.612999976         0.090999998
     0.193499982         0.387000024         0.908999979
     0.387000024         0.193500042         0.090999998
     0.612999976         0.806499958         0.908999979
     0.806499958         0.193499982         0.090999998
     0.193500042         0.806500018         0.908999979
     0.193499982         0.387000024         0.591000021
     0.806500018         0.612999976         0.409000009
     0.612999976         0.806499958         0.591000021
     0.387000024         0.193500042         0.409000009
     0.193500042         0.806500018         0.591000021
     0.806499958         0.193499982         0.409000009
"""


def _write_ht_config(path: Path):
    path.write_text(
        """
potcar_dir = "potcar_lib"

[vasp]
executable_path = "vasp_std"

[slurm]
header = "#!/bin/bash"

[vasp_opt]
ENCUT = 520
ISMEAR = 0
SIGMA = 0.05
ISYM = 0
SYMPREC = 1e-5
PSTRESS = 0.0
NCORE = 4
NELM = 200
POTIM = 0.05
EDIFF = 1e-7
EDIFFG = -0.01
IBRION = 2
ISIF = 3
kmesh = 0.025

[vasp_scf]
ENCUT = 520
ISMEAR = 0
SIGMA = 0.05
ISYM = 2
NCORE = 4
kmesh = 0.025

[vasp_eledos]
ENCUT = 520
ISMEAR = -5
ISYM = 2
NCORE = 4
NEDOS = 2000
kmesh = 0.025

[high_throughput_substitution]
XA = "3,4"
XB = "3-5"
pressures = [20, 50]
structure_filename = "structure.vasp"
preview = true
""".strip()
        + "\n",
        encoding="utf-8",
    )


def _write_opt_outcar(path: Path, converged: bool = True):
    marker = "reached required accuracy - stopping structural energy minimisation" if converged else "still optimizing"
    path.write_text(marker + "\n", encoding="utf-8")


def _write_scf_like_outcar(path: Path, converged: bool = True, include_kb: bool = False, nelm: int = 60, iterations: int = 5):
    lines = [f" NELM   =    {nelm};"]
    lines.extend([" Iteration    1( 1)" for _ in range(iterations)])
    if converged:
        lines.extend(
            [
                " FREE ENERGIE OF THE ION-ELECTRON SYSTEM",
                " aborting loop because EDIFF is reached",
                " General timing and accounting informations for this job",
            ]
        )
    if include_kb:
        lines.append(" in kB  *")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def test_selection_preview_and_range_parse():
    from high_throughput_substitution.selection import parse_atomic_selection, render_selection_preview

    assert parse_atomic_selection("1-3,5,7-8") == [1, 2, 3, 5, 7, 8]
    preview = render_selection_preview("XA", "3,4,5,57,90")
    assert "XA 选择预览:" in preview
    assert "(00)period_1:" in preview
    assert "(03)period_2:" in preview
    assert "(00)period_3:" in preview
    assert "Li" in preview
    assert "Be" in preview
    assert "B" in preview
    assert "(01)lanthanides:" in preview
    assert "(01)actinides:" in preview


def test_generate_cases_builds_period_manifest_and_case_status(chdir_work):
    from high_throughput_substitution.status import create_initial_status
    from high_throughput_substitution.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)

    result = generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    assert len(result["cases"]) == 6
    assert Path("ht_out/03.XA_Li/period_2/workflow_manifest.json").exists()
    assert not Path("ht_out/03.XA_Li/period_2/04.LiBeH7/workflow_manifest.json").exists()
    case_status = json.loads(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/case_status.json").read_text(encoding="utf-8"))
    assert case_status["pressures"]["20"]["vasp_opt"] == "unstarted"


def test_refresh_status_detects_finished_error_and_running(chdir_work):
    from high_throughput_substitution.status import create_initial_status, refresh_status
    from high_throughput_substitution.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")

    opt_dir = case_dir / "20/vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)

    scf_dir = case_dir / "20/vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=True, iterations=4, nelm=60)

    eledos_dir = case_dir / "20/vasp_eledos"
    eledos_dir.mkdir(parents=True, exist_ok=True)
    (eledos_dir / "DOSCAR").write_text("dos\n", encoding="utf-8")
    _write_scf_like_outcar(eledos_dir / "OUTCAR", converged=False)
    (eledos_dir / "TDOS.dat").write_text("tdos\n", encoding="utf-8")

    running_step_dir = case_dir / "50/vasp_opt"
    running_step_dir.mkdir(parents=True, exist_ok=True)
    (running_step_dir / "INCAR").write_text("incar\n", encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output=f"123 batch R job 00:01 {running_step_dir.resolve()}\n")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert case_status["pressures"]["20"]["vasp_opt"] == "finished"
    assert case_status["pressures"]["20"]["vasp_scf"] == "finished"
    assert case_status["pressures"]["20"]["vasp_eledos"] == "error"
    assert case_status["pressures"]["20"]["vasp_process_eledos"] == "error"
    assert case_status["pressures"]["50"]["vasp_opt"] == "running"
    assert workflow_status["steps"]["vasp_opt"]["finished"] >= 1
    assert workflow_status["steps"]["vasp_opt"]["running"] >= 1


def test_run_workflow_generates_submit_script_and_opt_dirs(chdir_work, monkeypatch):
    from high_throughput_substitution.workflow import run_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))
        if command[0] == "vasp_opt":
            pressure = command[-1]
            pressure_dir = Path(cwd) / pressure / "vasp_opt"
            pressure_dir.mkdir(parents=True, exist_ok=True)
            (pressure_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
            _write_opt_outcar(pressure_dir / "OUTCAR", converged=True)

    monkeypatch.setattr("high_throughput_substitution.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("high_throughput_substitution.status.get_active_workdirs", lambda user="mayuan": set())

    script_path = run_workflow(str(structure_path), str(config_path), "ht_out", run_calc=False, submit=False)

    assert Path(script_path).exists()
    assert any(command[0][0] == "vasp_opt" for command in calls)
    assert Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20/vasp_opt/CONTCAR").exists()


def test_continue_updates_case_status_and_runs_missing_steps(chdir_work, monkeypatch):
    from high_throughput_substitution.status import create_initial_status, refresh_status
    from high_throughput_substitution.structure_substitution import generate_cases
    from high_throughput_substitution.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    opt_dir = case_dir / "20/vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))
        cwd_path = Path(cwd)
        if command[0] == "vasp_scf":
            (cwd_path / "vasp_scf").mkdir(parents=True, exist_ok=True)
            (cwd_path / "vasp_scf/CHGCAR").write_text("chg\n", encoding="utf-8")
            (cwd_path / "vasp_scf/WAVECAR").write_text("wav\n", encoding="utf-8")
            _write_scf_like_outcar(cwd_path / "vasp_scf/OUTCAR", converged=True, iterations=4, nelm=60)
        elif command[0] == "vasp_eledos":
            (cwd_path / "vasp_eledos").mkdir(parents=True, exist_ok=True)
            (cwd_path / "vasp_eledos/DOSCAR").write_text("dos\n", encoding="utf-8")
            _write_scf_like_outcar(cwd_path / "vasp_eledos/OUTCAR", converged=True, iterations=4, nelm=60)
        elif command[0] == "vasp_process_eledos":
            (cwd_path / "vasp_eledos/pdos_merged_at_ef.csv").write_text("csv\n", encoding="utf-8")

    monkeypatch.setattr("high_throughput_substitution.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("high_throughput_substitution.status.get_active_workdirs", lambda user="mayuan": set())

    summary = continue_workflow("ht_out", run_calc=True, submit=False)
    refresh_status("ht_out", squeue_output="")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert "总 case 数" in summary
    assert any(command[0] == "vasp_scf" for command, _ in calls)
    assert case_status["pressures"]["20"]["vasp_process_eledos"] == "finished"


def test_high_throughput_substitute_structure_reads_structure_from_config(chdir_work, monkeypatch):
    from high_throughput_substitution import cli_structure as module

    structure_path = Path("ABH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)

    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"', 'structure_filename = "ABH7-P63mmc.vasp"'
    )
    config_path.write_text(config_text, encoding="utf-8")

    monkeypatch.setattr(sys, "argv", ["high_throughput_substitute_structure", "-c", "input_ht.toml", "-o", "ht_out"])
    monkeypatch.setattr("high_throughput_substitution.status.get_active_workdirs", lambda user="mayuan": set())
    buffer = io.StringIO()
    with redirect_stdout(buffer):
        module.main()

    output = buffer.getvalue()
    assert "XA 选择预览" in output
    assert Path("ht_out/03.XA_Li/period_2/04.LiBeH7/ABH7-P63mmc.vasp").exists()
