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
    write_python_helper_script,
    write_matdyn_band_input,
    write_matdyn_dos_input,
    write_q2r_input,
)
from qeflow.phonon.kpath import build_matdyn_band_path, run_vaspkit_for_kpath
from qeflow.phonon.merge import merge_split_results


class QEPhononMergeContinueSetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.config_file = config_file
        self.struct_file = struct_file
        self.config = load_phonon_config(config_file)
        self.work_dir = self.config["qe_phonon"]["work_dir"]
        self.commands = build_qe_commands(self.config)
        self.prefix = None

    def _resolve_structure(self) -> str:
        if self.struct_file:
            return self.struct_file
        target = os.path.join(self.work_dir, "POSCAR")
        if os.path.exists(target):
            return target
        if os.path.exists("POSCAR"):
            return "POSCAR"
        raise FileNotFoundError("找不到结构文件。请在当前目录准备 POSCAR 或使用 -i 指定。")

    def merge_only(self):
        struct_info = parse_poscar(os.path.join(self.work_dir, "POSCAR"))
        self.prefix = derive_prefix(self.config, struct_info)
        merge_split_results(self.work_dir, self.prefix, os.path.join(self.work_dir, "q_index_map.json"))

    def write_merge_helper(self) -> None:
        helper_path = os.path.join(self.work_dir, "merge_continue.py")
        write_python_helper_script(
            helper_path,
            """import os
from qeflow.phonon.qe_phonon_merge_continue import QEPhononMergeContinueSetup

os.chdir(os.path.dirname(os.path.abspath(__file__)))
setup = QEPhononMergeContinueSetup(config_file="input.toml", struct_file="POSCAR")
setup.work_dir = "."
setup.merge_only()
""",
        )

    def setup(self):
        struct_path = self._resolve_structure()
        prepare_work_tree(self.work_dir, struct_path, self.config_file)
        struct_info = parse_poscar(os.path.join(self.work_dir, "POSCAR"))
        self.prefix = derive_prefix(self.config, struct_info)
        write_q2r_input(self.work_dir, self.config, self.prefix)
        run_vaspkit_for_kpath(self.work_dir)
        kpoint_lines, _label_path = build_matdyn_band_path(
            self.work_dir,
            self.prefix,
            struct_info["lattice"],
            self.config["qe_phonon"]["phband"]["kpath_file"],
            int(self.config["qe_phonon"]["phband"]["kpath_points"]),
        )
        write_matdyn_band_input(self.work_dir, self.config, self.prefix, kpoint_lines)
        write_matdyn_dos_input(self.work_dir, self.config, self.prefix)
        self.write_merge_helper()
        create_job_script(
            os.path.join(self.work_dir, "run_qe.sh"),
            self.config,
            [
                "python merge_continue.py",
                f"{self.commands['q2r']} < q2r.in > q2r.out",
                f"{self.commands['matdyn']} < matdyn_band.in > matdyn_band.out",
                f"{self.commands['matdyn']} < matdyn_dos.in > matdyn_dos.out",
            ],
        )

    def execute(self):
        execute_commands(self.work_dir, ["bash run_qe.sh"], work_label=os.path.basename(self.work_dir), error_prefix="执行 QE phonon merge continue 出错")


def main():
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO phonon merge + continue")
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("--merge-only", action="store_true", help=argparse.SUPPRESS)
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")
    args = parser.parse_args()
    setup_obj = QEPhononMergeContinueSetup(config_file=args.config, struct_file=args.input)
    if args.merge_only:
        setup_obj.merge_only()
        return
    setup_obj.setup()
    handle_job_action(args.run, args.submit, setup_obj.work_dir, "run_qe.sh", setup_obj.execute)


if __name__ == "__main__":
    main()
