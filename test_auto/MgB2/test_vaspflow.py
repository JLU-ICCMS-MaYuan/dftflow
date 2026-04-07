"""VASP 模块测试 - API + CLI + 实际运行"""
import os
import subprocess
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
# vasp_cohp 测试
# ============================================================================

def test_vasp_cohp_api(chdir_work):
    """API 测试：vasp_cohp 文件生成"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    from vaspflow.cohp.vasp_cohp import VaspCohpSetup

    setup = VaspCohpSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(run_calc=False)

    assert (Path("vasp_cohp") / "INCAR").exists()
    incar_content = (Path("vasp_cohp") / "INCAR").read_text()
    assert "ISYM" in incar_content


def test_vasp_cohp_cli(chdir_work):
    """CLI 测试：vasp_cohp 命令行"""
    os.makedirs("vasp_scf", exist_ok=True)
    Path("vasp_scf/CHGCAR").touch()
    Path("vasp_scf/WAVECAR").touch()

    result = subprocess.run(
        ["vasp_cohp", "-i", "MgB2.vasp", "-c", "input_vasp.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_cohp") / "INCAR").exists()


@pytest.mark.run
def test_vasp_cohp_run(chdir_shared):
    """实际运行测试：vasp_cohp --run"""
    assert (Path("vasp_scf") / "CHGCAR").exists()

    result = subprocess.run(
        ["vasp_cohp", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("vasp_cohp") / "OUTCAR").exists()


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

    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rv4")

    assert (Path("vasp_opt") / "INCAR_1").exists()
    assert (Path("vasp_opt") / "INCAR_2").exists()
    assert (Path("vasp_opt") / "INCAR_3").exists()
    assert (Path("vasp_opt") / "INCAR_4").exists()
    assert (Path("vasp_opt") / "fouropt.sh").exists()


def test_vasp_opt_rv1_api(chdir_work):
    """API 测试：vasp_opt rv1 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rv1")

    assert (Path("vasp_opt") / "INCAR").exists()
    assert (Path("vasp_opt") / "oneopt.sh").exists()


def test_vasp_opt_rvf_api(chdir_work):
    """API 测试：vasp_opt rvf 模式"""
    from vaspflow.opt.vasp_opt import VaspOptSetup

    setup = VaspOptSetup(config_file="input_vasp.toml", struct_file="MgB2.vasp")
    setup.run(mode="rvf")

    assert (Path("vasp_opt") / "INCAR").exists()
    assert (Path("vasp_opt") / "fopt.sh").exists()


def test_vasp_opt_cli(chdir_work):
    """CLI 测试：vasp_opt 命令行"""
    result = subprocess.run(
        ["vasp_opt", "-i", "MgB2.vasp", "-c", "input_vasp.toml", "--mode", "rv4"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("vasp_opt") / "INCAR_1").exists()
