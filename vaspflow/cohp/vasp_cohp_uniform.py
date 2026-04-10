#!/usr/bin/env python3
import argparse
import os

from public.job import add_run_submit_arguments, handle_job_action
from vaspflow.cohp.cohp_common import BaseVaspCohpSetup, run_vaspkit
from vaspflow.templates import (
    get_vasp_cohp_uniform_allowed_keys,
    get_vasp_cohp_uniform_parameters_template,
)


class VaspCohpUniformSetup(BaseVaspCohpSetup):
    block_name = "vasp_cohp_uniform"
    work_dir = "vasp_cohp_uniform"

    def __init__(self, config_file="input.toml", struct_file=None):
        super().__init__(
            config_file,
            struct_file,
            get_vasp_cohp_uniform_parameters_template(),
            get_vasp_cohp_uniform_allowed_keys(),
        )

    def generate_kpoints(self):
        kmesh = self.config[self.block_name].get("kmesh")
        if not kmesh:
            print("未在配置文件中找到 'kmesh'，跳过生成 KPOINTS。")
            return

        print(f"检测到 kmesh = {kmesh}，正在使用 vaspkit 生成均匀网格 KPOINTS...")
        stdout, _ = run_vaspkit(self.work_dir, f"102\n2\n{kmesh}\n")
        if not os.path.exists(os.path.join(self.work_dir, "KPOINTS")):
            raise RuntimeError(f"vaspkit 未生成 KPOINTS。输出: {stdout}")
        print("KPOINTS 已通过 vaspkit 生成完成。")


def main():
    parser = argparse.ArgumentParser(description="VASP COHP Uniform Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_vasp.sh")
    args = parser.parse_args()

    setup = VaspCohpUniformSetup(config_file=args.config, struct_file=args.input)
    setup.setup()
    handle_job_action(args.run, args.submit, setup.work_dir, "run_vasp.sh", setup.execute)


if __name__ == "__main__":
    main()
