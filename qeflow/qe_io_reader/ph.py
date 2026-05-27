#!/usr/bin/env python3
import os
from dataclasses import dataclass
from typing import Dict, List

from qeflow.qe_io_reader.common import has_error_marker, read_text_if_exists


@dataclass
class PhOut:
    path: str

    def exists(self) -> bool:
        return os.path.exists(self.path)

    def text(self) -> str:
        return read_text_if_exists(self.path)

    def is_finished(self) -> bool:
        return "JOB DONE" in self.text()

    def has_error(self) -> bool:
        text = self.text()
        return bool(text) and has_error_marker(text)


def parse_dyn0_qpoints(dyn0_path: str) -> List[Dict[str, object]]:
    if not os.path.exists(dyn0_path):
        raise FileNotFoundError(f"找不到 dyn0 文件: {dyn0_path}")
    with open(dyn0_path, "r", encoding="utf-8") as handle:
        lines = [line.strip() for line in handle if line.strip()]
    if len(lines) < 2:
        raise ValueError(f"dyn0 文件格式不完整: {dyn0_path}")
    counts = lines[0].split()
    if len(counts) != 3:
        raise ValueError(f"dyn0 第一行无法解析 nq1/nq2/nq3: {lines[0]}")
    try:
        q_count = int(lines[1].split()[0])
    except (IndexError, ValueError) as exc:
        raise ValueError(f"dyn0 第二行无法解析 q 点数量: {lines[1]}") from exc
    if len(lines) < 2 + q_count:
        raise ValueError(f"dyn0 文件中的 q 点数量不足: 期望 {q_count}，实际 {len(lines) - 2}")

    qpoints: List[Dict[str, object]] = []
    for index in range(q_count):
        raw_line = lines[2 + index]
        parts = raw_line.split()
        if len(parts) != 3:
            raise ValueError(f"dyn0 q 点行无法解析为 3 个分量: {raw_line}")
        qpoints.append(
            {
                "index": index + 1,
                "q_crystal": [float(parts[0]), float(parts[1]), float(parts[2])],
            }
        )
    return qpoints
