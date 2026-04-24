import io
import json
import os
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

[mattersim]
executable_path = "python"

mattersim_source_path = "/tmp/mock_mattersim/src"

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

[substitutionflow]
XA = "3,4"
XB = "3-5"
pressures = [20, 50]
structure_filename = "structure.vasp"
preview = true
submit_groups = 1
""".strip()
        + "\n",
        encoding="utf-8",
    )


def _write_ht_config_without_mattersim(path: Path):
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

[substitutionflow]
XA = "3,4"
XB = "3-5"
pressures = [20, 50]
structure_filename = "structure.vasp"
preview = true
submit_groups = 1
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


def _write_mock_potcar(path: Path, symbol: str, zval: float, config_entries):
    lines = [
        f"  PAW_PBE {symbol} 23Dec2003",
        f"   {zval:.13f}",
        " parameters from PSCTR are:",
        f"   VRHFIN ={symbol} : [core= Xe]",
        "   POMASS =  140.115; ZVAL   =   %.3f    mass and valenz" % zval,
        "",
        "   Atomic configuration",
        f"   {len(config_entries)} entries",
        "     n  l   j            E        occ.",
    ]
    for n_value, l_value, occ_value in config_entries:
        lines.append(f"     {n_value}  {l_value}  0.50        -1.0000   {occ_value:.4f}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _make_potential_dir(root: Path, name: str, symbol: str, zval: float, config_entries):
    target = root / name
    target.mkdir(parents=True, exist_ok=True)
    _write_mock_potcar(target / "POTCAR", symbol, zval, config_entries)
    (target / "PSCTR").write_text("psctr\n", encoding="utf-8")


def _set_submit_groups(config_path: Path, groups: int):
    config_text = config_path.read_text(encoding="utf-8")
    config_text = config_text.replace("submit_groups = 1", f"submit_groups = {groups}")
    config_path.write_text(config_text, encoding="utf-8")


def _set_phonon_mode(config_path: Path, mode: str):
    config_text = config_path.read_text(encoding="utf-8")
    marker = '[substitutionflow]\n'
    config_text = config_text.replace(marker, marker + f'phonon_mode = "{mode}"\n', 1)
    config_path.write_text(config_text, encoding="utf-8")


def _write_qe_phonon_workflow_status(work_dir: Path, *, matdyn_phdos: str = "finished"):
    work_dir.mkdir(parents=True, exist_ok=True)
    payload = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished"},
        "merge": "finished",
        "q2r": "finished",
        "matdyn_phband": "finished",
        "matdyn_phdos": matdyn_phdos,
        "tasks": {},
    }
    (work_dir / "workflow_status.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def test_selection_preview_and_range_parse():
    from substitutionflow.selection import parse_atomic_selection, render_selection_preview

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
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )

    result = generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    assert len(result["cases"]) == 6
    assert Path("ht_out/03.XA_Li/period_2/workflow_manifest.json").exists()
    assert not Path("ht_out/03.XA_Li/period_2/04.LiBeH7/workflow_manifest.json").exists()
    case_status = json.loads(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/case_status.json").read_text(encoding="utf-8"))
    assert case_status["pressures"]["20"]["vasp_opt"] == "unstarted"
    assert case_status["pressures"]["20"]["qe_phonon"] == "unstarted"


def test_refresh_status_detects_finished_error_and_running(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")

    mattersim_dir = case_dir / "20/mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )

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

    _write_qe_phonon_workflow_status(case_dir / "20/qe_phonon_split_index", matdyn_phdos="finished")

    running_step_dir = case_dir / "50/mattersim_opt"
    running_step_dir.mkdir(parents=True, exist_ok=True)
    (running_step_dir / "POSCAR").write_text("poscar\n", encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output=f"123 batch R job 00:01 {running_step_dir.resolve()}\n")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert case_status["pressures"]["20"]["mattersim_opt"] == "finished"
    assert case_status["pressures"]["20"]["vasp_opt"] == "finished"
    assert case_status["pressures"]["20"]["vasp_scf"] == "finished"
    assert case_status["pressures"]["20"]["vasp_eledos"] == "error"
    assert case_status["pressures"]["20"]["vasp_process_eledos"] == "error"
    assert case_status["pressures"]["20"]["qe_phonon"] == "finished"
    assert case_status["pressures"]["50"]["mattersim_opt"] == "running"
    assert workflow_status["steps"]["mattersim_opt"]["finished"] >= 1
    assert workflow_status["steps"]["mattersim_opt"]["running"] >= 1
    assert workflow_status["steps"]["qe_phonon"]["finished"] >= 1


def test_refresh_status_marks_prepared_mattersim_as_unstarted(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/03.Li2H7")
    mattersim_dir = case_dir / "50/mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "POSCAR").write_text("poscar\n", encoding="utf-8")
    (mattersim_dir / "run_mattersim.sh").write_text("#!/bin/bash\n", encoding="utf-8")
    (mattersim_dir / "run_mattersim_opt.py").write_text("print('run')\n", encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output="")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert case_status["pressures"]["50"]["mattersim_opt"] == "unstarted"
    assert workflow_status["steps"]["mattersim_opt"]["unstarted"] >= 1


def test_refresh_status_without_mattersim_marks_step_finished(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    workflow_status = refresh_status("ht_out", squeue_output="")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert "mattersim_opt" not in case_status["pressures"]["20"]
    assert case_status["pressures"]["20"]["vasp_opt"] == "unstarted"
    assert case_status["pressures"]["20"]["qe_phonon"] == "unstarted"
    assert "mattersim_opt" not in workflow_status["steps"]
    assert "qe_phonon" in workflow_status["steps"]


def test_run_workflow_generates_group_scripts_and_opt_dirs(chdir_work, monkeypatch):
    from substitutionflow.workflow import run_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )
    _set_submit_groups(config_path, 2)

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))
        if command[0] == "mattersim_opt":
            pressure = command[-1]
            pressure_dir = Path(cwd) / pressure / "mattersim_opt"
            pressure_dir.mkdir(parents=True, exist_ok=True)
            (pressure_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
            (pressure_dir / "mattersim_opt.json").write_text(
                json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
                encoding="utf-8",
            )

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    summary = run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)

    assert "总生成单元数" in summary
    assert Path("ht_out/submit_group_01.sh").exists()
    assert Path("ht_out/submit_group_02.sh").exists()
    assert any(command[0][0] == "mattersim_opt" for command in calls)
    assert Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20/mattersim_opt/CONTCAR").exists()
    group_text = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8") + Path("ht_out/submit_group_02.sh").read_text(encoding="utf-8")
    assert "vasp_opt -i 20/mattersim_opt/CONTCAR -c input.toml -p 20 -r" in group_text
    assert "qe_phonon_split_index -i vasp_opt/CONTCAR -c ../input.toml -r" in group_text


def test_run_workflow_without_mattersim_starts_from_vasp_opt(chdir_work, monkeypatch):
    from substitutionflow.workflow import run_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    summary = run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)

    assert "总生成单元数" in summary
    assert calls
    assert all(command[0] == "vasp_opt" for command, _ in calls)
    assert all(command[2] == "XAXBH7-P63mmc.vasp" for command, _ in calls)
    assert all(Path(cwd).name != "20" for _, cwd in calls)
    assert not any(command[0] == "mattersim_opt" for command, _ in calls)
    script_text = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8")
    assert "mattersim_opt" not in script_text
    assert "vasp_opt -i XAXBH7-P63mmc.vasp -c input.toml -p 20 -r" in script_text
    assert "qe_phonon_split_index -i vasp_opt/CONTCAR -c ../input.toml -r" in script_text


def test_continue_updates_case_status_and_runs_missing_steps(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    mattersim_dir = case_dir / "20/mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))
        cwd_path = Path(cwd)
        if command[0] == "vasp_opt":
            pressure = command[-2]
            opt_dir = cwd_path / pressure / "vasp_opt"
            opt_dir.mkdir(parents=True, exist_ok=True)
            (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
            _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
        elif command[0] == "vasp_scf":
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
        elif command[0] == "qe_phonon_split_index":
            _write_qe_phonon_workflow_status(cwd_path / "qe_phonon_split_index", matdyn_phdos="finished")

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    summary = continue_workflow("ht_out", run_calc=True, submit=False)
    refresh_status("ht_out", squeue_output="")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert "总 case 数" in summary
    assert any(command[0] == "vasp_opt" for command, _ in calls)
    assert any(command[0] == "vasp_scf" for command, _ in calls)
    target_vasp_opt_calls = [
        (command, cwd)
        for command, cwd in calls
        if command[0] == "vasp_opt" and Path(cwd).resolve() == case_dir.resolve()
    ]
    target_scf_calls = [
        (command, cwd)
        for command, cwd in calls
        if command[0] == "vasp_scf" and Path(cwd).resolve() == (case_dir / "20").resolve()
    ]
    assert target_vasp_opt_calls
    assert target_scf_calls
    assert case_status["pressures"]["20"]["vasp_process_eledos"] == "finished"
    assert case_status["pressures"]["20"]["qe_phonon"] == "finished"
    assert any(command[0] == "qe_phonon_split_index" for command, _ in calls)


def test_continue_without_mattersim_runs_from_vasp_opt(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))
        cwd_path = Path(cwd)
        if command[0] == "vasp_opt":
            (cwd_path / "20/vasp_opt").mkdir(parents=True, exist_ok=True)
            (cwd_path / "20/vasp_opt/CONTCAR").write_text("contcar\n", encoding="utf-8")
            _write_opt_outcar(cwd_path / "20/vasp_opt/OUTCAR", converged=True)
        elif command[0] == "vasp_scf":
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
        elif command[0] == "qe_phonon_split_index":
            _write_qe_phonon_workflow_status(cwd_path / "qe_phonon_split_index", matdyn_phdos="finished")

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    summary = continue_workflow("ht_out", run_calc=True, submit=False)

    assert "总 case 数" in summary
    assert "mattersim_opt" not in summary
    assert any(command[0] == "vasp_opt" for command, _ in calls)
    assert any(command[0] == "qe_phonon_split_index" for command, _ in calls)
    assert all(Path(cwd).name != "20" for command, cwd in calls if command[0] == "vasp_opt")
    assert not any(command[0] == "mattersim_opt" for command, _ in calls)


def test_refresh_status_without_mattersim_removes_legacy_mattersim_key(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_status_path = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/case_status.json")
    payload = json.loads(case_status_path.read_text(encoding="utf-8"))
    payload["pressures"]["20"]["mattersim_opt"] = "finished"
    payload["pressures"]["50"]["mattersim_opt"] = "error"
    case_status_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output="")
    refreshed = json.loads(case_status_path.read_text(encoding="utf-8"))

    assert "mattersim_opt" not in refreshed["pressures"]["20"]
    assert "mattersim_opt" not in refreshed["pressures"]["50"]
    assert "mattersim_opt" not in workflow_status["steps"]


def test_case_status_upgrade_appends_qe_phonon_without_overwriting_existing_steps(chdir_work):
    from substitutionflow.status import _ensure_case_status_file, discover_cases
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")

    cases = discover_cases("ht_out")
    case_meta = cases["03.XA_Li/period_2/04.LiBeH7"]
    status_path = Path(case_meta["case_dir"]) / "case_status.json"
    legacy_payload = {
        "formula": case_meta["formula"],
        "xa_symbol": case_meta["xa_symbol"],
        "xb_symbol": case_meta["xb_symbol"],
        "input_structure": case_meta["input_structure"],
        "pressures": {
            "20": {
                "mattersim_opt": "finished",
                "vasp_opt": "finished",
                "vasp_scf": "running",
                "vasp_eledos": "unstarted",
                "vasp_process_eledos": "unstarted",
            },
            "50": {
                "mattersim_opt": "error",
                "vasp_opt": "unstarted",
                "vasp_scf": "unstarted",
                "vasp_eledos": "unstarted",
                "vasp_process_eledos": "unstarted",
            },
        },
    }
    status_path.write_text(json.dumps(legacy_payload, ensure_ascii=False, indent=2), encoding="utf-8")

    upgraded = _ensure_case_status_file(case_meta)

    assert upgraded["pressures"]["20"]["mattersim_opt"] == "finished"
    assert upgraded["pressures"]["20"]["vasp_scf"] == "running"
    assert upgraded["pressures"]["20"]["qe_phonon"] == "unstarted"
    assert upgraded["pressures"]["50"]["mattersim_opt"] == "error"
    assert upgraded["pressures"]["50"]["qe_phonon"] == "unstarted"


def test_refresh_status_rewrites_legacy_workflow_status_with_qe_phonon(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    legacy_workflow_status = {
        "root_output_dir": str(Path("ht_out").resolve()),
        "total_cases": 1,
        "steps": {
            "mattersim_opt": {"finished": 0, "running": 0, "error": 0, "unstarted": 12},
        },
    }
    Path("ht_out/workflow_status.json").write_text(
        json.dumps(legacy_workflow_status, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    workflow_status = refresh_status("ht_out", squeue_output="")

    assert "qe_phonon" in workflow_status["steps"]
    assert Path("ht_out/workflow_status.json").exists()


def test_build_submit_scripts_only_keeps_remaining_qe_phonon_when_previous_steps_finished(chdir_work, monkeypatch):
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.status import create_initial_status
    from substitutionflow.workflow import build_submit_scripts

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    mattersim_dir = case_dir / "mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )
    opt_dir = case_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    scf_dir = case_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    eledos_dir = case_dir / "vasp_eledos"
    eledos_dir.mkdir(parents=True, exist_ok=True)
    (eledos_dir / "DOSCAR").write_text("dos\n", encoding="utf-8")
    _write_scf_like_outcar(eledos_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    (eledos_dir / "pdos_merged_at_ef.csv").write_text("csv\n", encoding="utf-8")

    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())
    bundle = build_submit_scripts("ht_out", 1)
    target_unit = next(
        unit
        for script_info in bundle["scripts"]
        for unit in script_info["units"]
        if unit["case_rel"].endswith("04.LiBeH7") and unit["pressure_key"] == "20"
    )

    assert target_unit["steps"] == ["qe_phonon"]
    assert target_unit["commands"] == [
        f'cd "{case_dir.resolve()}"',
        "qe_phonon_split_index -i vasp_opt/CONTCAR -c ../input.toml -r",
    ]


def test_phonon_mode_switches_qe_phonon_command(chdir_work, monkeypatch):
    from substitutionflow.workflow import run_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )
    _set_phonon_mode(config_path, "nosplit")

    def fake_run_cli_command(command, cwd):
        if command[0] == "mattersim_opt":
            pressure = command[-1]
            pressure_dir = Path(cwd) / pressure / "mattersim_opt"
            pressure_dir.mkdir(parents=True, exist_ok=True)
            (pressure_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
            (pressure_dir / "mattersim_opt.json").write_text(
                json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
                encoding="utf-8",
            )

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)

    script_text = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8")
    assert "qe_phonon_nosplit -i vasp_opt/CONTCAR -c ../input.toml -r" in script_text


def test_invalid_phonon_mode_raises_clear_error(chdir_work):
    from substitutionflow.workflow import run_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )
    _set_phonon_mode(config_path, "bad_mode")

    try:
        run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)
    except ValueError as exc:
        assert "phonon_mode" in str(exc)
        assert "nosplit" in str(exc)
    else:
        raise AssertionError("非法 phonon_mode 应抛出 ValueError")


def test_continue_runs_only_qe_phonon_when_previous_steps_finished(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    mattersim_dir = pressure_dir / "mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )
    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    scf_dir = pressure_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    eledos_dir = pressure_dir / "vasp_eledos"
    eledos_dir.mkdir(parents=True, exist_ok=True)
    (eledos_dir / "DOSCAR").write_text("dos\n", encoding="utf-8")
    _write_scf_like_outcar(eledos_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    (eledos_dir / "pdos_merged_at_ef.csv").write_text("csv\n", encoding="utf-8")

    calls = []

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))
        if command[0] == "qe_phonon_split_index":
            _write_qe_phonon_workflow_status(Path(cwd) / "qe_phonon_split_index", matdyn_phdos="finished")

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    continue_workflow("ht_out", run_calc=True, submit=False)

    target_calls = [
        (command, cwd)
        for command, cwd in calls
        if Path(cwd).resolve() == pressure_dir.resolve() and command[0] == "qe_phonon_split_index"
    ]
    assert target_calls == [(["qe_phonon_split_index", "-i", "vasp_opt/CONTCAR", "-c", "../input.toml", "-r"], str(pressure_dir.resolve()))]


def test_continue_does_not_advance_to_qe_phonon_when_previous_step_error(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    mattersim_dir = pressure_dir / "mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )
    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    scf_dir = pressure_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    eledos_dir = pressure_dir / "vasp_eledos"
    eledos_dir.mkdir(parents=True, exist_ok=True)
    (eledos_dir / "DOSCAR").write_text("dos\n", encoding="utf-8")
    _write_scf_like_outcar(eledos_dir / "OUTCAR", converged=False)
    (eledos_dir / "TDOS.dat").write_text("tdos\n", encoding="utf-8")

    calls = []
    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", lambda command, cwd: calls.append((command, cwd)))
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())

    continue_workflow("ht_out", run_calc=True, submit=False)

    target_calls = [(command, cwd) for command, cwd in calls if Path(cwd).resolve() == pressure_dir.resolve()]
    assert target_calls
    assert all(command[0] == "vasp_eledos" for command, _ in target_calls)
    assert not any(command[0].startswith("qe_phonon") for command, _ in target_calls)


def test_continue_skips_running_qe_phonon(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    mattersim_dir = pressure_dir / "mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )
    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    scf_dir = pressure_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    eledos_dir = pressure_dir / "vasp_eledos"
    eledos_dir.mkdir(parents=True, exist_ok=True)
    (eledos_dir / "DOSCAR").write_text("dos\n", encoding="utf-8")
    _write_scf_like_outcar(eledos_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    (eledos_dir / "pdos_merged_at_ef.csv").write_text("csv\n", encoding="utf-8")
    running_qe_dir = pressure_dir / "qe_phonon_split_index"
    running_qe_dir.mkdir(parents=True, exist_ok=True)
    (running_qe_dir / "run_ph.sh").write_text("#!/bin/bash\n", encoding="utf-8")

    calls = []
    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", lambda command, cwd: calls.append((command, cwd)))
    monkeypatch.setattr(
        "substitutionflow.status.get_active_workdirs",
        lambda user="mayuan": {str(running_qe_dir.resolve())},
    )

    continue_workflow("ht_out", run_calc=True, submit=False)

    target_calls = [(command, cwd) for command, cwd in calls if Path(cwd).resolve() == pressure_dir.resolve()]
    assert not any(command[0].startswith("qe_phonon") for command, _ in target_calls)


def test_build_submit_scripts_split_units_without_breaking_chain(chdir_work, monkeypatch):
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.status import create_initial_status
    from substitutionflow.workflow import build_submit_scripts

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    _set_submit_groups(config_path, 3)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    mattersim_dir = case_dir / "20/mattersim_opt"
    mattersim_dir.mkdir(parents=True, exist_ok=True)
    (mattersim_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    (mattersim_dir / "mattersim_opt.json").write_text(
        json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
        encoding="utf-8",
    )
    opt_dir = case_dir / "20/vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)

    running_step_dir = Path("ht_out/03.XA_Li/period_2/03.Li2H7/50/mattersim_opt")
    running_step_dir.mkdir(parents=True, exist_ok=True)
    (running_step_dir / "POSCAR").write_text("poscar\n", encoding="utf-8")
    monkeypatch.setattr(
        "substitutionflow.status.get_active_workdirs",
        lambda user="mayuan": {str(running_step_dir.resolve())},
    )

    bundle = build_submit_scripts("ht_out", 3)

    assert bundle["configured_groups"] == 3
    assert bundle["actual_groups"] == 3
    assert bundle["total_units"] == 11

    group_texts = [
        Path(script_info["script_path"]).read_text(encoding="utf-8")
        for script_info in bundle["scripts"]
    ]
    joined = "\n".join(group_texts)
    assert "03.Li2H7/50" not in joined

    unit_hits = [text.count('cd "/work') + text.count('cd "') for text in group_texts]
    assert sum(script_info["unit_count"] for script_info in bundle["scripts"]) == 11
    target_units = [
        unit
        for script_info in bundle["scripts"]
        for unit in script_info["units"]
        if unit["case_rel"].endswith("04.LiBeH7") and unit["pressure_key"] == "20"
    ]
    assert len(target_units) == 1
    target_unit = target_units[0]
    target_text = "\n".join(target_unit["commands"])
    assert 'cd "/tmp' in target_text or 'cd "' in target_text
    assert f'cd "{Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20").resolve()}"' in target_text
    assert "vasp_scf -i vasp_opt/CONTCAR -c ../input.toml -r" in target_text
    assert "vasp_eledos -i vasp_scf/POSCAR -c ../input.toml -r" in target_text
    assert "vasp_process_eledos -d ./vasp_eledos --elements-total Li Be H" in target_text
    assert "qe_phonon_split_index -i vasp_opt/CONTCAR -c ../input.toml -r" in target_text
    assert "mattersim_opt -i structure.vasp -c input.toml -p 20 -r" not in target_text
    assert "vasp_opt -i mattersim_opt/CONTCAR -c ../input.toml -p 20 -r" not in target_text
    assert all(script_info["unit_count"] > 0 for script_info in bundle["scripts"])
    assert unit_hits


def test_submit_group_scripts_submits_each_group(chdir_work, monkeypatch):
    from substitutionflow.workflow import run_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )
    _set_submit_groups(config_path, 4)

    def fake_run_cli_command(command, cwd):
        if command[0] == "mattersim_opt":
            pressure = command[-1]
            pressure_dir = Path(cwd) / pressure / "mattersim_opt"
            pressure_dir.mkdir(parents=True, exist_ok=True)
            (pressure_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
            (pressure_dir / "mattersim_opt.json").write_text(
                json.dumps({"stages": {"stage1": {"converged": True}, "stage2": {"converged": True}}}),
                encoding="utf-8",
            )

    submitted = []

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())
    monkeypatch.setattr("substitutionflow.workflow.submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    summary = run_workflow(str(config_path), "ht_out", run_calc=False, submit=True)

    assert "计划分组数: 4" in summary
    assert len(submitted) == 4
    assert [script_name for _, script_name in submitted] == [
        "submit_group_01.sh",
        "submit_group_02.sh",
        "submit_group_03.sh",
        "submit_group_04.sh",
    ]


def test_continue_submit_uses_case_input_submit_groups(chdir_work, monkeypatch):
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.status import create_initial_status
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    _set_submit_groups(config_path, 5)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    submitted = []
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())
    monkeypatch.setattr("substitutionflow.workflow.submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    summary = continue_workflow("ht_out", run_calc=False, submit=True)

    assert "计划分组数: 5" in summary
    assert len(submitted) == 5
    assert Path("ht_out/submit_group_05.sh").exists()


def test_substitution_structure_reads_structure_from_config(chdir_work, monkeypatch):
    from substitutionflow import cli_structure as module

    structure_path = Path("ABH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)

    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"', 'structure_filename = "ABH7-P63mmc.vasp"'
    )
    config_path.write_text(config_text, encoding="utf-8")

    monkeypatch.setattr(sys, "argv", ["substitution_structure", "-c", "input_ht.toml", "-o", "ht_out"])
    monkeypatch.setattr("substitutionflow.status.get_active_workdirs", lambda user="mayuan": set())
    buffer = io.StringIO()
    with redirect_stdout(buffer):
        module.main()

    output = buffer.getvalue()
    assert "XA 选择预览" in output
    assert Path("ht_out/03.XA_Li/period_2/04.LiBeH7/ABH7-P63mmc.vasp").exists()


def test_substitution_cli_reads_structure_from_config(chdir_work, monkeypatch):
    from substitutionflow import cli_workflow as module

    structure_path = Path("ABH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"', 'structure_filename = "ABH7-P63mmc.vasp"'
    )
    config_path.write_text(config_text, encoding="utf-8")

    def fake_run_workflow(config_file, output_dir, run_calc=False, submit=False):
        assert config_file == "input_ht.toml"
        assert output_dir == "ht_out"
        assert run_calc is False
        assert submit is False
        return "ok"

    monkeypatch.setattr("substitutionflow.cli_workflow.run_workflow", fake_run_workflow)
    monkeypatch.setattr(sys, "argv", ["substitution", "-c", "input_ht.toml", "-o", "ht_out"])

    buffer = io.StringIO()
    with redirect_stdout(buffer):
        module.main()

    assert buffer.getvalue().strip() == "ok"


def test_prepare_potcar_discovers_elements_and_parses_config(chdir_work):
    from substitutionflow.prepare_potcar import (
        discover_candidate_groups,
        discover_required_elements,
    )

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'structure_filename = "structure.vasp"',
            'structure_filename = "XAXBH7-P63mmc.vasp"',
        ),
        encoding="utf-8",
    )

    pot_root = Path("potpaw_PBE54")
    _make_potential_dir(pot_root, "H", "H", 1, [(1, 0, 1)])
    _make_potential_dir(pot_root, "Li", "Li", 1, [(2, 0, 1)])
    _make_potential_dir(pot_root, "Be", "Be", 2, [(2, 0, 2)])
    _make_potential_dir(pot_root, "B", "B", 3, [(2, 1, 1), (2, 0, 2)])
    _make_potential_dir(pot_root, "Ce", "Ce", 12, [(5, 0, 2), (6, 0, 2), (5, 1, 6), (5, 2, 1), (4, 3, 1)])
    _make_potential_dir(pot_root, "Ce_GW", "Ce", 12, [(5, 0, 2), (6, 0, 2), (5, 1, 6), (5, 2, 1), (4, 3, 1)])

    elements = discover_required_elements(str(config_path))
    assert elements == ["H", "Li", "Be", "B"]

    groups = discover_candidate_groups(str(pot_root), ["Ce"])
    assert [candidate.candidate_name for candidate in groups["Ce"]] == ["Ce", "Ce_GW"]
    assert groups["Ce"][0].config_text == "4f1 5s2 5p6 5d1 6s2"


def test_prepare_potcar_interactive_selection_and_log_reuse(chdir_work, monkeypatch):
    from substitutionflow.prepare_potcar import prepare_potcar_library

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_text = config_path.read_text(encoding="utf-8")
    config_text = config_text.replace('structure_filename = "structure.vasp"', 'structure_filename = "XAXBH7-P63mmc.vasp"')
    config_text = config_text.replace('XA = "3,4"', 'XA = "58"')
    config_text = config_text.replace('XB = "3-5"', 'XB = "23"')
    config_path.write_text(config_text, encoding="utf-8")

    pot_root = Path("potpaw_PBE54")
    _make_potential_dir(pot_root, "H", "H", 1, [(1, 0, 1)])
    _make_potential_dir(pot_root, "Ce", "Ce", 12, [(5, 0, 2), (6, 0, 2), (5, 1, 6), (5, 2, 1), (4, 3, 1)])
    _make_potential_dir(pot_root, "Ce_GW", "Ce", 12, [(5, 0, 2), (6, 0, 2), (5, 1, 6), (5, 2, 1), (4, 3, 1)])
    _make_potential_dir(pot_root, "V", "V", 5, [(3, 2, 3), (4, 0, 2)])
    _make_potential_dir(pot_root, "V_pv", "V", 11, [(3, 1, 6), (3, 2, 3), (4, 0, 2)])

    answers = iter(["1", "2"])
    monkeypatch.setattr("builtins.input", lambda _prompt="": next(answers))

    log_path = prepare_potcar_library(str(config_path), str(pot_root))

    assert Path(log_path).exists()
    assert Path("potcar_lib/Ce").read_text(encoding="utf-8") == Path("potpaw_PBE54/Ce_GW/POTCAR").read_text(encoding="utf-8")
    assert Path("potcar_lib/V").read_text(encoding="utf-8") == Path("potpaw_PBE54/V/POTCAR").read_text(encoding="utf-8")
    log_text = Path(log_path).read_text(encoding="utf-8")
    assert "Ce = Ce_GW" in log_text
    assert "V = V" in log_text

    Path("potcar_lib/potcar_lib.log").write_text("Ce = Ce\nV = V_pv\n", encoding="utf-8")
    monkeypatch.setattr("builtins.input", lambda _prompt="": (_ for _ in ()).throw(AssertionError("不应进入交互")))
    prepare_potcar_library(str(config_path), str(pot_root))
    assert Path("potcar_lib/Ce").read_text(encoding="utf-8") == Path("potpaw_PBE54/Ce/POTCAR").read_text(encoding="utf-8")
    assert Path("potcar_lib/V").read_text(encoding="utf-8") == Path("potpaw_PBE54/V_pv/POTCAR").read_text(encoding="utf-8")


def test_prepare_potcar_writes_log_incrementally(chdir_work, monkeypatch):
    from substitutionflow.prepare_potcar import prepare_potcar_library

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_text = config_path.read_text(encoding="utf-8")
    config_text = config_text.replace('structure_filename = "structure.vasp"', 'structure_filename = "XAXBH7-P63mmc.vasp"')
    config_text = config_text.replace('XA = "3,4"', 'XA = "58"')
    config_text = config_text.replace('XB = "3-5"', 'XB = "23"')
    config_path.write_text(config_text, encoding="utf-8")

    pot_root = Path("potpaw_PBE54")
    _make_potential_dir(pot_root, "H", "H", 1, [(1, 0, 1)])
    _make_potential_dir(pot_root, "Ce", "Ce", 12, [(5, 0, 2), (6, 0, 2), (5, 1, 6), (5, 2, 1), (4, 3, 1)])
    _make_potential_dir(pot_root, "Ce_GW", "Ce", 12, [(5, 0, 2), (6, 0, 2), (5, 1, 6), (5, 2, 1), (4, 3, 1)])
    _make_potential_dir(pot_root, "V", "V", 5, [(3, 2, 3), (4, 0, 2)])
    _make_potential_dir(pot_root, "V_pv", "V", 11, [(3, 1, 6), (3, 2, 3), (4, 0, 2)])

    state = {"count": 0}

    def fake_input(_prompt=""):
        state["count"] += 1
        if state["count"] == 1:
            return "1"
        if state["count"] == 2:
            log_text = Path("potcar_lib/potcar_lib.log").read_text(encoding="utf-8")
            assert "V = V" in log_text
            assert "Ce =" not in log_text
            return "2"
        raise AssertionError("不应出现第三次交互")

    monkeypatch.setattr("builtins.input", fake_input)

    prepare_potcar_library(str(config_path), str(pot_root))

    log_text = Path("potcar_lib/potcar_lib.log").read_text(encoding="utf-8")
    assert "V = V" in log_text
    assert "Ce = Ce_GW" in log_text


def _write_at_ef_csv(path: Path, headers, values):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        ",".join(headers) + "\n" + ",".join(str(value) for value in values) + "\n",
        encoding="utf-8",
    )


def _prepare_plot_cases(root_dir: Path):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = root_dir / "XAXBH7-P63mmc.vasp"
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = root_dir / "input_ht.toml"
    _write_ht_config_without_mattersim(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"', 'structure_filename = "XAXBH7-P63mmc.vasp"'
    )
    config_text = config_text.replace('pressures = [20, 50]', 'pressures = [50, 100]')
    config_path.write_text(config_text, encoding="utf-8")
    generate_cases(str(structure_path), str(config_path), str(root_dir / "ht_out"))
    create_initial_status(str(root_dir / "ht_out"))
    return root_dir / "ht_out"


def test_substitution_plot_eledos_accepts_root_single_and_multiple_xa_dirs(chdir_work):
    from substitutionflow.plot_eledos import resolve_target_xa_dirs

    out_dir = _prepare_plot_cases(Path("."))
    out_dir = out_dir.resolve()
    expected = sorted(str(path.resolve()) for path in out_dir.glob("*.XA_*"))

    os.chdir(out_dir)
    assert sorted(resolve_target_xa_dirs([])) == expected
    assert sorted(resolve_target_xa_dirs([str(out_dir)])) == expected
    assert resolve_target_xa_dirs([str(out_dir / "03.XA_Li")]) == [str((out_dir / "03.XA_Li").resolve())]
    assert sorted(resolve_target_xa_dirs([str(out_dir / "03.XA_Li"), str(out_dir / "04.XA_Be")])) == [
        str((out_dir / "03.XA_Li").resolve()),
        str((out_dir / "04.XA_Be").resolve()),
    ]


def test_substitution_plot_eledos_rejects_mixed_root_and_xa_inputs(chdir_work):
    import pytest
    from substitutionflow.plot_eledos import resolve_target_xa_dirs

    out_dir = _prepare_plot_cases(Path("."))
    with pytest.raises(ValueError):
        resolve_target_xa_dirs([str(out_dir), str(out_dir / "03.XA_Li")])


def test_substitution_plot_eledos_generates_pressure_png_and_handles_missing_cells(chdir_work):
    from substitutionflow.plot_eledos import collect_pressure_plot_data, generate_xa_dir_plots

    out_dir = _prepare_plot_cases(Path("."))
    xa_dir = out_dir / "03.XA_Li"
    finished_case = xa_dir / "period_2" / "04.LiBeH7"
    missing_case = xa_dir / "period_2" / "05.LiBH7"

    _write_at_ef_csv(
        finished_case / "50" / "vasp_eledos" / "pdos_merged_at_ef.csv",
        ["Energy(eV)", "Li", "Be", "H", "TDOS"],
        [0.0, 1.2, 2.3, 3.4, 4.5],
    )
    _write_at_ef_csv(
        finished_case / "100" / "vasp_eledos" / "pdos_merged_at_ef.csv",
        ["Energy(eV)", "Li", "Be", "H", "TDOS"],
        [0.0, 1.5, 2.0, 2.5, 3.0],
    )
    _write_at_ef_csv(
        missing_case / "50" / "vasp_eledos" / "pdos_merged_at_ef.csv",
        ["Energy(eV)", "Li", "B", "TDOS"],
        [0.0, 0.8, 1.1, 1.4],
    )

    plot_data = collect_pressure_plot_data(str(xa_dir), "50")
    assert plot_data["rows"] == ["period_2"]
    assert plot_data["cells"][("period_2", 2)]["values"]["XB"] == 2.3
    assert plot_data["cells"][("period_2", 13)]["values"] is None
    assert plot_data["ymax"] > 4.5

    outputs = generate_xa_dir_plots(str(xa_dir))
    assert str((xa_dir / "eledos_summary_50.png").resolve()) in [str(Path(item).resolve()) for item in outputs]
    assert str((xa_dir / "eledos_summary_100.png").resolve()) in [str(Path(item).resolve()) for item in outputs]
    assert (xa_dir / "eledos_summary_50.png").exists()
    assert (xa_dir / "eledos_summary_100.png").exists()


def test_substitution_plot_eledos_main_defaults_to_current_directory(chdir_work, monkeypatch):
    from substitutionflow import plot_eledos as module

    out_dir = _prepare_plot_cases(Path("."))
    xa_dir = out_dir / "03.XA_Li"
    _write_at_ef_csv(
        xa_dir / "period_2" / "04.LiBeH7" / "50" / "vasp_eledos" / "pdos_merged_at_ef.csv",
        ["Energy(eV)", "Li", "Be", "H", "TDOS"],
        [0.0, 1.0, 2.0, 3.0, 4.0],
    )

    monkeypatch.chdir(out_dir)
    monkeypatch.setattr(sys, "argv", ["substitution_plot_eledos"])
    buffer = io.StringIO()
    with redirect_stdout(buffer):
        module.main()

    output = buffer.getvalue()
    assert "03.XA_Li/eledos_summary_50.png" in output or "03.XA_Li/eledos_summary_100.png" in output
