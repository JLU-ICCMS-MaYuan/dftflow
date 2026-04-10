#!/usr/bin/env python3
import argparse
import os
import shlex
import shutil
import sys

from public.config_templates import load_config_file
from public.job import add_run_submit_arguments, execute_commands, handle_job_action, write_job_script
from vaspflow.templates import (
    apply_vasp_config_overrides,
    get_vasp_lobsterin_allowed_keys,
    get_vasp_lobsterin_parameters_template,
)


class VaspLobsterinSetup:
    def __init__(self, config_file="input.toml"):
        self.parameter_template = get_vasp_lobsterin_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_vasp_config_overrides(
            self.parameter_template, self.input_config, get_vasp_lobsterin_allowed_keys()
        )
        self.work_dir = None

    def get_lobster_command(self):
        return self.config.get("lobster_executable_path")

    def validate_lobster_command(self):
        command = self.get_lobster_command()
        if not command:
            raise ValueError("缺少顶层配置 lobster_executable_path，无法执行或提交 LOBSTER。")
        executable = shlex.split(command)[0]
        if os.path.sep in executable or executable.startswith("~"):
            executable_path = os.path.expanduser(executable)
            if not os.path.exists(executable_path):
                raise FileNotFoundError(f"找不到 lobster_executable_path 指向的可执行文件: {executable_path}")
        return command

    def _parse_zvalances(self, zvalances):
        zval_dict = {}
        for entry in zvalances:
            if ":" not in entry:
                raise ValueError(f"价电子参数格式不对 '{entry}'。应为 Element:orb1,orb2")
            element, orbitals = entry.split(":", 1)
            zval_dict[element] = " ".join(orbitals.split(","))
        return zval_dict

    def _parse_custom_basis(self, custom_basis_args):
        if not custom_basis_args:
            return None
        basis_dict = {}
        for entry in custom_basis_args:
            if ":" not in entry:
                raise ValueError(f"自定义基组参数格式不对 '{entry}'。应为 Element:file.sto")
            element, sto_file = entry.split(":", 1)
            basis_dict[element] = sto_file
        return basis_dict

    def prepare_inputs(self, args):
        zval_custom = self._parse_zvalances(args.zvalances)
        custom_basis = self._parse_custom_basis(args.custom_basis)
        self.work_dir = f"{args.species_custom[0]}_{args.species_custom[1]}_{args.d_limit[0]}_{args.d_limit[1]}"
        os.makedirs(self.work_dir, exist_ok=True)
        print(f"创建目录: {self.work_dir}")

        self._link_required_files(kspace_cohp=args.kSpaceCOHP)
        self._write_lobsterin(
            mode=args.mode,
            cohp_start=args.COHPEnergy[0],
            cohp_end=args.COHPEnergy[1],
            species_custom1=args.species_custom[0],
            species_custom2=args.species_custom[1],
            lower_d=args.d_limit[0],
            upper_d=args.d_limit[1],
            zval_dict=zval_custom,
            custom_basis_dict=custom_basis,
            orbital_wise=args.orbitalWise,
            kspace_cohp=args.kSpaceCOHP,
        )
        self._create_run_script()
        print(f"\nLOBSTER 输入文件和运行脚本已在 {self.work_dir} 准备就绪。")
        print("提示: 如果运行 LOBSTER 后得不到 COHP，请检查 NBANDS 是否足够大。")
        print("提示: 确保在 VASP 计算中设置了 LWAVE = .TRUE. 以生成 WAVECAR。")

    def _link_required_files(self, kspace_cohp=False):
        files = ["WAVECAR", "CONTCAR", "KPOINTS", "OUTCAR", "POTCAR", "vasprun.xml", "POSCAR"]
        for filename in files:
            target = os.path.join(self.work_dir, filename)
            if os.path.lexists(target):
                os.remove(target)
            if os.path.exists(filename):
                if kspace_cohp and filename == "KPOINTS":
                    shutil.copy2(filename, target)
                    continue
                os.symlink(os.path.abspath(filename), target)
                continue
            if filename == "CONTCAR" and os.path.exists("POSCAR"):
                os.symlink(os.path.abspath("POSCAR"), target)
                continue
            print(f"警告: {filename} 不存在，LOBSTER 可能运行失败。")

        if kspace_cohp:
            kpath_source = "KPATH.in"
            kpath_target = os.path.join(self.work_dir, "KPATH.in")
            if os.path.lexists(kpath_target):
                os.remove(kpath_target)
            if os.path.exists(kpath_source):
                shutil.copy2(kpath_source, kpath_target)
            else:
                print("警告: KPATH.in 不存在，已请求 kSpaceCOHP，LOBSTER 可能运行失败。")

    def _write_lobsterin(
        self,
        mode,
        cohp_start,
        cohp_end,
        species_custom1,
        species_custom2,
        lower_d,
        upper_d,
        zval_dict,
        custom_basis_dict,
        orbital_wise,
        kspace_cohp,
    ):
        lobsterin_path = os.path.join(self.work_dir, "lobsterin")
        with open(lobsterin_path, "w", encoding="utf-8") as f:
            f.write(f"COHPstartEnergy  {cohp_start}\n")
            f.write(f"COHPendEnergy    {cohp_end}\n")
            if custom_basis_dict:
                f.write("useBasisSet custom\n")
                for element, sto_file in custom_basis_dict.items():
                    f.write(f"customSTOforAtom {element}  {sto_file}\n")
            else:
                f.write("usebasisset pbeVaspFit2015\n")
            if mode == 0:
                f.write("gaussianSmearingWidth 0.05\n")
            for element, orbitals in zval_dict.items():
                f.write(f"basisfunctions {element} {orbitals}\n")
            cohp_generator = (
                "cohpGenerator from "
                f"{lower_d} to {upper_d} type {species_custom1} type {species_custom2}"
            )
            if orbital_wise:
                cohp_generator += " orbitalWise"
            if kspace_cohp:
                cohp_generator += " kSpaceCOHP"
            f.write(cohp_generator + "\n")

    def _create_run_script(self):
        command = self.get_lobster_command()
        slurm_header = self.config.get("slurm", {}).get("header", "")
        run_script_path = os.path.join(self.work_dir, "run_lobster.sh")
        commands = [
            "echo 'Job started at' `date`",
            command or "echo 'lobster_executable_path 未配置'",
            "echo 'Job finished at' `date`",
        ]
        write_job_script(run_script_path, slurm_header, commands)

    def execute(self):
        if not self.work_dir:
            raise RuntimeError("尚未生成工作目录，无法执行 LOBSTER。")
        command = self.validate_lobster_command()
        execute_commands(
            self.work_dir,
            [command],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 LOBSTER 出错",
        )
        print(f"LOBSTER 运行完成，目录: {self.work_dir}")


def parse_args():
    parser = argparse.ArgumentParser(description="生成 LOBSTER 输入文件并准备运行目录")
    parser.add_argument(
        "-e",
        "--COHPEnergy",
        nargs=2,
        type=float,
        default=[-10.0, 5.0],
        help="COHP 起始终止能量, 例如 -e -10 5",
    )
    parser.add_argument("--species_custom", nargs=2, type=str, required=True, help="元素对, 例如 --species_custom Si O")
    parser.add_argument(
        "-z",
        "--zvalances",
        nargs="+",
        type=str,
        required=True,
        help="元素及其价电子轨道, 例如 -z Si:3s,3p O:2s,2p",
    )
    parser.add_argument(
        "-d",
        "--d_limit",
        nargs=2,
        type=float,
        required=True,
        help="原子间最小最大距离, 例如 -d 1.0 2.5",
    )
    parser.add_argument(
        "--orbitalWise",
        action="store_true",
        help="若设置，则在 cohpGenerator 行末增加 orbitalWise",
    )
    parser.add_argument(
        "--kSpaceCOHP",
        action="store_true",
        help="若设置，则在 cohpGenerator 行末增加 kSpaceCOHP",
    )
    parser.add_argument("-m", "--mode", type=int, default=5, help="选择生成的输入文件版本: 5 或 0 (默认 5)")
    parser.add_argument("--custom_basis", nargs="+", type=str, help="使用自定义基组, 例如 --custom_basis H:h.sto")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径 (默认为 input.toml)")
    add_run_submit_arguments(parser, "生成文件后直接执行 LOBSTER", "生成文件后提交 run_lobster.sh")
    return parser.parse_args()


def main():
    args = parse_args()
    try:
        setup = VaspLobsterinSetup(config_file=args.config)
        setup.prepare_inputs(args)
        if args.run or args.submit:
            setup.validate_lobster_command()
        handle_job_action(args.run, args.submit, setup.work_dir, "run_lobster.sh", setup.execute)
    except Exception as exc:
        print(f"错误: {exc}")
        sys.exit(1)


if __name__ == "__main__":
    main()
