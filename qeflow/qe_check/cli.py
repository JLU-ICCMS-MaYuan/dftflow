#!/usr/bin/env python3
import argparse
import os
import sys
from typing import Any, Dict, Tuple

from qeflow.qe_check.checks import check_ph, check_q2r, check_relax, check_scf
from qeflow.qe_input.phonon_input import resolve_prefix
from qeflow.qe_input.fileio import resolve_workspace
from qeflow.qe_input.structure import parse_poscar


def _resolve_target_directory(config_file: str) -> Tuple[Dict[str, Any], str, str]:
    config, root_dir, workspace = resolve_workspace(config_file, None, prepare_inputs=False)
    return config, root_dir, workspace


def main() -> None:
    parser = argparse.ArgumentParser(description="QE 输出状态检查器")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-scf", action="store_true", help="检查 scf")
    parser.add_argument("-relax", action="store_true", help="检查 relax/vc-relax")
    parser.add_argument("-ph", action="store_true", help="检查声子阶段")
    parser.add_argument("-q2r", action="store_true", help="检查 q2r")
    parser.add_argument("-matdyn-band", action="store_true", help="预留接口，暂未支持")
    parser.add_argument("-matdyn-dos", action="store_true", help="预留接口，暂未支持")
    parser.add_argument("-eph", action="store_true", help="预留接口，暂未支持")
    args = parser.parse_args()

    selected = [
        args.scf,
        args.relax,
        args.ph,
        args.q2r,
        args.matdyn_band,
        args.matdyn_dos,
        args.eph,
    ]
    if sum(bool(item) for item in selected) != 1:
        parser.error("必须且只能指定一个检查开关。")

    if args.eph or args.matdyn_band or args.matdyn_dos:
        print("暂未支持")
        return

    config, root_dir, workspace = _resolve_target_directory(args.config)

    if args.scf:
        target = workspace if os.path.isdir(workspace) else root_dir
        print(check_scf(target))
        return

    if args.relax:
        print(check_relax(root_dir))
        return

    prefix = config["qe_phonon"].get("prefix")
    if not prefix:
        poscar_path = os.path.join(workspace if os.path.isdir(workspace) else root_dir, "POSCAR")
        if os.path.exists(poscar_path):
            prefix = resolve_prefix(config, parse_poscar(poscar_path))
        else:
            prefix = "qe"

    if args.ph:
        print(check_ph(workspace, str(prefix))["state"])
        return

    if args.q2r:
        print(check_q2r(workspace, str(prefix)))
        return

    sys.exit(1)
