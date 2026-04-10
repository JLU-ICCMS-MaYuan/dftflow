#!/usr/bin/env python3
import argparse
import os

from public.job import add_run_submit_arguments, handle_job_action
from vaspflow.cohp.cohp_common import BaseVaspCohpSetup, run_vaspkit
from vaspflow.templates import (
    get_vasp_cohp_fatband_allowed_keys,
    get_vasp_cohp_fatband_parameters_template,
)


class VaspCohpFatbandSetup(BaseVaspCohpSetup):
    block_name = "vasp_cohp_fatband"
    work_dir = "vasp_cohp_fatband"

    def __init__(self, config_file="input.toml", struct_file=None):
        super().__init__(
            config_file,
            struct_file,
            get_vasp_cohp_fatband_parameters_template(),
            get_vasp_cohp_fatband_allowed_keys(),
        )

    def generate_kpoints(self):
        params = self.config[self.block_name]
        scf_kmesh = params.get("scf_kmesh")
        kpath_kmesh = params.get("kpath_kmesh")
        scheme = params.get("kmesh_scheme", "monkhorst").lower()
        if scf_kmesh is None or kpath_kmesh is None:
            raise ValueError("vasp_cohp_fatband 需要设置 scf_kmesh 和 kpath_kmesh。")
        if scheme not in {"monkhorst", "gamma"}:
            raise ValueError("kmesh_scheme 仅支持 monkhorst 或 gamma。")

        print("正在使用 vaspkit 303 生成 KPATH.in...")
        stdout_303, _ = run_vaspkit(self.work_dir, "303\n")
        if not os.path.exists(os.path.join(self.work_dir, "KPATH.in")):
            raise RuntimeError(f"vaspkit 303 未生成 KPATH.in。输出: {stdout_303}")

        scheme_code = "1" if scheme == "monkhorst" else "2"
        print("正在使用 vaspkit 251 生成 fatband KPOINTS...")
        stdout_251, _ = run_vaspkit(self.work_dir, f"251\n{scheme_code}\n{scf_kmesh}\n{kpath_kmesh}\n")
        if not os.path.exists(os.path.join(self.work_dir, "KPOINTS")):
            raise RuntimeError(f"vaspkit 251 未生成 KPOINTS。输出: {stdout_251}")
        print("KPOINTS 已通过 vaspkit 303 + 251 生成完成。")


def main():
    parser = argparse.ArgumentParser(description="VASP COHP Fatband Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_vasp.sh")
    args = parser.parse_args()

    setup = VaspCohpFatbandSetup(config_file=args.config, struct_file=args.input)
    setup.setup()
    handle_job_action(args.run, args.submit, setup.work_dir, "run_vasp.sh", setup.execute)


if __name__ == "__main__":
    main()
