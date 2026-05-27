#!/usr/bin/env python3
import argparse

from public.job import add_run_submit_arguments
from qeflow.phonon.archive import (
    PHBAND_DIR,
    PHDOS_DIR,
    build_collect_phband_commands,
    build_collect_phdos_commands,
    collect_phband_outputs,
    collect_phdos_outputs,
    phband_archive_files,
    phdos_archive_files,
    phonon_plot_files,
)
from qeflow.phonon.phonon_plot import plot_phonon_outputs
from qeflow.phonon.phonon_tables import build_phonon_band_tables, build_phonon_dos_tables
from qeflow.phonon.postprocess_workflow import (
    PROCESS_SCRIPT,
    prepare_matdyn_phband_collect_step,
    prepare_matdyn_phdos_collect_step,
    prepare_process_phonon_step,
    run_process_phonon,
    run_process_phonon_steps,
)
from qeflow.phonon.qe_matdyn_phband import prepare_matdyn_phband_step
from qeflow.phonon.qe_matdyn_phdos import prepare_matdyn_phdos_step
from qeflow.phonon.qe_q2r import prepare_q2r_step


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO 声子 q2r/matdyn 后处理编排与绘图")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    parser.add_argument("-i", "--input", default=None, help="结构文件路径")
    parser.add_argument(
        "--collect",
        nargs="?",
        const="all",
        choices=("all", "phband", "phdos"),
        help="只归档已有结果，可选 phband/phdos；省略目标时同时归档二者",
    )
    parser.add_argument("--plot", action="store_true", help="只读取 phband/phdos 归档结果并绘图")
    add_run_submit_arguments(parser, "本地运行 q2r/phband/phdos、归档并绘图", "提交 q2r/phband/phdos 与归档脚本")
    args = parser.parse_args()
    if args.collect and args.plot:
        parser.error("--collect 不能与 --plot 同时使用。")
    if (args.collect or args.plot) and (args.run or args.submit):
        parser.error("--collect/--plot 不能与 -r/-s 同时使用。")
    raise SystemExit(run_process_phonon(args.config, args.input, args.run, args.submit, args.collect, args.plot))


if __name__ == "__main__":
    main()
