"""CLI 入口包装器：确保优先加载本项目的 vasp 包，避免与其他同名包冲突。"""

from importlib.machinery import PathFinder
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import sys


def _prefer_local_vasp() -> None:
    """在 sys.meta_path 前插入本地查找器，确保 vasp 解析到当前项目。"""
    project_root = Path(__file__).resolve().parent

    class _LocalVaspFinder:  # noqa: D401 - 简短说明
        @classmethod
        def find_spec(cls, fullname: str, path=None, target=None):
            if fullname == "vasp":
                init_path = project_root / "__init__.py"
                if init_path.exists():
                    return spec_from_file_location(
                        fullname,
                        init_path,
                        submodule_search_locations=[str(project_root)],
                    )
            return None

    sys.meta_path.insert(0, _LocalVaspFinder)
    if str(project_root) not in sys.path:
        sys.path.insert(0, str(project_root))


def main() -> None:
    _prefer_local_vasp()
    from vasp.cli import main as cli_main

    cli_main()
