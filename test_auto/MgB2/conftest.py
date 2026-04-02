"""pytest fixtures for MgB2 tests."""
import os
import shutil
import pytest
from pathlib import Path


def pytest_configure(config):
    """注册自定义 marker"""
    config.addinivalue_line(
        "markers", "run: 标记需要实际运行计算的测试（耗时较长）"
    )


@pytest.fixture(scope="session")
def base_dir():
    """返回 test_auto/MgB2/ 的绝对路径"""
    return Path(__file__).parent.absolute()


@pytest.fixture(scope="session")
def shared_work_dir(tmp_path_factory, base_dir):
    """
    创建一个会话级别的共享工作目录，所有 -r 测试在此执行。
    这样 vasp_scf 的输出可以被后续模块复用。
    """
    work_dir = tmp_path_factory.mktemp("mgb2_shared")

    # 拷贝结构文件
    shutil.copy2(base_dir / "MgB2.vasp", work_dir / "MgB2.vasp")

    # 拷贝 POTCAR 库
    shutil.copytree(base_dir / "potcar_lib", work_dir / "potcar_lib")

    # 拷贝 QE 赝势
    shutil.copytree(base_dir / "pp", work_dir / "pp")

    # 拷贝配置文件
    shutil.copy2(base_dir / "input_vasp.toml", work_dir / "input_vasp.toml")
    shutil.copy2(base_dir / "input_qe.toml", work_dir / "input_qe.toml")
    shutil.copy2(base_dir / "input_w90.toml", work_dir / "input_w90.toml")

    return work_dir


@pytest.fixture
def work_dir(tmp_path, base_dir):
    """
    为每个测试创建独立的临时工作目录（用于 API 和 CLI 非运行测试）
    """
    # 拷贝结构文件
    shutil.copy2(base_dir / "MgB2.vasp", tmp_path / "MgB2.vasp")

    # 拷贝 POTCAR 库
    shutil.copytree(base_dir / "potcar_lib", tmp_path / "potcar_lib")

    # 拷贝 QE 赝势
    shutil.copytree(base_dir / "pp", tmp_path / "pp")

    # 拷贝配置文件
    shutil.copy2(base_dir / "input_vasp.toml", tmp_path / "input_vasp.toml")
    shutil.copy2(base_dir / "input_qe.toml", tmp_path / "input_qe.toml")
    shutil.copy2(base_dir / "input_w90.toml", tmp_path / "input_w90.toml")

    return tmp_path


@pytest.fixture
def chdir_work(work_dir):
    """切换到工作目录，测试结束后恢复"""
    original_dir = os.getcwd()
    os.chdir(work_dir)
    yield work_dir
    os.chdir(original_dir)


@pytest.fixture
def chdir_shared(shared_work_dir):
    """切换到共享工作目录，测试结束后恢复"""
    original_dir = os.getcwd()
    os.chdir(shared_work_dir)
    yield shared_work_dir
    os.chdir(original_dir)
