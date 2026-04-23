#!/usr/bin/env python3
import argparse
import os

from public.job import add_run_submit_arguments, execute_commands, handle_job_action
from qeflow.phonon.common import (
    build_qe_commands,
    create_job_script,
    derive_prefix,
    load_phonon_config,
    parse_poscar,
    prepare_work_tree,
    write_matdyn_band_input,
    write_matdyn_dos_input,
    write_ph_input,
    write_q2r_input,
    write_scf_input,
)
from qeflow.phonon.kpath import build_matdyn_band_path, run_vaspkit_for_kpath


class QEPhononSetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.config_file = config_file
        self.struct_file = struct_file
        self.config = load_phonon_config(config_file)
        self.work_dir = self.config["qe_phonon"]["work_dir"]
        self.commands = build_qe_commands(self.config)

    def _resolve_structure(self) -> str:
        if self.struct_file:
            return self.struct_file
        if os.path.exists("POSCAR"):
            return "POSCAR"
        raise FileNotFoundError("找不到结构文件。请在当前目录准备 POSCAR 或使用 -i 指定。")

    def setup(self):
        struct_path = self._resolve_structure()
        target_poscar = prepare_work_tree(self.work_dir, struct_path, self.config_file)
        struct_info = parse_poscar(target_poscar)
        prefix = derive_prefix(self.config, struct_info)
        write_scf_input(self.work_dir, self.config, struct_info, prefix)
        write_ph_input(self.work_dir, self.config, prefix)
        write_q2r_input(self.work_dir, self.config, prefix)
        run_vaspkit_for_kpath(self.work_dir)
        kpoint_lines, _label_path = build_matdyn_band_path(
            self.work_dir,
            prefix,
            struct_info["lattice"],
            self.config["qe_phonon"]["phband"]["kpath_file"],
            int(self.config["qe_phonon"]["phband"]["kpath_points"]),
        )
        write_matdyn_band_input(self.work_dir, self.config, prefix, kpoint_lines)
        write_matdyn_dos_input(self.work_dir, self.config, prefix)
        create_job_script(
            os.path.join(self.work_dir, "run_qe.sh"),
            self.config,
            [
                f"{self.commands['pw']} < scf.in > scf.out",
                f"{self.commands['ph']} < ph.in > ph.out",
                f"{self.commands['q2r']} < q2r.in > q2r.out",
                f"{self.commands['matdyn']} < matdyn_band.in > matdyn_band.out",
                f"{self.commands['matdyn']} < matdyn_dos.in > matdyn_dos.out",
            ],
        )

    def execute(self):
        execute_commands(self.work_dir, ["bash run_qe.sh"], work_label=os.path.basename(self.work_dir), error_prefix="执行 QE phonon 出错")


def main():
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO phonon 全链路工作流")
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")
    args = parser.parse_args()
    setup_obj = QEPhononSetup(config_file=args.config, struct_file=args.input)
    setup_obj.setup()
    handle_job_action(args.run, args.submit, setup_obj.work_dir, "run_qe.sh", setup_obj.execute)


if __name__ == "__main__":
    main()
