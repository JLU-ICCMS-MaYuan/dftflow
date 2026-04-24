"""QE 模块测试 - API + CLI + 实际运行"""
import os
import subprocess
import sys
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
    assert "pseudo_dir = './pp'" in scf_content
    assert "Mg   24.3050 mg_pbe_v1.4.uspp" in scf_content
    assert "B    10.8100 b_pbe_v1.4.uspp.F.UPF" in scf_content


def test_qe_scf_cli(chdir_work):
    """CLI 测试：qe_scf 命令行"""
    result = subprocess.run(
        ["qe_scf", "-i", "MgB2.vasp", "-c", "input_qe.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    assert (Path("qe_scf") / "scf.in").exists()


def test_qe_scf_submit_uses_run_qe_script(chdir_work, monkeypatch):
    from qeflow.scf import qe_scf as module

    captured = {}

    def fake_handle_job_action(run_calc, submit_job, work_dir, script_name, execute_fn):
        captured["run_calc"] = run_calc
        captured["submit_job"] = submit_job
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "handle_job_action", fake_handle_job_action)
    monkeypatch.setattr(sys, "argv", ["qe_scf", "-i", "MgB2.vasp", "-c", "input_qe.toml", "-s"])

    module.main()

    assert captured["run_calc"] is False
    assert captured["submit_job"] is True
    assert captured["work_dir"] == "qe_scf"
    assert captured["script_name"] == "run_qe.sh"


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
    assert "pseudo_dir = './pp'" in nscf_content
    assert "Mg   24.3050 mg_pbe_v1.4.uspp" in nscf_content
    assert "B    10.8100 b_pbe_v1.4.uspp.F.UPF" in nscf_content


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
    eband_content = (Path("qe_eband") / "eband.in").read_text()
    assert "pseudo_dir = './pp'" in eband_content
    assert "Mg   24.3050 mg_pbe_v1.4.uspp" in eband_content
    assert "B    10.8100 b_pbe_v1.4.uspp.F.UPF" in eband_content


def test_extract_pseudo_element_token_supports_mixed_filename_prefixes():
    from qeflow.common import extract_pseudo_element_token

    assert extract_pseudo_element_token("Mg.pbe-n-kjpaw.UPF") == "Mg"
    assert extract_pseudo_element_token("b_pbe_v1.4.uspp.F.UPF") == "b"
    assert extract_pseudo_element_token("li_pbe_v1.4.uspp.F.UPF") == "li"
    assert extract_pseudo_element_token("Ag_ONCV_PBE-1.0.upf") == "Ag"
    assert extract_pseudo_element_token("Hf-sp.oncvpsp.upf") == "Hf"
    assert extract_pseudo_element_token("123-invalid.upf") == ""


def test_resolve_pseudo_files_matches_case_insensitive_prefixes(tmp_path):
    from qeflow.common import resolve_pseudo_files

    pp_dir = tmp_path / "pp_auto"
    pp_dir.mkdir()
    for filename in [
        "Mg.pbe-n-kjpaw.UPF",
        "b_pbe_v1.4.uspp.F.UPF",
        "li_pbe_v1.4.uspp.F.UPF",
        "Hf-sp.oncvpsp.upf",
    ]:
        (pp_dir / filename).write_text("", encoding="utf-8")

    resolved = resolve_pseudo_files(str(pp_dir), ["Mg", "B", "Li", "Hf"])

    assert resolved == {
        "Mg": "Mg.pbe-n-kjpaw.UPF",
        "B": "b_pbe_v1.4.uspp.F.UPF",
        "Li": "li_pbe_v1.4.uspp.F.UPF",
        "Hf": "Hf-sp.oncvpsp.upf",
    }


def test_qe_scf_requires_pp_dir_for_auto_pseudo_matching(chdir_work):
    from qeflow.scf.qe_scf import QESetup

    config_path = Path("missing_pp_dir.toml")
    config_path.write_text(
        """
[qe]
executable_path = "pw.x"

[qe_scf.CONTROL]
outdir = "'./tmp'"
""".strip()
        + "\n",
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="缺少 pp_dir 配置"):
        QESetup(config_file=str(config_path), struct_file="MgB2.vasp").setup()


def test_qe_scf_fails_when_element_pseudo_is_missing(chdir_work):
    from qeflow.scf.qe_scf import QESetup

    missing_dir = Path("pp_missing")
    missing_dir.mkdir()
    (missing_dir / "mg_pbe_v1.4.uspp").write_text("", encoding="utf-8")

    config_path = Path("missing_pseudo.toml")
    config_path.write_text(
        f"""
pp_dir = "{missing_dir}"

[qe]
executable_path = "pw.x"

[qe_scf.CONTROL]
outdir = "'./tmp'"
""".strip()
        + "\n",
        encoding="utf-8",
    )

    with pytest.raises(FileNotFoundError, match="元素 B"):
        QESetup(config_file=str(config_path), struct_file="MgB2.vasp").setup()


def test_qe_scf_fails_when_element_has_multiple_pseudos(chdir_work):
    from qeflow.scf.qe_scf import QESetup

    duplicate_dir = Path("pp_duplicate")
    duplicate_dir.mkdir()
    (duplicate_dir / "mg_pbe_v1.4.uspp").write_text("", encoding="utf-8")
    (duplicate_dir / "Mg.UPF").write_text("", encoding="utf-8")
    (duplicate_dir / "b_pbe_v1.4.uspp.F.UPF").write_text("", encoding="utf-8")

    config_path = Path("duplicate_pseudo.toml")
    config_path.write_text(
        f"""
pp_dir = "{duplicate_dir}"

[qe]
executable_path = "pw.x"

[qe_scf.CONTROL]
outdir = "'./tmp'"
""".strip()
        + "\n",
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="多个赝势文件"):
        QESetup(config_file=str(config_path), struct_file="MgB2.vasp").setup()


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
    assert "pseudo_dir" not in (Path("qe_eledos") / "eletdos.in").read_text()
    assert "pseudo_dir" not in (Path("qe_eledos") / "elepdos.in").read_text()


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
    # DOS 计算生成 eletdos.out 和 elepdos.in
    assert (Path("qe_eledos") / "eletdos.out").exists()
    assert (Path("qe_eledos") / "elepdos.in").exists()
