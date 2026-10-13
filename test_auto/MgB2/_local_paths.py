"""读取 docs/local/paths.toml 并替换输入文件中的 @NAME@ 机器路径占位符。

仓库内 test_auto/MgB2/input_*.toml、local_run/*.toml、local_run/*.sh 与
qeflow 示例使用 @NAME@ 占位符，不含任何机器真实路径；真实路径只存在于
gitignored 的 docs/local/paths.toml（键位于 [placeholders] 表）。

- pytest：conftest.py 拷贝输入文件到工作目录后调用 apply_placeholders 注入。
- 手动计算：直接运行本模块把占位符就地写回目标文件（替换后的文件勿提交）。
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
LOCAL_PATHS_FILE = REPO_ROOT / "docs" / "local" / "paths.toml"


def load_local_paths() -> dict[str, str]:
    """读取本地真实路径表；文件缺失时返回空表并提示。"""
    if not LOCAL_PATHS_FILE.exists():
        print(
            f"[local_paths] 未找到 {LOCAL_PATHS_FILE}，跳过占位符替换；"
            "请参考 docs/manual-testing.md 创建该文件。",
            file=sys.stderr,
        )
        return {}
    with LOCAL_PATHS_FILE.open("rb") as fh:
        data = tomllib.load(fh)
    return {str(key): str(value) for key, value in data.get("placeholders", {}).items()}


def apply_placeholders(target: Path, mapping: dict[str, str] | None = None) -> None:
    """将 target 文本文件中的 @NAME@ 占位符替换为本地真实路径（就地写回）。"""
    if mapping is None:
        mapping = load_local_paths()
    if not mapping:
        return
    text = target.read_text(encoding="utf-8")
    for key, value in mapping.items():
        token = f"@{key}@"
        if token in text:
            text = text.replace(token, value)
    target.write_text(text, encoding="utf-8")


def _main(argv: list[str]) -> int:
    mapping = load_local_paths()
    if not mapping:
        print("[local_paths] 缺少 docs/local/paths.toml，无法替换占位符", file=sys.stderr)
        return 1
    targets = [Path(arg) for arg in argv] or [REPO_ROOT / "test_auto" / "MgB2"]
    changed = 0
    for target in targets:
        files = (
            [target]
            if target.is_file()
            else sorted(p for p in target.rglob("*") if p.suffix in {".toml", ".sh"} and p.is_file())
        )
        for file_path in files:
            before = file_path.read_text(encoding="utf-8")
            apply_placeholders(file_path, mapping)
            if file_path.read_text(encoding="utf-8") != before:
                changed += 1
                print(f"[local_paths] 已替换: {file_path}")
    print(f"[local_paths] 完成，共替换 {changed} 个文件")
    return 0


if __name__ == "__main__":
    raise SystemExit(_main(sys.argv[1:]))
