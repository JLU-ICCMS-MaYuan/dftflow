#!/usr/bin/env python3
"""差分核对固定子仓库的实际选窗函数；仅测试，不安装或启动 AiiDA。"""
import argparse
import ast
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import typing

import numpy as np


class AttributeDict(dict):
    def __getattr__(self, name):
        return self[name]

    def __setattr__(self, name, value):
        self[name] = value


class Dict:
    def __init__(self, value=None):
        self.value = copy.deepcopy(value or {})

    def get_dict(self):
        return copy.deepcopy(self.value)


class Bands:
    def __init__(self, value):
        self.value = np.array(value)

    def get_bands(self):
        return self.value


def load_functions(path, names, namespace):
    tree = ast.parse(path.read_text())
    nodes = [n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name in names]
    if len(nodes) != len(names):
        raise ValueError(f"上游函数集合变化: {path}")
    for node in nodes:
        node.decorator_list = []
    # 直接执行来源函数 AST，不在测试里重写平移或上限算法。
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(path), "exec"), namespace)


def upstream_prepare(repo):
    root = repo / "external/aiida-wannier90-workflows"
    revision = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    if revision != "1c0115cb7c77541a0bd549bc05005df435d651b6":
        raise ValueError("子仓库版本不匹配，需先审查策略变化")
    src = root / "src/aiida_wannier90_workflows"
    bands_module = types.ModuleType("aiida_wannier90_workflows.utils.bands")
    ns = {"np": np, "ty": typing}
    load_functions(src / "utils/bands/__init__.py", {"get_homo_lumo", "remove_exclude_bands"}, ns)
    bands_module.get_homo_lumo = ns["get_homo_lumo"]
    bands_module.remove_exclude_bands = ns["remove_exclude_bands"]
    sys.modules[bands_module.__name__] = bands_module
    scdm = types.ModuleType("aiida_wannier90_workflows.utils.scdm")

    def unused(*_args, **_kwargs):
        raise AssertionError("默认组合模式不得调用 ENERGY_AUTO 的投影估能路径")

    scdm.get_energy_of_projectability = unused
    sys.modules[scdm.__name__] = scdm
    ns = {"AttributeDict": AttributeDict, "orm": types.SimpleNamespace(Dict=Dict)}
    load_functions(src / "workflows/base/wannier90.py", {"prepare_inputs"}, ns)
    return ns["prepare_inputs"], revision


def compare(binary, prepare, bands, fermi, num_wann, excluded, temp):
    xml = temp / "bands.xml"
    states = "".join("<ks_energies><eigenvalues>" + " ".join(format(e / 27.211386245988, ".17e") for e in row)
                     + "</eigenvalues></ks_energies>" for row in bands)
    xml.write_text(f"<espresso><output><band_structure><nbnd>{len(bands[0])}</nbnd><nks>{len(bands)}</nks>"
                   f"<fermi_energy>{fermi / 27.211386245988:.17e}</fermi_energy>{states}</band_structure></output></espresso>")
    args = [str(binary), "window", str(xml), "--num-wann", str(num_wann)]
    if excluded:
        args += ["--exclude-bands", ",".join(map(str, excluded))]
    result = subprocess.run(args, check=True, capture_output=True, text=True)
    actual = json.loads(result.stdout)
    # 按 XML 实际转换后的精度传递同一组数据，避免比较两个不同的输入。
    parsed = [[float(format(e / 27.211386245988, ".17e")) * 27.211386245988 for e in row] for row in bands]
    ef = float(format(fermi / 27.211386245988, ".17e")) * 27.211386245988
    parameters = {"num_wann": num_wann, "fermi_energy": ef, "dis_froz_max": 2.0,
                  "dis_proj_min": 0.01, "dis_proj_max": 0.95}
    if excluded:
        parameters["exclude_bands"] = excluded
    obj = types.SimpleNamespace()
    obj._inputs_namespace = "wannier90"
    obj.inputs = AttributeDict(shift_energy_windows=True, auto_energy_windows=False,
                              bands=Bands(parsed), wannier90=AttributeDict())
    obj.exposed_inputs = lambda *_: AttributeDict(parameters=Dict(parameters))
    prepare.__globals__["Wannier90Calculation"] = object
    expected = prepare(obj).parameters.get_dict()
    for key in ["dis_froz_max", "dis_proj_min", "dis_proj_max"]:
        if not np.isclose(actual["parameters"][key], expected[key], rtol=0, atol=1e-10):
            raise AssertionError((key, actual["parameters"][key], expected[key]))
    for key in ["dis_win_min", "dis_win_max", "dis_froz_min"]:
        if key in actual["parameters"] or key in expected:
            raise AssertionError(f"不得补造未设置边界: {key}")
    return actual


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--qe-xml", type=Path)
    parser.add_argument("--num-wann", type=int, default=5)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[3]
    prepare, revision = upstream_prepare(repo)
    cases = [
        ([[ -0.0002, 0.0002, 4., 6.], [-0.0001, 0.0003, 4.5, 7.]], 0., 3, []),
        ([[-2., -1., 2., 2.5], [-2., -0.8, 3., 6.]], 0., 3, []),
        ([[-2., -1., 2., 2.5], [-2., -0.8, 3., 6.]], 0., 2, []),
        ([[-10., -1., 1., 8.], [-9., -0.5, 2., 7.]], 0., 2, [1]),
    ]
    # 固定种子覆盖不同维数、排除带及离散采样间隙；不进行科学参数扫描。
    rng = np.random.default_rng(16)
    for _ in range(32):
        bands = np.sort(rng.uniform(-5., 12., size=(8, 10)), axis=1)
        bands[:, 0] = -6.
        bands[:, -1] = 13.
        cases.append((bands.tolist(), 0., int(rng.integers(2, 8)), [1] if rng.random() > 0.5 else []))
    if args.qe_xml:
        import xml.etree.ElementTree as ET
        root = ET.parse(args.qe_xml).getroot()
        band = root.find("./output/band_structure")
        values = [[float(v.replace("D", "e").replace("d", "e")) * 27.211386245988
                   for v in node.text.split()] for node in band.findall("./ks_energies/eigenvalues")]
        fermi = float(band.findtext("fermi_energy")) * 27.211386245988
        cases.append((values, fermi, args.num_wann, []))
    with tempfile.TemporaryDirectory(prefix="wannier-policy-") as tmp:
        for case in cases:
            result = compare(args.binary.resolve(), prepare, *case, Path(tmp))
    print(json.dumps({"upstream_revision": revision, "cases_passed": len(cases),
                      "last_selection": result, "scientific_programs_run": False}, ensure_ascii=False))


if __name__ == "__main__":
    main()
