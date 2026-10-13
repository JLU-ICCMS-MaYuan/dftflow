"""pytest fixtures for MgB2 tests."""
import os
import shutil
import sys
import pytest
from pathlib import Path

# 占位符注入：仓库内 input_*.toml 使用 @NAME@ 占位符（不含机器真实路径），
# 拷贝到工作目录时从 gitignored 的 docs/local/paths.toml 读真实路径替换。
sys.path.insert(0, str(Path(__file__).parent))
from _local_paths import apply_placeholders, load_local_paths  # noqa: E402

_INPUT_TOMLS = ("input_vasp.toml", "input_qe.toml", "input_w90.toml")


def _copy_inputs(target_dir: Path) -> None:
    """拷贝三个输入配置并注入本机真实路径。"""
    for name in _INPUT_TOMLS:
        shutil.copy2(Path(__file__).parent / name, target_dir / name)
        apply_placeholders(target_dir / name)
    if load_local_paths():
        for name in _INPUT_TOMLS:
            text = (target_dir / name).read_text(encoding="utf-8")
            if "@" in text and any(token in text for token in ("@QE_", "@VASP_", "@LOBSTER_", "@WANNIER90_", "@PW2WANNIER90_", "@POSTW90_")):
                pytest.exit(
                    f"[conftest] {name} 仍含未替换的机器路径占位符，请检查 docs/local/paths.toml",
                    returncode=4,
                )


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

    # 拷贝配置文件（含占位符注入）
    _copy_inputs(work_dir)

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

    # 拷贝配置文件（含占位符注入）
    _copy_inputs(tmp_path)

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
