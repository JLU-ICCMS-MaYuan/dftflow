#!/usr/bin/env python3
"""检查真实 EPW 5.9 冒烟产物；不替代插值质量或超导数值验收。"""
import argparse
import json
import math
import re
import xml.etree.ElementTree as ET
from pathlib import Path


def verify_electronic_mesh(work: Path) -> dict:
    root = ET.parse(work / "tmp/MgB2.save/data-file-schema.xml").getroot()
    band = root.find("./output/band_structure")
    entries = band.findall("ks_energies")
    if int(band.findtext("nks")) != 32 or len(entries) != 32 or int(band.findtext("nbnd")) != 14:
        raise ValueError("NSCF 不满足 4×4×2、14 带的测试输入")
    basis = root.find("./output/basis_set/reciprocal_lattice")
    vectors = [[float(v) for v in basis.findtext(f"b{i}").split()] for i in (1, 2, 3)]

    def cross(a, b):
        return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]

    def dot(a, b):
        return sum(x*y for x, y in zip(a, b))

    dual = [cross(vectors[1], vectors[2]), cross(vectors[2], vectors[0]), cross(vectors[0], vectors[1])]
    determinant = dot(vectors[0], dual[0])
    if abs(determinant) < 1e-12:
        raise ValueError("倒易晶格不可逆")
    points, weights = set(), []
    for entry in entries:
        point = entry.find("k_point")
        cart = [float(v) for v in point.text.split()]
        fractional = [dot(cart, v) / determinant for v in dual]
        scaled = [v*n for v, n in zip(fractional, (4, 4, 2))]
        if any(not math.isfinite(v) or abs(v-round(v)) > 1e-8 for v in scaled):
            raise ValueError("输出 k 点不属于声明的均匀网格")
        points.add(tuple(round(v) % n for v, n in zip(scaled, (4, 4, 2))))
        weights.append(float(point.attrib["weight"]))
        energies = [float(v) for v in entry.findtext("eigenvalues").split()]
        if len(energies) != 14 or any(not math.isfinite(v) for v in energies):
            raise ValueError("NSCF 本征值无效")
    if len(points) != 32 or any(not math.isfinite(w) or w <= 0 for w in weights):
        raise ValueError("NSCF 网格有缺点、重复点或错误权重")
    if any(abs(w/sum(weights)-1/32) > 1e-10 for w in weights):
        raise ValueError("NSCF 归一化权重不均匀")
    return {"k_grid": [4, 4, 2], "nks": 32, "nbnd": 14,
            "nsym": int(root.findtext("./output/symmetries/nsym"))}


def verify(work: Path) -> dict:
    if (work / "epw.exitcode").read_text().strip() != "0":
        raise ValueError("EPW 进程未成功退出")
    electronic_mesh = verify_electronic_mesh(work)
    log = (work / "epw.out").read_text()
    if not re.search(r"Program EPW v\.5\.9\s", log):
        raise ValueError("缺少目标 EPW 5.9 运行证据")
    if re.search(r"error in routine|segmentation fault|MPI_Abort", log, re.I):
        raise ValueError("EPW 日志包含运行错误")
    # EPW 5.9 没有 PW/PH 的 JOB DONE 标志。结束计时也不能替代退出码。
    if "Total program execution" not in log or not re.search(r"EPW\s*:\s*.*WALL", log):
        raise ValueError("EPW 日志没有完整结束计时")
    wout = (work / "MgB2.wout").read_text()
    if re.search(r"convergence\s+(?:was\s+)?not\s+(?:reached|achieved)|not\s+converged", wout, re.I):
        raise ValueError("Wannier 日志明确报告未收敛")
    for phase in ("Disentanglement", "Wannierisation"):
        if f"<<< {phase} convergence criteria satisfied >>>" not in wout:
            raise ValueError(f"{phase} 尚无明确收敛证据")
    spectra = sorted(work.glob("MgB2.a2f.*"))
    if len(spectra) != 1:
        raise ValueError("预期恰好一份原生 α²F 输出")
    text = spectra[0].read_text()
    numeric, footer = text.split("Integrated el-ph coupling", 1)
    rows = [[float(x) for x in line.split()] for line in numeric.splitlines() if line.strip()]
    if len(rows) != 500 or any(len(row) != 11 for row in rows):
        raise ValueError("谱表应有 500 点及十列声子展宽")
    if any(not math.isfinite(v) for row in rows for v in row):
        raise ValueError("谱表包含非有限值")
    if any(row[0] <= 0 or any(v < 0 for v in row[1:]) for row in rows):
        raise ValueError("谱表存在非法能量或负谱权重")
    if any(a[0] >= b[0] for a, b in zip(rows, rows[1:])):
        raise ValueError("谱能量未严格递增")
    if not any(row[1] > 0 for row in rows):
        raise ValueError("MgB₂ 正向冒烟测试没有非零谱权重")
    integrated = [float(x) for x in footer.splitlines()[1].split()[1:]]
    if len(integrated) != 10 or any(not math.isfinite(v) for v in integrated):
        raise ValueError("积分耦合常数列不完整")
    # 原生表保留七位小数；此检查只校验导出谱与其积分，不比较材料参考。
    recovered = []
    for column in range(1, 11):
        value = sum((b[0] - a[0]) * (a[column] / a[0] + b[column] / b[0])
                    for a, b in zip(rows, rows[1:]))
        if abs(value - integrated[column - 1]) > 1e-6 + 1e-6 * abs(integrated[column - 1]):
            raise ValueError(f"第 {column} 列谱积分与原生累计 λ 不一致")
        recovered.append(value)
    summed = float(re.search(r"Summed el-ph coupling\s+(\S+)", footer)[1])
    if not math.isfinite(summed):
        raise ValueError("直接求和 λ 非有限")
    return {
        "schema_version": 1, "native_epc_smoke_passed": True,
        "interpolation_quality_validated": False, "eliashberg_validated": False,
        "epwflow_automatic_chain_validated": False,
        "electronic_mesh": electronic_mesh,
        "spectrum": spectra[0].name, "energy_unit": "meV", "samples": len(rows),
        "lambda_integral": integrated, "lambda_from_export": recovered,
        "lambda_sum": summed,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("work", type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.work.resolve()), ensure_ascii=False, indent=2))
    except (ValueError, OSError, IndexError, TypeError, AttributeError, ET.ParseError) as error:
        parser.exit(1, f"原生 EPW 冒烟检查失败：{error}\n")
