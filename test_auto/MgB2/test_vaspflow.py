"""VASP 模块测试 - API + CLI + 实际运行"""
import os
import shutil
import subprocess
import sys
import pytest
from pathlib import Path


# ============================================================================
# vasp_scf 测试
# ============================================================================

def test_vasp_scf_api(chdir_work):
    """API 测试：vasp_scf 文件生成"""
    from vaspflow.scf.vasp_scf import VaspSetup

    setup = VaspSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    # 验证文件生成
    assert (Path("vasp_scf") / "INCAR").exists()
    assert (Path("vasp_scf") / "POTCAR").exists()
    assert (Path("vasp_scf") / "KPOINTS").exists()
    assert (Path("vasp_scf") / "POSCAR").exists()
    assert (Path("vasp_scf") / "run_vasp.sh").exists()

    # 验证 INCAR 内容
    incar_content = (Path("vasp_scf") / "INCAR").read_text()
    assert "ENCUT" in incar_content
    assert "520" in incar_content


def test_vasp_scf_cli(chdir_work):
    """CLI 测试：vasp_scf 命令行"""
    result = subprocess.run(
        ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_scf") / "INCAR").exists()
    assert (Path("vasp_scf") / "POTCAR").exists()


def test_vasp_scf_submit_uses_run_vasp_script(chdir_work, monkeypatch):
    from vaspflow.scf import vasp_scf as module

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(sys, "argv", ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "-s"])

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "vasp_scf"
    assert captured["script_name"] == "run_vasp.sh"


def test_vasp_lobsterin_api(chdir_work):
    from argparse import Namespace
    from vaspflow.cohp.vasp_lobsterin import VaspLobsterinSetup

    for filename in ["WAVECAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR"]:
        Path(filename).write_text(f"{filename}\n", encoding="utf-8")

    setup = VaspLobsterinSetup(config_file="input_vasp.toml")
    args = Namespace(
        COHPEnergy=[-10.0, 5.0],
        species_custom=["Mg", "B"],
        zvalances=["Mg:3s,3p", "B:2s,2p"],
        d_limit=[1.0, 2.5],
        mode=0,
        custom_basis=["Mg:mg.sto"],
        config="input_vasp.toml",
    )

    setup.prepare_inputs(args)

    work_dir = Path("Mg_B_1.0_2.5")
    assert (work_dir / "lobsterin").exists()
    assert (work_dir / "run_lobster.sh").exists()
    assert (work_dir / "CONTCAR").is_symlink()

    lobsterin_content = (work_dir / "lobsterin").read_text(encoding="utf-8")
    assert "COHPstartEnergy  -10.0" in lobsterin_content
    assert "gaussianSmearingWidth 0.05" in lobsterin_content
    assert "basisfunctions Mg 3s 3p" in lobsterin_content
    assert "customSTOforAtom Mg  mg.sto" in lobsterin_content

    run_script_content = (work_dir / "run_lobster.sh").read_text(encoding="utf-8")
    assert "/home/mayuan/bin/lobster-5.1.1" in run_script_content


def test_vasp_lobsterin_submit_uses_run_lobster_script(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_lobsterin as module

    for filename in ["WAVECAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR"]:
        Path(filename).write_text(f"{filename}\n", encoding="utf-8")

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "vasp_lobsterin",
            "-c",
            "input_vasp.toml",
            "-s",
            "--species_custom",
            "Mg",
            "B",
            "-z",
            "Mg:3s,3p",
            "B:2s,2p",
            "-d",
            "1.0",
            "2.5",
        ],
    )

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "Mg_B_1.0_2.5"
    assert captured["script_name"] == "run_lobster.sh"


@pytest.mark.run
def test_vasp_scf_run(chdir_shared):
    """实际运行测试：vasp_scf --run"""
    result = subprocess.run(
        ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    print("STDOUT:", result.stdout)
    print("STDERR:", result.stderr)
    print("Return code:", result.returncode)

    assert result.returncode == 0
    assert (Path("vasp_scf") / "OUTCAR").exists()
    assert (Path("vasp_scf") / "CHGCAR").exists()
    assert (Path("vasp_scf") / "WAVECAR").exists()


# ============================================================================
# vasp_eband 测试
# ============================================================================

def test_vasp_eband_api(chdir_work):
    """API 测试：vasp_eband 文件生成"""
    # 创建假的 SCF 输出
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.eband.vasp_eband import VaspBandSetup

    setup = VaspBandSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_eband") / "INCAR").exists()
    incar_content = (Path("vasp_eband") / "INCAR").read_text()
    assert "ICHARG" in incar_content
    assert "11" in incar_content


def test_vasp_eband_cli(chdir_work):
    """CLI 测试：vasp_eband 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_eband", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_eband") / "INCAR").exists()


@pytest.mark.run
def test_vasp_eband_run(chdir_shared):
    """实际运行测试：vasp_eband --run（依赖 vasp_scf）"""
    # 确保 vasp_scf 已运行
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_eband", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_eband") / "OUTCAR").exists()


# ============================================================================
# vasp_eledos 测试
# ============================================================================

def test_vasp_eledos_api(chdir_work):
    """API 测试：vasp_eledos 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.eledos.vasp_eledos import VaspDosSetup

    setup = VaspDosSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_eledos") / "INCAR").exists()
    incar_content = (Path("vasp_eledos") / "INCAR").read_text()
    assert "ISMEAR" in incar_content


def test_vasp_eledos_cli(chdir_work):
    """CLI 测试：vasp_eledos 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_eledos", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_eledos") / "INCAR").exists()


@pytest.mark.run
def test_vasp_eledos_run(chdir_shared):
    """实际运行测试：vasp_eledos --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_eledos", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_eledos") / "OUTCAR").exists()


# ============================================================================
# vasp_fermisurface 测试
# ============================================================================

def test_vasp_fermisurface_api(chdir_work):
    """API 测试：vasp_fermisurface 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.fermisurface.vasp_fermisurface import VaspFermiSetup

    setup = VaspFermiSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_fermisurface") / "INCAR").exists()


def test_vasp_fermisurface_uses_vaspkit_261(chdir_work, monkeypatch):
    from vaspflow.fermisurface.vasp_fermisurface import VaspFermiSetup

    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    captured = {}

    class FakeProcess:
        def communicate(self, input=None):
            captured["input"] = input
            Path("vasp_fermisurface/KPOINTS").write_text("fake kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        captured["cmd"] = cmd
        captured["cwd"] = cwd
        return FakeProcess()

    monkeypatch.setattr("vaspflow.fermisurface.vasp_fermisurface.subprocess.Popen", fake_popen)

    setup = VaspFermiSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert captured["cmd"] == ["vaspkit"]
    assert captured["cwd"].endswith("vasp_fermisurface")
    assert captured["input"] == "261\n0.04\n"
    assert (Path("vasp_fermisurface") / "KPOINTS").exists()


def test_vasp_fermisurface_cli(chdir_work):
    """CLI 测试：vasp_fermisurface 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_fermi", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_fermisurface") / "INCAR").exists()


@pytest.mark.run
def test_vasp_fermisurface_run(chdir_shared):
    """实际运行测试：vasp_fermisurface --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_fermi", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_fermisurface") / "OUTCAR").exists()


# ============================================================================
# vasp_cohp_uniform / vasp_cohp_fatband 测试
# ============================================================================

def test_vasp_cohp_uniform_api(chdir_work):
    """API 测试：vasp_cohp_uniform 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp.vasp_cohp_uniform import VaspCohpUniformSetup

    setup = VaspCohpUniformSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_cohp_uniform") / "INCAR").exists()
    incar_content = (Path("vasp_cohp_uniform") / "INCAR").read_text()
    assert "ISYM" in incar_content


def test_vasp_cohp_uniform_cli(chdir_work, monkeypatch):
    """CLI 测试：vasp_cohp_uniform 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp import vasp_cohp_uniform as module

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = cwd

        def communicate(self, input=None):
            Path(self.cwd, "KPOINTS").write_text("fake uniform kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        return FakeProcess(cwd)

    monkeypatch.setattr("vaspflow.cohp.cohp_common.subprocess.Popen", fake_popen)
    monkeypatch.setattr(sys, "argv", ["vasp_cohp_uniform", "-i", "MgB2.vasp", "-c", "input_vasp.toml"])

    module.main()
    assert (Path("vasp_cohp_uniform") / "INCAR").exists()


@pytest.mark.run
def test_vasp_cohp_uniform_run(chdir_shared):
    """实际运行测试：vasp_cohp_uniform --run"""
    if shutil.which("vasp_cohp_uniform") is None:
        pytest.skip("当前环境未安装 vasp_cohp_uniform CLI。")

    if not (Path("vasp_scf") / "CHGCAR").exists():
        scf_result = subprocess.run(
            ["vasp_scf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
            capture_output=True,
            text=True,
            timeout=600,
        )
        if scf_result.returncode != 0:
            pytest.skip("当前环境无法完成前置 vasp_scf 真实计算，跳过 cohp_uniform --run 测试。")

    result = subprocess.run(
        ["vasp_cohp_uniform", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_cohp_uniform") / "OUTCAR").exists()


def test_vasp_cohp_fatband_api(chdir_work, monkeypatch):
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp.vasp_cohp_fatband import VaspCohpFatbandSetup

    captured = []

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = cwd

        def communicate(self, input=None):
            captured.append((self.cwd, input))
            if input == "303\n":
                Path(self.cwd, "KPATH.in").write_text("fake kpath\n", encoding="utf-8")
            else:
                Path(self.cwd, "KPOINTS").write_text("fake fatband kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        assert cmd == ["vaspkit"]
        return FakeProcess(cwd)

    monkeypatch.setattr("vaspflow.cohp.cohp_common.subprocess.Popen", fake_popen)

    setup = VaspCohpFatbandSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_cohp_fatband") / "INCAR").exists()
    assert (Path("vasp_cohp_fatband") / "KPATH.in").exists()
    assert (Path("vasp_cohp_fatband") / "KPOINTS").exists()
    assert captured[0][1] == "303\n"
    assert captured[1][1] == "251\n1\n0.03\n0.03\n"


def test_vasp_cohp_fatband_cli(chdir_work, monkeypatch):
    from vaspflow.cohp import vasp_cohp_fatband as module

    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    class FakeProcess:
        def __init__(self, cwd):
            self.cwd = cwd

        def communicate(self, input=None):
            if input == "303\n":
                Path(self.cwd, "KPATH.in").write_text("fake kpath\n", encoding="utf-8")
            else:
                Path(self.cwd, "KPOINTS").write_text("fake fatband kpoints\n", encoding="utf-8")
            return ("", "")

    def fake_popen(cmd, cwd=None, stdin=None, stdout=None, stderr=None, text=None):
        return FakeProcess(cwd)

    monkeypatch.setattr("vaspflow.cohp.cohp_common.subprocess.Popen", fake_popen)

    monkeypatch.setattr(sys, "argv", ["vasp_cohp_fatband", "-i", "MgB2.vasp", "-c", "input_vasp.toml"])

    module.main()
    assert (Path("vasp_cohp_fatband") / "INCAR").exists()


# ============================================================================
# vasp_locpot 测试
# ============================================================================

def test_vasp_locpot_api(chdir_work):
    """API 测试：vasp_locpot 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.locpot.vasp_locpot import VaspLocpotSetup

    setup = VaspLocpotSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_locpot") / "INCAR").exists()
    incar_content = (Path("vasp_locpot") / "INCAR").read_text()
    assert "LVHAR" in incar_content
    assert "NGX" in incar_content


def test_vasp_locpot_cli(chdir_work):
    """CLI 测试：vasp_locpot 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_locpot", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_locpot") / "INCAR").exists()


@pytest.mark.run
def test_vasp_locpot_run(chdir_shared):
    """实际运行测试：vasp_locpot --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_locpot", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_locpot") / "OUTCAR").exists()


# ============================================================================
# vasp_charge 测试
# ============================================================================

def test_vasp_charge_api(chdir_work):
    """API 测试：vasp_charge 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.charge.vasp_charge import VaspChargeSetup

    setup = VaspChargeSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    # 目录名包含能量范围
    charge_dirs = list(Path(".").glob("vasp_charge*"))
    assert len(charge_dirs) > 0

    charge_dir = charge_dirs[0]
    assert (charge_dir / "INCAR").exists()
    incar_content = (charge_dir / "INCAR").read_text()
    assert "LPARD" in incar_content
    assert "EINT" in incar_content


def test_vasp_charge_cli(chdir_work):
    """CLI 测试：vasp_charge 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_charge", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    charge_dirs = list(Path(".").glob("vasp_charge*"))
    assert len(charge_dirs) > 0


@pytest.mark.run
def test_vasp_charge_run(chdir_shared):
    """实际运行测试：vasp_charge --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_charge", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    charge_dirs = list(Path(".").glob("vasp_charge*"))
    assert len(charge_dirs) > 0
    assert (charge_dirs[0] / "OUTCAR").exists()


# ============================================================================
# vasp_elf 测试
# ============================================================================

def test_vasp_elf_api(chdir_work):
    """API 测试：vasp_elf 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.elf.vasp_elf import VaspElfSetup

    setup = VaspElfSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_elf") / "INCAR").exists()
    incar_content = (Path("vasp_elf") / "INCAR").read_text()
    assert "LELF" in incar_content


def test_vasp_elf_cli(chdir_work):
    """CLI 测试：vasp_elf 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_elf", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_elf") / "INCAR").exists()


@pytest.mark.run
def test_vasp_elf_run(chdir_shared):
    """实际运行测试：vasp_elf --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_elf", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_elf") / "OUTCAR").exists()


# ============================================================================
# vasp_opt 测试（不运行 --run，太耗时）
# ============================================================================

def test_vasp_opt_rv4_api(chdir_work):
    """API 测试：vasp_opt rv4 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    Path("KPOINTS").write_text("external kpoints\n", encoding="utf-8")
    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rv4")

    assert (Path("vasp_opt") / "INCAR_1").exists()
    assert (Path("vasp_opt") / "INCAR_2").exists()
    assert (Path("vasp_opt") / "INCAR_3").exists()
    assert (Path("vasp_opt") / "INCAR_4").exists()
    assert (Path("vasp_opt") / "fouropt.sh").exists()
    assert not (Path("vasp_opt") / "KPOINTS").exists()

    incar_1 = (Path("vasp_opt") / "INCAR_1").read_text(encoding="utf-8")
    incar_2 = (Path("vasp_opt") / "INCAR_2").read_text(encoding="utf-8")
    incar_3 = (Path("vasp_opt") / "INCAR_3").read_text(encoding="utf-8")
    incar_4 = (Path("vasp_opt") / "INCAR_4").read_text(encoding="utf-8")

    assert "KSPACING = 0.8" in incar_1
    assert "KSPACING = 0.6" in incar_2
    assert "KSPACING = 0.4" in incar_3
    assert f"KSPACING = {0.04 * 2 * 3.1415926}" in incar_4


def test_vasp_opt_rv1_api(chdir_work):
    """API 测试：vasp_opt rv1 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    Path("KPOINTS").write_text("external kpoints\n", encoding="utf-8")
    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rv1")

    assert (Path("vasp_opt") / "INCAR").exists()
    assert (Path("vasp_opt") / "oneopt.sh").exists()
    assert not (Path("vasp_opt") / "KPOINTS").exists()

    incar_content = (Path("vasp_opt") / "INCAR").read_text(encoding="utf-8")
    assert f"KSPACING = {0.04 * 2 * 3.1415926}" in incar_content


def test_vasp_opt_rvf_api(chdir_work):
    """API 测试：vasp_opt rvf 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    Path("KPOINTS").write_text("external kpoints\n", encoding="utf-8")
    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rvf")

    assert (Path("vasp_opt") / "INCAR").exists()
    assert (Path("vasp_opt") / "fopt.sh").exists()
    assert not (Path("vasp_opt") / "KPOINTS").exists()

    incar_content = (Path("vasp_opt") / "INCAR").read_text(encoding="utf-8")
    assert f"KSPACING = {0.04 * 2 * 3.1415926}" in incar_content


def test_vasp_opt_prefers_explicit_kspacing_over_kmesh(chdir_work):
    from pathlib import Path

    config_path = Path("input_vasp_opt_kspacing.toml")
    content = Path("input_vasp.toml").read_text(encoding="utf-8").replace(
        "[vasp_opt]\nkmesh = 0.04",
        "[vasp_opt]\nkmesh = 0.04\nKSPACING = 0.23",
    )
    config_path.write_text(content, encoding="utf-8")

    from vaspflow.opt.vasp_opt import VaspOptSetup

    setup = VaspOptSetup(config_file=str(config_path), struct_file="MgB2.vasp")
    setup.run(mode="rv1")

    incar_content = (Path("vasp_opt") / "INCAR").read_text(encoding="utf-8")
    assert "KSPACING = 0.23" in incar_content


def test_vasp_opt_cli(chdir_work):
    """CLI 测试：vasp_opt 命令行"""
    result = subprocess.run(
        ["vasp_opt", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--mode", "rv4"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_opt") / "INCAR_1").exists()
