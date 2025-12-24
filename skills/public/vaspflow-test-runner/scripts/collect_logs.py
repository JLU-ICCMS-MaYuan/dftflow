#!/usr/bin/env python3
"""
汇总 vaspflow 输出日志为单个文本，便于诊断。
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
from typing import Iterable


CANDIDATE_NAMES = {
    "pipeline.log",
    "pipeline_report.txt",
    "pipeline_checkpoint.json",
    "stderr",
    "stdout",
    "vasp.out",
    "vasp.log",
}


def iter_candidate_files(root: Path) -> Iterable[Path]:
    for dirpath, _, filenames in os.walk(root):
        for name in filenames:
            if name in CANDIDATE_NAMES:
                yield Path(dirpath) / name


def read_tail(path: Path, max_bytes: int) -> str:
    data = path.read_bytes()
    if len(data) > max_bytes:
        data = data[-max_bytes:]
        prefix = f"...(截断，保留末尾 {max_bytes} 字节)\n"
    else:
        prefix = ""
    try:
        text = data.decode("utf-8", errors="replace")
    except Exception:
        text = data.decode("latin-1", errors="replace")
    return prefix + text


def main() -> None:
    parser = argparse.ArgumentParser(description="汇总 vaspflow 日志到单一文本文件")
    parser.add_argument("--workdir", required=True, help="工作目录（含压强子目录）")
    parser.add_argument("--output", default="collected_logs.txt", help="输出文件路径")
    parser.add_argument(
        "--max-bytes",
        type=int,
        default=20000,
        help="单个文件的最大读取字节数，默认 20000（读取尾部）",
    )
    args = parser.parse_args()

    root = Path(args.workdir).expanduser().resolve()
    if not root.exists():
        raise SystemExit(f"工作目录不存在: {root}")

    files = sorted(set(iter_candidate_files(root)))
    if not files:
        raise SystemExit("未找到可汇总的日志文件")

    output_path = Path(args.output).expanduser().resolve()
    parts: list[str] = []
    parts.append(f"# vaspflow 日志汇总\n源目录: {root}\n")
    for fp in files:
        parts.append(f"\n==== {fp.relative_to(root)} ====\n")
        parts.append(read_tail(fp, args.max_bytes))

    output_path.write_text("\n".join(parts), encoding="utf-8")
    print(f"已写入 {output_path}（文件数: {len(files)}）")


if __name__ == "__main__":
    main()
