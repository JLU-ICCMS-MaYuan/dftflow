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


def _append_qe_phonon_config(path: Path):
    pp_dir_text = '\npp_dir = "qe_pseudo_lib"\n'
    qe_text = """
[qe]
executable_path = "/opt/qe/bin"

[qe_phonon]
prefix = "ht_ph"

[qe_phonon.scf]
ecutwfc = 80.0
ecutrho = 640.0
outdir = "./qe_out"
kpoints = [3, 3, 2]

[qe_phonon.ph]
qpoints = [2, 2, 1]
recover = true

[qe_phonon.q2r]
zasr = "crystal"

[qe_phonon.phband]
kpath_file = "KPATH.custom"
kpath_points = 60

[qe_phonon.phdos]
qpoints = [6, 6, 4]
ndos = 400

"""
    config_text = path.read_text(encoding="utf-8")
    config_text = config_text.replace('potcar_dir = "potcar_lib"\n', 'potcar_dir = "potcar_lib"\n' + pp_dir_text, 1)
    config_text = config_text.replace("[substitutionflow]\n", qe_text + "[substitutionflow]\n", 1)
    path.write_text(config_text, encoding="utf-8")


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


def _write_mock_upf(path: Path, element: str, z_valence: float, config_text: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        (
            '<?xml version="1.0"?>\n'
            f'<UPF element="{element}" z_valence="{z_valence}">\n'
            f'  <PP_HEADER element="{element}" z_valence="{z_valence}" valence_configuration="{config_text}"/>\n'
            "</UPF>\n"
        ),
        encoding="utf-8",
    )


def _write_realistic_mock_upf(path: Path, element: str, z_valence: float, valence_lines: str, input_lines: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        (
            '<UPF version="2.0.1">\n'
            "  <PP_INFO>\n"
            "    Valence configuration:\n"
            "    nl pn  l   occ       Rcut    Rcut US       E pseu\n"
            f"{valence_lines}\n"
            "    Generation configuration:\n"
            f"{input_lines}\n"
            "  </PP_INFO>\n"
            f'  <PP_HEADER element="{element}" z_valence="{z_valence:.6E}"/>\n'
            "</UPF>\n"
        ),
        encoding="utf-8",
    )


def _set_submit_groups(config_path: Path, groups: int):
    config_text = config_path.read_text(encoding="utf-8")
    config_text = config_text.replace("submit_groups = 1", f"submit_groups = {groups}")
    config_path.write_text(config_text, encoding="utf-8")


def _set_phonon_mode(config_path: Path, mode: str):
    config_text = config_path.read_text(encoding="utf-8")
    marker = '[substitutionflow]\n'
    config_text = config_text.replace(marker, marker + f'phonon_mode = "{mode}"\n', 1)
    config_path.write_text(config_text, encoding="utf-8")


def _write_qe_phonon_workflow_status(
    work_dir: Path,
    *,
    workflow_name: str = "qe_phonon_split_index",
    scf: str = "finished",
    ph_state: str = "finished",
    dyn0: str = "finished",
    tasks=None,
    merge: str = "finished",
    q2r: str = "finished",
    matdyn_phband: str = "finished",
    matdyn_phdos: str = "finished",
):
    work_dir.mkdir(parents=True, exist_ok=True)
    if tasks is None:
        tasks = {"q_0001": "finished"} if workflow_name != "qe_phonon_nosplit" else {}
    payload = {
        "scf": scf,
        "ph": {"state": ph_state, "dyn0": dyn0, "tasks": tasks},
        "merge": merge,
        "q2r": q2r,
        "matdyn_phband": matdyn_phband,
        "matdyn_phdos": matdyn_phdos,
    }
    if workflow_name == "qe_phonon_nosplit":
        payload["ph"] = {"state": ph_state, "dyn0": dyn0, "tasks": {}}
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


def test_generate_cases_writes_full_case_input_config_for_qe_phonon(chdir_work):
    from public.config_templates import load_config_file
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    _append_qe_phonon_config(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"',
        'structure_filename = "XAXBH7-P63mmc.vasp"',
    )
    config_path.write_text(config_text, encoding="utf-8")

    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    case_config = load_config_file(str(case_dir / "input.toml"))

    assert (case_dir / "XAXBH7-P63mmc.vasp").exists()
    assert (case_dir / "input.toml").exists()
    assert (case_dir.parent / "workflow_manifest.json").exists()
    assert (case_dir / "case_status.json").exists()

    assert case_config["pp_dir"] == "qe_pseudo_lib"
    assert case_config["qe"]["executable_path"] == "/opt/qe/bin"
    assert case_config["qe_phonon"]["prefix"] == "ht_ph"
    assert case_config["qe_phonon"]["scf"]["ecutwfc"] == 80.0
    assert case_config["qe_phonon"]["scf"]["kpoints"] == [3, 3, 2]
    assert case_config["qe_phonon"]["ph"]["qpoints"] == [2, 2, 1]
    assert case_config["qe_phonon"]["ph"]["recover"] is True
    assert case_config["qe_phonon"]["q2r"]["zasr"] == "crystal"
    assert case_config["qe_phonon"]["phband"]["kpath_file"] == "KPATH.custom"
    assert case_config["qe_phonon"]["phdos"]["ndos"] == 400
    assert case_config["chgcar_path"] == "./vasp_scf/CHGCAR"
    assert case_config["wavecar_path"] == "./vasp_scf/WAVECAR"


def test_case_input_supports_qe_phonon_prepare_from_pressure_directory(chdir_work):
    from qeflow.phonon._shared import create_context, prepare_root_scf_step
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    _append_qe_phonon_config(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"',
        'structure_filename = "XAXBH7-P63mmc.vasp"',
    )
    config_path.write_text(config_text, encoding="utf-8")

    generate_cases(str(structure_path), str(config_path), "ht_out")
    pseudo_dir = Path("ht_out/qe_pseudo_lib")
    pseudo_dir.mkdir(parents=True, exist_ok=True)
    _write_mock_upf(pseudo_dir / "Li.upf", "Li", 1.0, "2s1")
    _write_mock_upf(pseudo_dir / "Be.upf", "Be", 2.0, "2s2")
    _write_mock_upf(pseudo_dir / "H.upf", "H", 1.0, "1s1")
    expected_pseudo_dir = str(pseudo_dir.resolve())

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    vasp_opt_dir = pressure_dir / "vasp_opt"
    vasp_opt_dir.mkdir(parents=True, exist_ok=True)
    (vasp_opt_dir / "CONTCAR").write_text(
        Path("ht_out/03.XA_Li/period_2/04.LiBeH7/XAXBH7-P63mmc.vasp").read_text(encoding="utf-8"),
        encoding="utf-8",
    )

    os.chdir(pressure_dir)
    context = create_context("../input.toml", "vasp_opt/CONTCAR", workflow_name="qe_phonon_split_index")
    prepare_root_scf_step(context)

    work_dir = Path(context.workspace)
    scf_text = (work_dir / "scf.in").read_text(encoding="utf-8")

    assert work_dir.name == "qe_phonon_split_index"
    assert (work_dir / "POSCAR").exists()
    assert f"pseudo_dir = '{expected_pseudo_dir}'" in scf_text
    assert "prefix = 'ht_ph'" in scf_text
    assert "outdir = './qe_out'" in scf_text
    assert "Li.upf" in scf_text
    assert "Be.upf" in scf_text
    assert "K_POINTS automatic" in scf_text
    assert "  3 3 2 0 0 0" in scf_text


def test_refresh_status_detects_finished_error_and_unstarted(chdir_work):
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

    prepared_step_dir = case_dir / "50/mattersim_opt"
    prepared_step_dir.mkdir(parents=True, exist_ok=True)
    (prepared_step_dir / "POSCAR").write_text("poscar\n", encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output=f"123 batch R job 00:01 {prepared_step_dir.resolve()}\n")
    case_status = json.loads((case_dir / "case_status.json").read_text(encoding="utf-8"))

    assert case_status["pressures"]["20"]["mattersim_opt"] == "finished"
    assert case_status["pressures"]["20"]["vasp_opt"] == "finished"
    assert case_status["pressures"]["20"]["vasp_scf"] == "finished"
    assert case_status["pressures"]["20"]["vasp_eledos"] == "error"
    assert case_status["pressures"]["20"]["vasp_process_eledos"] == "error"
    assert case_status["pressures"]["20"]["qe_phonon"] == "finished"
    assert case_status["pressures"]["50"]["mattersim_opt"] == "unstarted"
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]["mattersim_opt"] == "finished"
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]["vasp_eledos"] == "error"
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["50"]["mattersim_opt"] == "unstarted"


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
    assert workflow_status["cases"]["03.XA_Li/period_2/03.Li2H7"]["pressures"]["50"]["mattersim_opt"] == "unstarted"


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
    assert "mattersim_opt" not in workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]["qe_phonon"] == "unstarted"


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

    summary = run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)

    assert "总生成单元数" in summary
    assert Path("ht_out/submit_group_01.sh").exists()
    assert Path("ht_out/submit_group_02.sh").exists()
    assert any(command[0][0] == "mattersim_opt" for command in calls)
    assert Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20/mattersim_opt/CONTCAR").exists()
    group_text = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8") + Path("ht_out/submit_group_02.sh").read_text(encoding="utf-8")
    expected_case_config = str(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").resolve())
    assert "vasp_opt -i 20/mattersim_opt/CONTCAR -c input.toml -p 20 -r" in group_text
    assert f"qe_phonon_split_index -i vasp_opt/CONTCAR -c {expected_case_config} -s" in group_text


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

    summary = run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)

    assert "总生成单元数" in summary
    assert calls
    assert all(command[0] == "vasp_opt" for command, _ in calls)
    assert all(command[2] == "XAXBH7-P63mmc.vasp" for command, _ in calls)
    assert all(Path(cwd).name != "20" for _, cwd in calls)
    assert not any(command[0] == "mattersim_opt" for command, _ in calls)
    script_text = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8")
    expected_case_config = str(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").resolve())
    assert "mattersim_opt" not in script_text
    assert "vasp_opt -i XAXBH7-P63mmc.vasp -c input.toml -p 20 -r" in script_text
    assert f"qe_phonon_split_index -i vasp_opt/CONTCAR -c {expected_case_config} -s" in script_text


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

    summary = continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))
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

    summary = continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))

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
    assert "mattersim_opt" not in workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]


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
                "vasp_scf": "error",
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
    assert upgraded["pressures"]["20"]["vasp_scf"] == "error"
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
            "mattersim_opt": {"finished": 0, "error": 0, "unstarted": 12},
        },
    }
    Path("ht_out/workflow_status.json").write_text(
        json.dumps(legacy_workflow_status, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    workflow_status = refresh_status("ht_out", squeue_output="")

    target_case = workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]
    assert target_case["pressures"]["20"]["qe_phonon"] == "unstarted"
    assert Path("ht_out/workflow_status.json").exists()


def test_create_initial_status_writes_case_detail_workflow_status(chdir_work):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")

    workflow_status = create_initial_status("ht_out")

    target_case = workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]
    assert workflow_status["root_output_dir"] == str(Path("ht_out").resolve())
    assert "total_cases" not in workflow_status
    assert "steps" not in workflow_status
    assert target_case["formula"] == "LiBeH7"
    assert target_case["xa_symbol"] == "Li"
    assert target_case["xb_symbol"] == "Be"
    assert target_case["input_structure"].endswith("structure.vasp")
    assert target_case["pressures"]["20"]["vasp_opt"] == "unstarted"
    assert target_case["pressures"]["20"]["qe_phonon"] == "unstarted"


def test_refresh_status_rewrites_legacy_input_structure_to_local_case_structure(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"',
        'structure_filename = "XAXBH7-P63mmc.vasp"',
    )
    config_path.write_text(config_text, encoding="utf-8")

    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    manifest_path = Path("ht_out/03.XA_Li/period_2/workflow_manifest.json")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["input_structure"] = "/old/machine/work/XA_XB_7.vasp"
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")

    case_status_path = Path("ht_out/03.XA_Li/period_2/03.Li2H7/case_status.json")
    case_status = json.loads(case_status_path.read_text(encoding="utf-8"))
    case_status["input_structure"] = "/old/machine/work/XA_XB_7.vasp"
    case_status_path.write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output="")

    refreshed_case = json.loads(case_status_path.read_text(encoding="utf-8"))
    expected_structure = str(Path("ht_out/03.XA_Li/period_2/03.Li2H7/XAXBH7-P63mmc.vasp").resolve())
    assert refreshed_case["input_structure"] == expected_structure
    assert workflow_status["cases"]["03.XA_Li/period_2/03.Li2H7"]["input_structure"] == expected_structure


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

    bundle = build_submit_scripts("ht_out", 1)
    target_unit = next(
        unit
        for script_info in bundle["scripts"]
        for unit in script_info["units"]
        if unit["case_rel"].endswith("04.LiBeH7") and unit["pressure_key"] == "20"
    )
    expected_case_config = str(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").resolve())

    assert target_unit["steps"] == ["qe_phonon"]
    assert target_unit["commands"] == [
        f'cd "{case_dir.resolve()}"',
        f"qe_phonon_split_index -i vasp_opt/CONTCAR -c {expected_case_config} -s",
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

    run_workflow(str(config_path), "ht_out", run_calc=False, submit=False)

    script_text = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8")
    expected_case_config = str(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").resolve())
    assert f"qe_phonon_nosplit -i vasp_opt/CONTCAR -c {expected_case_config} -r" in script_text


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

    continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))
    expected_case_config = str((pressure_dir.parent / "input.toml").resolve())

    target_calls = [
        (command, cwd)
        for command, cwd in calls
        if Path(cwd).resolve() == pressure_dir.resolve() and command[0] == "qe_phonon_split_index"
    ]
    assert target_calls == [(["qe_phonon_split_index", "-i", "vasp_opt/CONTCAR", "-c", expected_case_config, "-r"], str(pressure_dir.resolve()))]


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
    case_status = json.loads((pressure_dir.parent / "case_status.json").read_text(encoding="utf-8"))
    case_status["pressures"]["20"]["mattersim_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_scf"] = "finished"
    case_status["pressures"]["20"]["vasp_eledos"] = "error"
    (pressure_dir.parent / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

    calls = []
    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", lambda command, cwd: calls.append((command, cwd)))
    original_discover_cases = __import__("substitutionflow.workflow", fromlist=["discover_cases"]).discover_cases

    def only_target_case(output_dir):
        cases = original_discover_cases(output_dir)
        return {key: value for key, value in cases.items() if Path(value["case_dir"]).resolve() == pressure_dir.parent.resolve()}

    monkeypatch.setattr("substitutionflow.workflow.discover_cases", only_target_case)

    continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))

    target_calls = [(command, cwd) for command, cwd in calls if Path(cwd).resolve() == pressure_dir.resolve()]
    assert target_calls == []
    case_status = json.loads((pressure_dir.parent / "case_status.json").read_text(encoding="utf-8"))
    assert case_status["pressures"]["20"]["vasp_eledos"] == "error"
    assert case_status["pressures"]["20"]["qe_phonon"] == "unstarted"


def test_rewrite_workflow_states_only_rewrites_status_files(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status, load_case_status, load_workflow_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import rewrite_workflow_states

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    case_dir = pressure_dir.parent
    case_status = load_case_status(str(case_dir))
    case_status["pressures"]["20"]["vasp_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_scf"] = "finished"
    (case_dir / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)


    summary = rewrite_workflow_states("ht_out", config_file=str(config_path))

    updated_case = load_case_status(str(case_dir))
    workflow_status = load_workflow_status("ht_out")
    assert summary.startswith("总 case 数: ")
    assert "总未完成单元数" in summary
    assert updated_case["pressures"]["20"]["vasp_opt"] == "finished"
    assert updated_case["pressures"]["20"]["vasp_scf"] == "unstarted"
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]["vasp_scf"] == "unstarted"
    assert Path("ht_out/submit_group_01.sh").exists()


def test_reduce_qe_phonon_workflow_status_for_nosplit_and_split(chdir_work):
    from substitutionflow.status import reduce_qe_phonon_workflow_status

    nosplit_unstarted = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished"},
        "q2r": "unstarted",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
    }
    nosplit_running = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished"},
        "q2r": "running",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
    }
    split_unstarted = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished", "tasks": {"q_0001": "unstarted", "q_0002": "unstarted"}},
        "merge": "unstarted",
        "q2r": "unstarted",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
    }
    split_running = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished", "tasks": {"q_0001": "finished", "q_0002": "running"}},
        "merge": "unstarted",
        "q2r": "unstarted",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
    }
    split_error = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished", "tasks": {"q_0001": "finished", "q_0002": "finished"}},
        "merge": "error",
        "q2r": "unstarted",
        "matdyn_phband": "unstarted",
        "matdyn_phdos": "unstarted",
    }
    split_finished = {
        "scf": "finished",
        "ph": {"state": "finished", "dyn0": "finished", "tasks": {"q_0001": "finished", "q_0002": "finished"}},
        "merge": "finished",
        "q2r": "finished",
        "matdyn_phband": "finished",
        "matdyn_phdos": "finished",
    }

    assert reduce_qe_phonon_workflow_status("qe_phonon_nosplit", nosplit_unstarted) == "unstarted"
    assert reduce_qe_phonon_workflow_status("qe_phonon_nosplit", nosplit_running) == "error"
    assert reduce_qe_phonon_workflow_status("qe_phonon_split_index", split_unstarted) == "unstarted"
    assert reduce_qe_phonon_workflow_status("qe_phonon_split_index", split_running) == "error"
    assert reduce_qe_phonon_workflow_status("qe_phonon_split_coords", split_error) == "error"
    assert reduce_qe_phonon_workflow_status("qe_phonon_split_index", split_finished) == "finished"


def test_refresh_status_qe_phonon_uses_workflow_status_instead_of_directory_trace(chdir_work):
    from substitutionflow.status import create_initial_status, refresh_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    qe_dir = pressure_dir / "qe_phonon_split_index"
    qe_dir.mkdir(parents=True, exist_ok=True)
    (qe_dir / "stale.txt").write_text("leftover\n", encoding="utf-8")

    workflow_status = refresh_status("ht_out", squeue_output="")
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]["qe_phonon"] == "unstarted"

    _write_qe_phonon_workflow_status(qe_dir, tasks={"q_0001": "finished", "q_0002": "finished"})
    workflow_status = refresh_status("ht_out", squeue_output="")
    assert workflow_status["cases"]["03.XA_Li/period_2/04.LiBeH7"]["pressures"]["20"]["qe_phonon"] == "finished"


def test_rewrite_workflow_states_resets_errors_and_repairs_qe_phonon_poscar_only(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status, load_case_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import rewrite_workflow_states

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    _append_qe_phonon_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    case_dir = pressure_dir.parent
    case_status = load_case_status(str(case_dir))
    case_status["pressures"]["20"]["vasp_opt"] = "error"
    case_status["pressures"]["20"]["vasp_eledos"] = "finished"
    case_status["pressures"]["20"]["qe_phonon"] = "error"
    case_status["pressures"]["50"]["vasp_scf"] = "error"
    (case_dir / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("fresh contcar\n", encoding="utf-8")
    eledos_dir = pressure_dir / "vasp_eledos"
    eledos_dir.mkdir(parents=True, exist_ok=True)
    (eledos_dir / "DOSCAR").write_text("dos\n", encoding="utf-8")
    _write_scf_like_outcar(eledos_dir / "OUTCAR", converged=True, iterations=4, nelm=60)
    qe_dir = pressure_dir / "qe_phonon_split_index"
    qe_dir.mkdir(parents=True, exist_ok=True)
    (qe_dir / "input.toml").write_text("[qe]\n", encoding="utf-8")
    (qe_dir / "POSCAR").write_text("old poscar\n", encoding="utf-8")
    (qe_dir / "scf.in").write_text("stale\n", encoding="utf-8")
    _write_qe_phonon_workflow_status(
        qe_dir,
        tasks={"q_0001": "error", "q_0002": "finished"},
        merge="error",
        q2r="unstarted",
        matdyn_phband="unstarted",
        matdyn_phdos="unstarted",
    )


    buffer = io.StringIO()
    with redirect_stdout(buffer):
        summary = rewrite_workflow_states("ht_out", config_file=str(config_path))

    updated_case = load_case_status(str(case_dir))
    rewritten_output = buffer.getvalue()
    repaired_poscar = (qe_dir / "POSCAR").read_text(encoding="utf-8")
    qe_workflow_status = json.loads((qe_dir / "workflow_status.json").read_text(encoding="utf-8"))

    assert summary.startswith("总 case 数: ")
    assert "重写步骤数: 2" in summary
    assert "重写 qe_phonon 子步骤数: 2" in summary
    assert updated_case["pressures"]["20"]["vasp_opt"] == "unstarted"
    assert updated_case["pressures"]["20"]["qe_phonon"] == "unstarted"
    assert updated_case["pressures"]["20"]["vasp_eledos"] == "finished"
    assert updated_case["pressures"]["50"]["vasp_scf"] == "unstarted"
    assert "event:rewrite" not in rewritten_output
    assert "from:error" not in rewritten_output
    assert "to:unstarted" not in rewritten_output
    assert "formula:LiBeH7" not in rewritten_output
    assert "case:ht_out/03.XA_Li/period_2/04.LiBeH7/20" not in rewritten_output
    assert "FORMULA" in summary
    assert "PRESSURE" in summary
    assert "STEPS" in summary
    assert "CASE" in summary
    assert "重写结果:" in summary
    assert "LiBeH7" in summary
    assert "20" in summary
    assert "vasp_opt,qe_phonon" in summary
    assert "ht_out/03.XA_Li/period_2/04.LiBeH7" in summary
    assert (qe_dir / "input.toml").read_text(encoding="utf-8") == "[qe]\n"
    assert qe_workflow_status["merge"] == "unstarted"
    assert qe_workflow_status["ph"]["tasks"]["q_0001"] == "unstarted"
    assert qe_workflow_status["ph"]["tasks"]["q_0002"] == "finished"
    assert repaired_poscar == "fresh contcar\n"
    assert (qe_dir / "scf.in").exists()
    assert "刷新状态:" in rewritten_output
    assert "重写状态:" in rewritten_output
    assert "ht_out/03.XA_Li/period_2/04.LiBeH7" in rewritten_output


def test_rewrite_workflow_states_with_config_rewrites_case_input_truth_sources(chdir_work):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import rewrite_workflow_states

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
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    new_config_path = Path("input_ht_new.toml")
    new_config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'potcar_dir = "potcar_lib"',
            'potcar_dir = "potcar_lib_new"',
        ),
        encoding="utf-8",
    )

    summary = rewrite_workflow_states("ht_out", config_file=str(new_config_path))
    case_input = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").read_text(encoding="utf-8")

    assert 'potcar_dir = "potcar_lib_new"' in case_input
    assert "重写 case 配置数:" in summary


def test_rewrite_workflow_states_groups_multiple_steps_into_single_row(chdir_work):
    from substitutionflow.status import create_initial_status, load_case_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import rewrite_workflow_states

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    case_status = load_case_status(str(case_dir))
    case_status["pressures"]["20"]["vasp_opt"] = "error"
    case_status["pressures"]["20"]["vasp_scf"] = "error"
    (case_dir / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")
    pressure_dir = case_dir / "20"
    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=False)
    scf_dir = pressure_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=False)

    summary = rewrite_workflow_states("ht_out", config_file=str(config_path))

    target_lines = [line for line in summary.splitlines() if "ht_out/03.XA_Li/period_2/04.LiBeH7" in line]
    assert len(target_lines) == 1
    assert "vasp_opt,vasp_scf" in target_lines[0]


def test_rewrite_workflow_states_without_main_step_rewrites_omits_result_table(chdir_work):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import rewrite_workflow_states

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    _append_qe_phonon_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    qe_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20/qe_phonon_split_index")
    qe_dir.mkdir(parents=True, exist_ok=True)
    _write_qe_phonon_workflow_status(
        qe_dir,
        tasks={"q_0001": "error", "q_0002": "finished"},
        merge="error",
        q2r="unstarted",
        matdyn_phband="unstarted",
        matdyn_phdos="unstarted",
    )

    summary = rewrite_workflow_states("ht_out", config_file=str(config_path))

    assert "重写 qe_phonon 子步骤数: 2" in summary
    assert "重写结果:" not in summary
    assert "FORMULA" not in summary


def test_continue_workflow_submit_with_config_rewrites_case_inputs_before_submit(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

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
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    new_config_path = Path("input_ht_new.toml")
    new_config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'potcar_dir = "potcar_lib"',
            'potcar_dir = "potcar_lib_new"',
        ),
        encoding="utf-8",
    )

    submitted = []
    monkeypatch.setattr("substitutionflow.workflow._submit_group_scripts", lambda output_dir, bundle: submitted.append((output_dir, bundle["total_units"])))

    continue_workflow("ht_out", run_calc=False, submit=True, config_file=str(new_config_path))
    case_input = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").read_text(encoding="utf-8")

    assert submitted
    assert 'potcar_dir = "potcar_lib_new"' in case_input


def test_continue_workflow_after_rewrite_states_advances_from_first_unfinished_step(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status, load_case_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow, rewrite_workflow_states

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    _append_qe_phonon_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    qe_dir = pressure_dir / "qe_phonon_split_index"
    qe_dir.mkdir(parents=True, exist_ok=True)
    (qe_dir / "input.toml").write_text("[qe]\n", encoding="utf-8")
    (qe_dir / "POSCAR").write_text("old poscar\n", encoding="utf-8")
    _write_qe_phonon_workflow_status(
        qe_dir,
        tasks={"q_0001": "finished", "q_0002": "finished"},
        merge="error",
        q2r="unstarted",
        matdyn_phband="unstarted",
        matdyn_phdos="unstarted",
    )

    case_status_path = pressure_dir.parent / "case_status.json"
    case_status = load_case_status(str(pressure_dir.parent))
    case_status["pressures"]["20"]["vasp_scf"] = "error"
    case_status_path.write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

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
        elif command[0] == "qe_phonon_split_index":
            _write_qe_phonon_workflow_status(
                Path(cwd) / "qe_phonon_split_index",
                tasks={"q_0001": "finished", "q_0002": "finished"},
            )

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)

    rewrite_workflow_states("ht_out", config_file=str(config_path))
    summary = continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))

    target_calls = [
        (command, cwd)
        for command, cwd in calls
        if Path(cwd).resolve() == pressure_dir.resolve()
    ]
    assert "总 case 数" in summary
    assert [command[0] for command, _ in target_calls[:4]] == [
        "vasp_scf",
        "vasp_eledos",
        "vasp_process_eledos",
        "qe_phonon_split_index",
    ]
    assert all(command[0] != "vasp_opt" for command, _ in target_calls)


def test_continue_workflow_run_refreshes_status_before_advancing(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status, load_case_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    case_dir = pressure_dir.parent
    case_status = load_case_status(str(case_dir))
    case_status["pressures"]["20"]["vasp_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_scf"] = "finished"
    (case_dir / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    scf_dir = pressure_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=False)

    calls = []
    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", lambda command, cwd: calls.append((command, cwd)))
    import substitutionflow.workflow as workflow_module

    original_discover_cases = workflow_module.discover_cases

    def only_target_case(output_dir):
        cases = original_discover_cases(output_dir)
        return {key: value for key, value in cases.items() if Path(value["case_dir"]).resolve() == case_dir.resolve()}

    monkeypatch.setattr(workflow_module, "discover_cases", only_target_case)

    continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))

    target_calls = [(command, cwd) for command, cwd in calls if Path(cwd).resolve() == pressure_dir.resolve()]
    assert target_calls == []
    updated_case = load_case_status(str(case_dir))
    assert updated_case["pressures"]["20"]["vasp_scf"] == "error"


def test_continue_workflow_submit_refreshes_status_before_building_scripts(chdir_work, monkeypatch):
    from substitutionflow.status import create_initial_status, load_case_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    case_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7")
    pressure_dir = case_dir / "20"
    case_status = load_case_status(str(case_dir))
    case_status["pressures"]["20"]["mattersim_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_scf"] = "finished"
    (case_dir / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

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
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=False)

    submitted = []
    monkeypatch.setattr("substitutionflow.workflow.submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    summary = continue_workflow("ht_out", run_calc=False, submit=True, config_file=str(config_path))

    script_lines = Path("ht_out/submit_group_01.sh").read_text(encoding="utf-8").splitlines()
    target_cd = f'cd "{pressure_dir.resolve()}"'
    target_index = script_lines.index(target_cd)
    assert "总未完成单元数" in summary
    assert submitted == [("ht_out", "submit_group_01.sh")]
    assert script_lines[target_index + 1].startswith("vasp_scf ")


def test_continue_workflow_default_mode_refreshes_status_summary(chdir_work):
    from substitutionflow.status import create_initial_status, load_case_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pressure_dir = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20")
    case_dir = pressure_dir.parent
    case_status = load_case_status(str(case_dir))
    case_status["pressures"]["20"]["vasp_opt"] = "finished"
    case_status["pressures"]["20"]["vasp_scf"] = "finished"
    (case_dir / "case_status.json").write_text(json.dumps(case_status, ensure_ascii=False, indent=2), encoding="utf-8")

    opt_dir = pressure_dir / "vasp_opt"
    opt_dir.mkdir(parents=True, exist_ok=True)
    (opt_dir / "CONTCAR").write_text("contcar\n", encoding="utf-8")
    _write_opt_outcar(opt_dir / "OUTCAR", converged=True)
    scf_dir = pressure_dir / "vasp_scf"
    scf_dir.mkdir(parents=True, exist_ok=True)
    (scf_dir / "CHGCAR").write_text("chg\n", encoding="utf-8")
    (scf_dir / "WAVECAR").write_text("wav\n", encoding="utf-8")
    _write_scf_like_outcar(scf_dir / "OUTCAR", converged=False)

    buffer = io.StringIO()
    with redirect_stdout(buffer):
        summary = continue_workflow("ht_out", run_calc=False, submit=False, config_file=str(config_path))

    updated_case = load_case_status(str(case_dir))
    progress_output = buffer.getvalue()
    assert "总 case 数" in summary
    assert "总未完成单元数" in summary
    assert updated_case["pressures"]["20"]["vasp_scf"] == "error"
    assert "刷新状态:" in progress_output
    assert Path("ht_out/submit_group_01.sh").exists()


def test_continue_workflow_default_mode_with_config_rewrites_case_inputs_and_generates_submit_scripts(chdir_work):
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases
    from substitutionflow.workflow import continue_workflow

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    new_config_path = Path("input_ht_new.toml")
    new_config_path.write_text(
        config_path.read_text(encoding="utf-8").replace(
            'potcar_dir = "potcar_lib"',
            'potcar_dir = "potcar_lib_new"',
        ),
        encoding="utf-8",
    )

    summary = continue_workflow("ht_out", run_calc=False, submit=False, config_file=str(new_config_path))

    case_input = Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").read_text(encoding="utf-8")
    assert 'potcar_dir = "potcar_lib_new"' in case_input
    assert "总未完成单元数" in summary
    assert Path("ht_out/submit_group_01.sh").exists()

def test_continue_workflow_run_prints_case_pressure_step_progress(chdir_work, monkeypatch):
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

    def fake_run_cli_command(command, cwd):
        calls.append((command, cwd))

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)

    output = io.StringIO()
    with redirect_stdout(output):
        continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))

    captured = output.getvalue()
    assert "运行进度:" in captured
    assert "first:vasp_eledos" in captured
    assert "04.vasp_eledos" in captured
    assert "event:break" in captured
    assert "state:error" in captured
    assert "case:ht_out/03.XA_Li/period_2/04.LiBeH7/20" in captured
    assert "pressure:20" in captured
    assert not any(
        command[0] == "vasp_eledos" and Path(cwd).resolve() == pressure_dir.resolve()
        for command, cwd in calls
    )


def test_run_workflow_run_prints_skip_run_stop_progress(chdir_work, monkeypatch):
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

    def fake_run_cli_command(command, cwd):
        cwd_path = Path(cwd)
        if command[0] == "vasp_opt" and "-r" not in command:
            return
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
            _write_scf_like_outcar(cwd_path / "vasp_eledos/OUTCAR", converged=False)
            (cwd_path / "vasp_eledos/TDOS.dat").write_text("tdos\n", encoding="utf-8")

    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", fake_run_cli_command)

    output = io.StringIO()
    with redirect_stdout(output):
        run_workflow(str(config_path), "ht_out", run_calc=True, submit=False)

    captured = output.getvalue()
    assert "运行进度:" in captured
    assert "first:vasp_opt" in captured
    assert "01.vasp_opt" in captured
    assert "event:run" in captured
    assert "reason:           case:ht_out/03.XA_Li/period_2/03.Li2H7/20" in captured
    assert "case:ht_out/03.XA_Li/period_2/03.Li2H7/20" in captured
    assert "02.vasp_scf" in captured
    assert "03.vasp_eledos" in captured
    assert "event:break" in captured
    assert "state:error" in captured


def test_continue_treats_qe_phonon_trace_without_workflow_status_as_unstarted(chdir_work, monkeypatch):
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
    qe_dir = pressure_dir / "qe_phonon_split_index"
    qe_dir.mkdir(parents=True, exist_ok=True)
    (qe_dir / "run_ph.sh").write_text("#!/bin/bash\n", encoding="utf-8")

    calls = []
    monkeypatch.setattr("substitutionflow.workflow.run_cli_command", lambda command, cwd: calls.append((command, cwd)))

    continue_workflow("ht_out", run_calc=True, submit=False, config_file=str(config_path))

    target_calls = [(command, cwd) for command, cwd in calls if Path(cwd).resolve() == pressure_dir.resolve()]
    assert any(command[0].startswith("qe_phonon") for command, _ in target_calls)


def test_build_submit_scripts_split_units_without_breaking_chain(chdir_work):
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

    prepared_step_dir = Path("ht_out/03.XA_Li/period_2/03.Li2H7/50/mattersim_opt")
    prepared_step_dir.mkdir(parents=True, exist_ok=True)
    (prepared_step_dir / "POSCAR").write_text("poscar\n", encoding="utf-8")

    bundle = build_submit_scripts("ht_out", 3)

    assert bundle["configured_groups"] == 3
    assert bundle["actual_groups"] == 3
    assert bundle["total_units"] == 12

    group_texts = [
        Path(script_info["script_path"]).read_text(encoding="utf-8")
        for script_info in bundle["scripts"]
    ]
    joined = "\n".join(group_texts)
    assert "03.Li2H7/50" in joined

    unit_hits = [text.count('cd "/work') + text.count('cd "') for text in group_texts]
    assert sum(script_info["unit_count"] for script_info in bundle["scripts"]) == 12
    target_units = [
        unit
        for script_info in bundle["scripts"]
        for unit in script_info["units"]
        if unit["case_rel"].endswith("04.LiBeH7") and unit["pressure_key"] == "20"
    ]
    assert len(target_units) == 1
    target_unit = target_units[0]
    target_text = "\n".join(target_unit["commands"])
    expected_case_config = str(Path("ht_out/03.XA_Li/period_2/04.LiBeH7/input.toml").resolve())
    assert 'cd "/tmp' in target_text or 'cd "' in target_text
    assert f'cd "{Path("ht_out/03.XA_Li/period_2/04.LiBeH7/20").resolve()}"' in target_text
    assert "vasp_scf -i vasp_opt/CONTCAR -c ../input.toml -r" in target_text
    assert "vasp_eledos -i vasp_scf/POSCAR -c ../input.toml -r" in target_text
    assert "vasp_process_eledos -d ./vasp_eledos --elements-total Li Be H" in target_text
    assert f"qe_phonon_split_index -i vasp_opt/CONTCAR -c {expected_case_config} -s" in target_text
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
    monkeypatch.setattr("substitutionflow.workflow.submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    output = io.StringIO()
    with redirect_stdout(output):
        summary = run_workflow(str(config_path), "ht_out", run_calc=False, submit=True)

    captured = output.getvalue()
    assert "计划分组数: 4" in summary
    assert len(submitted) == 4
    assert "提交进度:" in captured
    assert "unit:first:mattersim" in captured
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
    monkeypatch.setattr("substitutionflow.workflow.submit_script", lambda work_dir, script_name: submitted.append((work_dir, script_name)))

    output = io.StringIO()
    with redirect_stdout(output):
        summary = continue_workflow("ht_out", run_calc=False, submit=True, config_file=str(config_path))

    captured = output.getvalue()
    assert "计划分组数: 5" in summary
    assert len(submitted) == 5
    assert "提交进度:" in captured
    assert "unit:first:" in captured
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


def test_substitution_continue_cli_rewrite_states_mode(chdir_work, monkeypatch):
    from substitutionflow import cli_continue as module

    monkeypatch.setattr(
        "substitutionflow.cli_continue.rewrite_workflow_states",
        lambda output_dir, config_file=None: f"rewrite:{output_dir}:{config_file}",
    )
    monkeypatch.setattr(
        "substitutionflow.cli_continue.continue_workflow",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("不应调用 continue_workflow")),
    )
    monkeypatch.setattr(sys, "argv", ["substitution_continue", "-o", "ht_out", "-c", "input_ht.toml", "-rws"])

    buffer = io.StringIO()
    with redirect_stdout(buffer):
        module.main()

    assert buffer.getvalue().strip() == "rewrite:ht_out:input_ht.toml"


def test_substitution_continue_cli_passes_config_to_continue_workflow(chdir_work, monkeypatch):
    from substitutionflow import cli_continue as module

    monkeypatch.setattr(
        "substitutionflow.cli_continue.rewrite_workflow_states",
        lambda *args, **kwargs: (_ for _ in ()).throw(AssertionError("不应调用 rewrite_workflow_states")),
    )
    monkeypatch.setattr(
        "substitutionflow.cli_continue.continue_workflow",
        lambda output_dir, run_calc=False, submit=False, config_file=None: f"{output_dir}|{run_calc}|{submit}|{config_file}",
    )
    monkeypatch.setattr(sys, "argv", ["substitution_continue", "-o", "ht_out", "-c", "new_input.toml", "-s"])

    buffer = io.StringIO()
    with redirect_stdout(buffer):
        module.main()

    assert buffer.getvalue().strip() == "ht_out|False|True|new_input.toml"


def test_substitution_continue_cli_requires_config(chdir_work, monkeypatch):
    from substitutionflow import cli_continue as module

    monkeypatch.setattr(sys, "argv", ["substitution_continue", "-o", "ht_out"])

    try:
        module.main()
        assert False, "预期缺少 -c/--config 时抛出 SystemExit"
    except SystemExit as exc:
        assert exc.code == 2


def test_substitution_continue_cli_rewrite_states_conflicts_with_run(chdir_work, monkeypatch):
    from substitutionflow import cli_continue as module

    monkeypatch.setattr(sys, "argv", ["substitution_continue", "-o", "ht_out", "-rws", "-r"])

    try:
        module.main()
        assert False, "预期参数冲突时抛出 SystemExit"
    except SystemExit as exc:
        assert exc.code == 2


def test_substitution_continue_cli_rewrite_states_conflicts_with_submit(chdir_work, monkeypatch):
    from substitutionflow import cli_continue as module

    monkeypatch.setattr(sys, "argv", ["substitution_continue", "-o", "ht_out", "-rws", "-s"])

    try:
        module.main()
        assert False, "预期参数冲突时抛出 SystemExit"
    except SystemExit as exc:
        assert exc.code == 2


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


def test_prepare_qe_pseudo_discovers_candidates_and_parses_upf(chdir_work):
    from substitutionflow.prepare_qe_pseudo import discover_qe_candidate_groups
    from substitutionflow.prepare_potcar import discover_required_elements

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

    pseudo_root = Path("qe_pseudo_source")
    _write_mock_upf(pseudo_root / "Li.upf", "Li", 1.0, "2s1")
    _write_mock_upf(pseudo_root / "li_pbe_v1.uspp", "Li", 3.0, "1s2 2s1")
    _write_mock_upf(pseudo_root / "Be.upf", "Be", 2.0, "2s2")
    _write_mock_upf(pseudo_root / "B.upf", "B", 3.0, "2s2 2p1")
    _write_mock_upf(pseudo_root / "H.upf", "H", 1.0, "1s1")

    elements = discover_required_elements(str(config_path))
    assert elements == ["H", "Li", "Be", "B"]

    groups = discover_qe_candidate_groups(str(pseudo_root), ["Li"])
    assert [candidate.filename for candidate in groups["Li"]] == ["Li.upf", "li_pbe_v1.uspp"]
    assert groups["Li"][0].config_text == "2s1"
    assert groups["Li"][1].z_valence == 3.0


def test_prepare_qe_pseudo_parses_valence_configuration_like_prepare_potcar(chdir_work):
    from substitutionflow.prepare_qe_pseudo import parse_qe_pseudo_candidate

    pseudo_path = Path("H.pbe-rrkjus_psl.1.0.0.UPF")
    _write_realistic_mock_upf(
        pseudo_path,
        "H",
        1.0,
        "    1S  1  0  1.00      0.800      1.000    -0.477210",
        (
            "3\n"
            "1S  1  0  1.00  0.00  0.80  1.00  0.0\n"
            "1S  1  0  0.00  0.05  0.80  1.00  0.0\n"
            "2P  2  1 -1.00  0.05  0.75  0.75  0.0"
        ),
    )

    candidate = parse_qe_pseudo_candidate(str(pseudo_path), "H")

    assert candidate.z_valence == 1.0
    assert candidate.config_text == "1s1"


def test_prepare_qe_pseudo_copies_files_reuses_log_without_rewriting_case_input(chdir_work, monkeypatch):
    from substitutionflow.prepare_qe_pseudo import prepare_qe_pseudo_library
    from substitutionflow.status import create_initial_status
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    _append_qe_phonon_config(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"',
        'structure_filename = "XAXBH7-P63mmc.vasp"',
    )
    config_path.write_text(config_text, encoding="utf-8")

    generate_cases(str(structure_path), str(config_path), "ht_out")
    create_initial_status("ht_out")

    pseudo_root = Path("qe_pseudo_source")
    _write_mock_upf(pseudo_root / "H.upf", "H", 1.0, "1s1")
    _write_mock_upf(pseudo_root / "Li_a.upf", "Li", 1.0, "2s1")
    _write_mock_upf(pseudo_root / "Li_b.upf", "Li", 3.0, "1s2 2s1")
    _write_mock_upf(pseudo_root / "Be.upf", "Be", 2.0, "2s2")
    _write_mock_upf(pseudo_root / "B.upf", "B", 3.0, "2s2 2p1")
    pseudo_root_abs = pseudo_root.resolve()

    answers = iter(["2"])
    monkeypatch.setattr("builtins.input", lambda _prompt="": next(answers))

    os.chdir("ht_out")
    log_path = prepare_qe_pseudo_library(str(Path("..") / config_path), str(pseudo_root_abs))

    qe_lib_dir = Path("qe_pseudo_lib")
    assert Path(log_path).exists()
    assert (qe_lib_dir / "Li_b.upf").exists()
    assert (qe_lib_dir / "Be.upf").exists()
    assert (qe_lib_dir / "B.upf").exists()
    assert (qe_lib_dir / "H.upf").exists()

    log_text = Path(log_path).read_text(encoding="utf-8")
    assert "Li = Li_b.upf" in log_text

    case_input = Path("03.XA_Li/period_2/04.LiBeH7/input.toml").read_text(encoding="utf-8")
    assert 'pp_dir = "qe_pseudo_lib"' in case_input

    qe_case_input = Path("03.XA_Li/period_2/03.Li2H7/input.toml").read_text(encoding="utf-8")
    assert 'pp_dir = "qe_pseudo_lib"' in qe_case_input

    Path("qe_pseudo_lib/qe_pseudo_lib.log").write_text("Li = Li_a.upf\nBe = Be.upf\nB = B.upf\nH = H.upf\n", encoding="utf-8")
    monkeypatch.setattr("builtins.input", lambda _prompt="": (_ for _ in ()).throw(AssertionError("不应进入交互")))
    prepare_qe_pseudo_library(str(Path("..") / config_path), str(pseudo_root_abs))
    assert (qe_lib_dir / "Li_a.upf").read_text(encoding="utf-8") == (pseudo_root_abs / "Li_a.upf").read_text(encoding="utf-8")


def test_prepare_qe_pseudo_supports_qe_phonon_generated_inputs(chdir_work, monkeypatch):
    from qeflow.phonon._shared import create_context, prepare_root_scf_step
    from substitutionflow.prepare_qe_pseudo import prepare_qe_pseudo_library
    from substitutionflow.structure_substitution import generate_cases

    structure_path = Path("XAXBH7-P63mmc.vasp")
    structure_path.write_text(PLACEHOLDER_POSCAR, encoding="utf-8")
    config_path = Path("input_ht.toml")
    _write_ht_config_without_mattersim(config_path)
    _append_qe_phonon_config(config_path)
    config_text = config_path.read_text(encoding="utf-8").replace(
        'structure_filename = "structure.vasp"',
        'structure_filename = "XAXBH7-P63mmc.vasp"',
    )
    config_path.write_text(config_text, encoding="utf-8")

    generate_cases(str(structure_path), str(config_path), "ht_out")

    pseudo_root = Path("qe_pseudo_source")
    _write_mock_upf(pseudo_root / "H.upf", "H", 1.0, "1s1")
    _write_mock_upf(pseudo_root / "Li.upf", "Li", 1.0, "2s1")
    _write_mock_upf(pseudo_root / "Be.upf", "Be", 2.0, "2s2")
    _write_mock_upf(pseudo_root / "B.upf", "B", 3.0, "2s2 2p1")
    pseudo_root_abs = pseudo_root.resolve()

    monkeypatch.setattr("builtins.input", lambda _prompt="": (_ for _ in ()).throw(AssertionError("不应进入交互")))
    os.chdir("ht_out")
    prepare_qe_pseudo_library(str(Path("..") / config_path), str(pseudo_root_abs))
    expected_qe_dir = str(Path("qe_pseudo_lib").resolve())

    pressure_dir = Path("03.XA_Li/period_2/03.Li2H7/20")
    vasp_opt_dir = pressure_dir / "vasp_opt"
    vasp_opt_dir.mkdir(parents=True, exist_ok=True)
    (vasp_opt_dir / "CONTCAR").write_text(PLACEHOLDER_POSCAR.replace("XB", "Li").replace("XA", "Li"), encoding="utf-8")

    os.chdir(pressure_dir)
    context = create_context("../input.toml", "vasp_opt/CONTCAR", workflow_name="qe_phonon_split_index")
    prepare_root_scf_step(context)

    scf_text = (Path(context.workspace) / "scf.in").read_text(encoding="utf-8")
    assert f"pseudo_dir = '{expected_qe_dir}'" in scf_text
    assert "Li.upf" in scf_text
    assert "H.upf" in scf_text


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
    assert len(plot_data["summary_rows"]) == 3
    finished_row = next(row for row in plot_data["summary_rows"] if row["status"] == "finished")
    error_row = next(row for row in plot_data["summary_rows"] if row["status"] == "error")
    assert finished_row["values"]["XA"] == 1.2
    assert finished_row["values"]["XB"] == 2.3
    assert finished_row["values"]["H"] == 3.4
    assert finished_row["values"]["TDOS"] == 4.5
    assert error_row["values"]["XA"] != error_row["values"]["XA"]

    outputs = generate_xa_dir_plots(str(xa_dir))
    assert str((xa_dir / "eledos_summary_50.png").resolve()) in [str(Path(item).resolve()) for item in outputs]
    assert str((xa_dir / "eledos_summary_50.dat").resolve()) in [str(Path(item).resolve()) for item in outputs]
    assert str((xa_dir / "eledos_summary_100.png").resolve()) in [str(Path(item).resolve()) for item in outputs]
    assert str((xa_dir / "eledos_summary_100.dat").resolve()) in [str(Path(item).resolve()) for item in outputs]
    assert (xa_dir / "eledos_summary_50.png").exists()
    assert (xa_dir / "eledos_summary_50.dat").exists()
    assert (xa_dir / "eledos_summary_100.png").exists()
    assert (xa_dir / "eledos_summary_100.dat").exists()

    dat_content = (xa_dir / "eledos_summary_50.dat").read_text(encoding="utf-8")
    assert "体系化学配比名称 元素XA的费米能级处DOS 元素XB的费米能级处DOS H的费米能级处DOS TDOS status" in dat_content
    assert f"{error_row['formula']} nan nan nan nan error" in dat_content


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
    assert "处理 eledos:" in output
    assert "[########################]" in output
    assert "03.XA_Li/eledos_summary_50.png" in output or "03.XA_Li/eledos_summary_100.png" in output
