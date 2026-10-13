"""Wannier90 模块测试 - API + CLI + 实际运行"""
import os
import subprocess
import sys
import pytest
from pathlib import Path


# ============================================================================
# wannier_init 测试
# ============================================================================

def test_wannier_init_api(chdir_work):
    """API 测试：wannier_init 文件生成"""
    # wannier_init 需要直接调用脚本，因为它不是类结构
    result = subprocess.run(
        ["python3", "-m", "wannier90flow.wannier_init",
         "--input", "MgB2.vasp", "--config", "input_w90.toml"],
        capture_output=True,
        text=True
    )

    # 检查是否生成了 .win 文件
    win_files = list(Path("wannier90").glob("*.win"))
    assert len(win_files) > 0, "应该生成 .win 文件"

    # 验证 .win 文件内容
    win_content = win_files[0].read_text()
    assert "unit_cell_cart" in win_content
    assert "atoms_frac" in win_content or "atoms_cart" in win_content
    assert "projections" in win_content
    assert "mp_grid" in win_content


def test_wannier_init_cli(chdir_work):
    """CLI 测试：wannier_init 命令行"""
    result = subprocess.run(
        ["wannier_init", "--input", "MgB2.vasp", "--config", "input_w90.toml"],
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    win_files = list(Path("wannier90").glob("*.win"))
    assert len(win_files) > 0


def test_wannier_init_submit_uses_run_wannier90_script(chdir_work, monkeypatch):
    from wannier90flow import wannier_init as module

    captured = {}

    def fake_submit_script(work_dir, script_name):
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    monkeypatch.setattr(sys, "argv", ["wannier_init", "--input", "MgB2.vasp", "--config", "input_w90.toml", "-s"])

    module.main()

    assert captured["work_dir"] == "wannier90"
    assert captured["script_name"] == "run_wannier90.sh"


@pytest.mark.run
def test_wannier_init_run(chdir_shared):
    """实际运行测试：wannier_init（生成 .win 文件）"""
    result = subprocess.run(
        ["wannier_init", "--input", "MgB2.vasp", "--config", "input_w90.toml"],
        capture_output=True,
        text=True,
        timeout=60
    )

    assert result.returncode == 0
    win_files = list(Path("wannier90").glob("*.win"))
    assert len(win_files) > 0

    # 验证生成的 .win 文件可以被 wannier90 读取（不实际运行 wannier90）
    win_content = win_files[0].read_text()
    assert "begin unit_cell_cart" in win_content
    assert "end unit_cell_cart" in win_content


def test_wannier_post_submit_uses_run_postw90_script(chdir_work, monkeypatch):
    from wannier90flow import wannier_post as module

    win_path = Path("wannier90") / "MgB2.win"
    win_path.parent.mkdir(exist_ok=True)
    win_path.write_text("num_wann = 8\n", encoding="utf-8")

    captured = {}

    def fake_submit_script(work_dir, script_name):
        captured["work_dir"] = work_dir
        captured["script_name"] = script_name

    monkeypatch.setattr(module, "submit_script", fake_submit_script)
    monkeypatch.setattr(sys, "argv", ["wannier_post", "-i", str(win_path), "-c", "input_w90.toml", "-s"])

    module.main()

    assert captured["work_dir"] == str(win_path.parent.resolve())
    assert captured["script_name"] == "run_postw90.sh"
