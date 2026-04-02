"""QE 模块测试 - API + CLI + 实际运行"""
import os
import subprocess
import pytest
from pathlib import Path


# ============================================================================
# qe_scf 测试
# ============================================================================

def test_qe_scf_api(chdir_work):
    """API 测试：qe_scf 文件生成"""
    from qeflow.scf.qe_scf import QESetup

    setup = QESetup(config_file="input_qe.toml", struct_file="MgB2.vasp")
    setup.setup()

    # 验证文件生成
    assert (Path("qe_scf") / "scf.in").exists()
    assert (Path("qe_scf") / "run_qe.sh").exists()

    # 验证 scf.in 内容
    scf_content = (Path("qe_scf") / "scf.in").read_text()
    assert "CELL_PARAMETERS" in scf_content
    assert "ATOMIC_POSITIONS" in scf_content
    assert "K_POINTS" in scf_content
    assert "ecutwfc" in scf_content


def test_qe_scf_cli(chdir_work):
    """CLI 测试：qe_scf 命令行"""
    result = subprocess.run(
        ["qe_scf", "-i", "MgB2.vasp", "-c", "input_qe.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("qe_scf") / "scf.in").exists()


@pytest.mark.run
def test_qe_scf_run(chdir_shared):
    """实际运行测试：qe_scf --run"""
    result = subprocess.run(
        ["qe_scf", "-i", "MgB2.vasp", "-c", "input_qe.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("qe_scf") / "scf.out").exists() or (Path("qe_scf") / "tmp").exists()


# ============================================================================
# qe_nscf 测试
# ============================================================================

def test_qe_nscf_api(chdir_work):
    """API 测试：qe_nscf 文件生成"""
    # 创建假的 qe_scf 输出
    os.makedirs("qe_scf/tmp", exist_ok=True)
    Path("qe_scf/scf.out").touch()

    from qeflow.nscf.qe_nscf import QENSCFSetup

    setup = QENSCFSetup(config_file="input_qe.toml", struct_file="MgB2.vasp")
    setup.setup()

    assert (Path("qe_nscf") / "nscf.in").exists()
    assert (Path("qe_nscf") / "run_qe.sh").exists()

    # 验证 nscf.in 内容
    nscf_content = (Path("qe_nscf") / "nscf.in").read_text()
    assert "calculation" in nscf_content
    assert "nscf" in nscf_content


def test_qe_nscf_cli(chdir_work):
    """CLI 测试：qe_nscf 命令行"""
    os.makedirs("qe_scf/tmp", exist_ok=True)
    Path("qe_scf/scf.out").touch()

    result = subprocess.run(
        ["qe_nscf", "-i", "MgB2.vasp", "-c", "input_qe.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("qe_nscf") / "nscf.in").exists()


@pytest.mark.run
def test_qe_nscf_run(chdir_shared):
    """实际运行测试：qe_nscf --run（依赖 qe_scf）"""
    # 确保 qe_scf 已运行
    assert (Path("qe_scf") / "tmp").exists() or (Path("qe_scf") / "scf.out").exists()

    result = subprocess.run(
        ["qe_nscf", "-i", "MgB2.vasp", "-c", "input_qe.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("qe_nscf") / "nscf.out").exists() or (Path("qe_nscf") / "tmp").exists()


# ============================================================================
# qe_eband 测试
# ============================================================================

def test_qe_eband_api(chdir_work):
    """API 测试：qe_eband 文件生成"""
    # 创建假的 qe_nscf 输出
    os.makedirs("qe_nscf/tmp", exist_ok=True)
    Path("qe_nscf/nscf.out").touch()

    from qeflow.eband.qe_eband import QEEBandSetup

    setup = QEEBandSetup(config_file="input_qe.toml", struct_file="MgB2.vasp")
    setup.setup()

    assert (Path("qe_eband") / "eband.in").exists()
    assert (Path("qe_eband") / "run_qe.sh").exists()


def test_qe_eband_cli(chdir_work):
    """CLI 测试：qe_eband 命令行"""
    os.makedirs("qe_nscf/tmp", exist_ok=True)
    Path("qe_nscf/nscf.out").touch()

    result = subprocess.run(
        ["qe_eband", "-i", "MgB2.vasp", "-c", "input_qe.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("qe_eband") / "eband.in").exists()


@pytest.mark.run
def test_qe_eband_run(chdir_shared):
    """实际运行测试：qe_eband --run（依赖 qe_nscf）"""
    assert (Path("qe_nscf") / "tmp").exists() or (Path("qe_nscf") / "nscf.out").exists()

    result = subprocess.run(
        ["qe_eband", "-i", "MgB2.vasp", "-c", "input_qe.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    assert (Path("qe_eband") / "eband.out").exists() or (Path("qe_eband") / "tmp").exists()


# ============================================================================
# qe_eledos 测试
# ============================================================================

def test_qe_eledos_api(chdir_work):
    """API 测试：qe_eledos 文件生成"""
    # 创建假的 qe_scf 输出
    os.makedirs("qe_scf/tmp", exist_ok=True)
    Path("qe_scf/scf.out").touch()

    from qeflow.eledos.qe_eledos import QEEleDosSetup

    setup = QEEleDosSetup(config_file="input_qe.toml", struct_file="MgB2.vasp")
    setup.setup()

    assert (Path("qe_eledos") / "eletdos.in").exists()
    assert (Path("qe_eledos") / "elepdos.in").exists()
    assert (Path("qe_eledos") / "run_qe.sh").exists()


def test_qe_eledos_cli(chdir_work):
    """CLI 测试：qe_eledos 命令行"""
    os.makedirs("qe_scf/tmp", exist_ok=True)
    Path("qe_scf/scf.out").touch()

    result = subprocess.run(
        ["qe_eledos", "-i", "MgB2.vasp", "-c", "input_qe.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("qe_eledos") / "eletdos.in").exists()


@pytest.mark.run
def test_qe_eledos_run(chdir_shared):
    """实际运行测试：qe_eledos --run（依赖 qe_scf）"""
    assert (Path("qe_scf") / "tmp").exists() or (Path("qe_scf") / "scf.out").exists()

    result = subprocess.run(
        ["qe_eledos", "-i", "MgB2.vasp", "-c", "input_qe.toml", "--run"],
        capture_output=True,
        text=True,
        timeout=600
    )

    assert result.returncode == 0
    # DOS 计算可能生成多个输出文件
    assert (Path("qe_eledos") / "dos.out").exists() or len(list(Path("qe_eledos").glob("*.dos"))) > 0
