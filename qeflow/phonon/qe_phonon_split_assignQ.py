#!/usr/bin/env python3
import argparse
import os
from pathlib import Path

from public.job import add_run_submit_arguments, execute_commands, handle_job_action
from qeflow.phonon.common import (
    build_qe_commands,
    create_job_script,
    derive_prefix,
    derive_phonon_filenames,
    load_phonon_config,
    parse_poscar,
    prepare_work_tree,
    resolve_phonon_qpoints,
    write_python_helper_script,
    write_scf_input,
    write_split_ph_input_assignq,
)
from qeflow.phonon.qgrid import generate_irreducible_qpoints, write_dyn0, write_q_index_map


class QEPhononSplitAssignQSetup:
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
        if os.path.exists("POSCAR"):
            return "POSCAR"
        raise FileNotFoundError("找不到结构文件。请在当前目录准备 POSCAR 或使用 -i 指定。")

    def _xml_path(self) -> str:
        outdir = self.config["qe_phonon"]["scf"]["outdir"]
        return os.path.join(self.work_dir, outdir, f"{self.prefix}.save", "data-file-schema.xml")

    def materialize_split_directories(self) -> int:
        ph_cfg = self.config["qe_phonon"]["ph"]
        filenames = derive_phonon_filenames(self.prefix)
        nq1, nq2, nq3 = resolve_phonon_qpoints(ph_cfg, ("nq1", "nq2", "nq3"))
        qpoints = generate_irreducible_qpoints(self._xml_path(), nq1, nq2, nq3)
        write_q_index_map(os.path.join(self.work_dir, "q_index_map.json"), qpoints)
        write_dyn0(os.path.join(self.work_dir, f"{filenames['fildyn']}0"), nq1, nq2, nq3, qpoints)
        root_scf = Path(self.work_dir) / "scf.in"
        root_poscar = Path(self.work_dir) / "POSCAR"
        child_dirs = []
        for item in qpoints:
            q_dir = Path(self.work_dir) / f"q_{int(item['index']):02d}"
            q_dir.mkdir(parents=True, exist_ok=True)
            child_dirs.append(q_dir)
            (q_dir / "scf.in").write_text(root_scf.read_text(encoding="utf-8"), encoding="utf-8")
            (q_dir / "POSCAR").write_text(root_poscar.read_text(encoding="utf-8"), encoding="utf-8")
            write_split_ph_input_assignq(str(q_dir), self.config, self.prefix, int(item["index"]))
            create_job_script(
                str(q_dir / "run_qe.sh"),
                self.config,
                [
                    f"{self.commands['pw']} < scf.in > scf.out",
                    f"{self.commands['ph']} < split_ph.in > split_ph.out",
                ],
            )
        return len(child_dirs)

    def write_prepare_helper(self) -> None:
        helper_path = os.path.join(self.work_dir, "prepare_split.py")
        write_python_helper_script(
            helper_path,
            """import os
from qeflow.phonon.common import derive_prefix, parse_poscar
from qeflow.phonon.qe_phonon_split_assignQ import QEPhononSplitAssignQSetup

os.chdir(os.path.dirname(os.path.abspath(__file__)))
setup = QEPhononSplitAssignQSetup(config_file="input.toml", struct_file="POSCAR")
setup.work_dir = "."
struct_info = parse_poscar("POSCAR")
setup.prefix = derive_prefix(setup.config, struct_info)
setup.materialize_split_directories()
""",
        )

    def setup(self):
        struct_path = self._resolve_structure()
        target_poscar = prepare_work_tree(self.work_dir, struct_path, self.config_file)
        struct_info = parse_poscar(target_poscar)
        self.prefix = derive_prefix(self.config, struct_info)
        write_scf_input(self.work_dir, self.config, struct_info, self.prefix)
        self.write_prepare_helper()
        create_job_script(
            os.path.join(self.work_dir, "run_qe.sh"),
            self.config,
            [
                f"{self.commands['pw']} < scf.in > scf.out",
                "python prepare_split.py",
                "for d in q_*; do bash \"$d/run_qe.sh\"; done",
            ],
        )

    def execute(self):
        execute_commands(self.work_dir, ["bash run_qe.sh"], work_label=os.path.basename(self.work_dir), error_prefix="执行 QE phonon split assignQ 出错")


def main():
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO phonon 按不可约 q 点编号拆分")
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("--prepare-only", action="store_true", help=argparse.SUPPRESS)
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")
    args = parser.parse_args()
    setup_obj = QEPhononSplitAssignQSetup(config_file=args.config, struct_file=args.input)
    if args.prepare_only:
        if args.input:
            setup_obj.struct_file = args.input
        struct_info = parse_poscar(os.path.join(setup_obj.work_dir, "POSCAR"))
        setup_obj.prefix = derive_prefix(setup_obj.config, struct_info)
        setup_obj.materialize_split_directories()
        return
    setup_obj.setup()
    handle_job_action(args.run, args.submit, setup_obj.work_dir, "run_qe.sh", setup_obj.execute)


if __name__ == "__main__":
    main()
