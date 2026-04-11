#!/usr/bin/env python3
import argparse
import os
import shutil
import subprocess
from typing import List
from public.config_templates import load_config_file
from public.executables import resolve_vasp_command
from public.job import add_run_submit_arguments, execute_commands, handle_job_action, write_job_script
from vaspflow.templates import (
    apply_vasp_config_overrides,
    get_vasp_opt_allowed_keys,
    get_vasp_opt_parameters_template,
)

try:
    import tomllib  # Python 3.11+
except ImportError:
    try:
        import toml
    except ImportError:
        print("Error: 'toml' or 'tomllib' (Python 3.11+) is required.")
        print("Please install it using: pip install toml")
        raise SystemExit(1)


class VaspOptSetup:
    def __init__(self, config_file="input.toml", struct_file=None, pressure_override=None, work_dir="vasp_opt"):
        self.work_dir = work_dir
        self.parameter_template = get_vasp_opt_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_vasp_config_overrides(
            self.parameter_template, self.input_config, get_vasp_opt_allowed_keys()
        )
        self.struct_file = struct_file
        self.opt_params = self._load_opt_params()
        self.opt_stage_templates = self.config["opt_stage_templates"]
        self.pstress_value = self._resolve_pstress(pressure_override)

    @staticmethod
    def _gpa_to_kbar(pressure_gpa):
        return float(pressure_gpa) * 10.0

    def _load_opt_params(self):
        opt_params = self.config.get("vasp_opt")
        if not opt_params:
            raise ValueError("配置缺少 [vasp_opt] 块")
        return opt_params

    def _resolve_pstress(self, pressure_override):
        if pressure_override is not None:
            return self._gpa_to_kbar(pressure_override)
        pstress = self.opt_params.get("PSTRESS")
        if pstress is None:
            raise ValueError("缺少 PSTRESS，且未通过 -p 指定")
        return pstress

    def _write_incar(self, filepath, lines):
        with open(filepath, "w") as incar:
            for line in lines:
                incar.write(line if line.endswith("\n") else line + "\n")

    def get_elements_from_poscar(self):
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        if not os.path.exists(poscar_path):
            raise FileNotFoundError(f"找不到 {poscar_path} 文件")
        with open(poscar_path, "r") as f:
            lines = f.readlines()
            elements = lines[5].split()
            if elements and elements[0].isdigit():
                print("警告: 检测到可能不包含元素名称的 POSCAR (VASP 4 格式)。")
                print("请确保 POSCAR 第6行包含元素名称。")
            return elements

    def generate_potcar(self, elements):
        print(f"检测到元素顺序: {' '.join(elements)}")
        potcar_dir = self.config.get("potcar_dir", "")
        if not potcar_dir:
            raise ValueError("未在配置文件中找到 'potcar_dir' (赝势目录路径)")
        potcar_dir = os.path.expanduser(potcar_dir)
        potcar_content = []
        for el in elements:
            path = os.path.join(potcar_dir, el)
            if not os.path.exists(path):
                raise FileNotFoundError(f"在 {potcar_dir} 中找不到元素 {el} 的 POTCAR 文件")
            with open(path, "r") as f:
                potcar_content.append(f.read())
        potcar_path = os.path.join(self.work_dir, "POTCAR")
        with open(potcar_path, "w") as f:
            f.writelines(potcar_content)
        print(f"POTCAR 已从 {potcar_dir} 合并完成。")

    def opt_incar1(self, incar_dirpath):
        incar_filepath = os.path.join(incar_dirpath, "INCAR_1")
        params = self.opt_stage_templates["stage1"]
        lines = [
            f"ISTART   = {params['ISTART']}",
            f"ICHARG   = {params['ICHARG']}",
            f"ISYM     = {params['ISYM']}",
            f"ENCUT    = {params['ENCUT']}",
            f"PREC     = {params['PREC']}",
            f"SYMPREC  = {self.opt_params['SYMPREC']}",
            f"NCORE    = {params['NCORE']}",
            f"KSPACING = {params['KSPACING']}",
            f"ISMEAR   = {params['ISMEAR']}",
            f"SIGMA    = {params['SIGMA']}",
            f"NELM     = {params['NELM']}",
            f"NELMIN   = {params['NELMIN']}",
            f"EDIFF    = {params['EDIFF']}",
            f"EDIFFG   = {params['EDIFFG']}",
            f"NSW      = {params['NSW']}",
            f"IBRION   = {params['IBRION']}",
            f"ISIF     = {params['ISIF']}",
            f"POTIM    = {params['POTIM']}",
            f"LWAVE    = {params['LWAVE']}",
            f"LCHARG   = {params['LCHARG']}",
            f"PSTRESS  = {float(self.pstress_value)}",
        ]
        self._write_incar(incar_filepath, lines)
        return incar_filepath

    def opt_incar2(self, incar_dirpath):
        incar_filepath = os.path.join(incar_dirpath, "INCAR_2")
        params = self.opt_stage_templates["stage2"]
        lines = [
            f"ISTART   = {params['ISTART']}",
            f"ICHARG   = {params['ICHARG']}",
            f"ISYM     = {params['ISYM']}",
            f"ENCUT    = {params['ENCUT']}",
            f"PREC     = {params['PREC']}",
            f"SYMPREC  = {self.opt_params['SYMPREC']}",
            f"NCORE    = {params['NCORE']}",
            f"KSPACING = {params['KSPACING']}",
            f"ISMEAR   = {params['ISMEAR']}",
            f"SIGMA    = {params['SIGMA']}",
            f"NELM     = {params['NELM']}",
            f"NELMIN   = {params['NELMIN']}",
            f"EDIFF    = {params['EDIFF']}",
            f"EDIFFG   = {params['EDIFFG']}",
            f"NSW      = {params['NSW']}",
            f"IBRION   = {params['IBRION']}",
            f"ISIF     = {params['ISIF']}",
            f"POTIM    = {params['POTIM']}",
            f"LWAVE    = {params['LWAVE']}",
            f"LCHARG   = {params['LCHARG']}",
            f"PSTRESS  = {float(self.pstress_value)}",
        ]
        self._write_incar(incar_filepath, lines)
        return incar_filepath

    def opt_incar3(self, incar_dirpath):
        incar_filepath = os.path.join(incar_dirpath, "INCAR_3")
        params = self.opt_stage_templates["stage3"]
        lines = [
            f"ISTART   = {params['ISTART']}",
            f"ICHARG   = {params['ICHARG']}",
            f"ISYM     = {params['ISYM']}",
            f"ENCUT    = {params['ENCUT']}",
            f"PREC     = {params['PREC']}",
            f"SYMPREC  = {self.opt_params['SYMPREC']}",
            f"NCORE    = {params['NCORE']}",
            f"KSPACING = {params['KSPACING']}",
            f"ISMEAR   = {params['ISMEAR']}",
            f"SIGMA    = {params['SIGMA']}",
            f"NELM     = {params['NELM']}",
            f"NELMIN   = {params['NELMIN']}",
            f"EDIFF    = {params['EDIFF']}",
            f"EDIFFG   = {params['EDIFFG']}",
            f"NSW      = {params['NSW']}",
            f"IBRION   = {params['IBRION']}",
            f"ISIF     = {params['ISIF']}",
            f"POTIM    = {params['POTIM']}",
            f"LWAVE    = {params['LWAVE']}",
            f"LCHARG   = {params['LCHARG']}",
            f"PSTRESS  = {float(self.pstress_value)}",
        ]
        self._write_incar(incar_filepath, lines)
        return incar_filepath

    def opt_incar4(self, incar_dirpath):
        incar_filepath = os.path.join(incar_dirpath, "INCAR_4")
        lines = [
            "ISTART   = 0",
            "ICHARG   = 2",
            f"ISYM     = {self.opt_params['ISYM']}",
            f"ENCUT    = {self.opt_params['ENCUT']}",
            "PREC     = Accurate",
            f"SYMPREC  = {self.opt_params['SYMPREC']}",
        ]
        if self.opt_params.get("NPAR") is not None:
            lines.append(f"NPAR   = {self.opt_params['NPAR']}")
        if self.opt_params.get("NCORE") is not None:
            lines.append(f"NCORE  = {self.opt_params['NCORE']}")
        if self.opt_params.get("KPAR") is not None:
            lines.append(f"KPAR   = {self.opt_params['KPAR']}")
        kspacing = self.opt_params.get("KSPACING")
        if kspacing is None:
            kmesh = self.opt_params.get("kmesh")
            kspacing = None if kmesh is None else kmesh * 2 * 3.1415926
        if kspacing is None:
            raise ValueError("缺少 KSPACING 且未设置 [vasp_opt].kmesh")
        ediff = self.opt_params.get("EDIFF", 1e-7)
        ediffg = self.opt_params.get("EDIFFG", -0.01)
        ibrion = self.opt_params.get("IBRION", 2)
        isif = self.opt_params.get("ISIF", 3)
        lines.extend([
            f"KSPACING = {kspacing}",
            f"ISMEAR   = {self.opt_params['ISMEAR']}",
            f"SIGMA    = {self.opt_params['SIGMA']}",
            f"NELM     = {self.opt_params['NELM']}",
            "NELMIN   = 6",
            f"EDIFF    = {ediff}",
            f"EDIFFG   = {ediffg}",
            "NSW      = 500",
            f"IBRION   = {ibrion}",
            f"ISIF     = {isif}",
            f"POTIM    = {self.opt_params['POTIM']}",
            "LWAVE  = .FALSE.",
            "LCHARG = .FALSE.",
            f"PSTRESS  = {float(self.pstress_value)}",
        ])
        self._write_incar(incar_filepath, lines)
        return incar_filepath

    def opt_incar(self, incar_dirpath):
        incar_filepath = os.path.join(incar_dirpath, "INCAR")
        lines = [
            "ISTART   = 0",
            "ICHARG   = 2",
            f"ISYM     = {self.opt_params['ISYM']}",
            f"ENCUT    = {self.opt_params['ENCUT']}",
            "PREC     = Accurate",
            f"SYMPREC  = {self.opt_params['SYMPREC']}",
        ]
        if self.opt_params.get("NPAR") is not None:
            lines.append(f"NPAR   = {self.opt_params['NPAR']}")
        if self.opt_params.get("NCORE") is not None:
            lines.append(f"NCORE  = {self.opt_params['NCORE']}")
        if self.opt_params.get("KPAR") is not None:
            lines.append(f"KPAR   = {self.opt_params['KPAR']}")
        kspacing = self.opt_params.get("KSPACING")
        if kspacing is None:
            kmesh = self.opt_params.get("kmesh")
            kspacing = None if kmesh is None else kmesh * 2 * 3.1415926
        if kspacing is None:
            raise ValueError("缺少 KSPACING 且未设置 [vasp_opt].kmesh")
        ediff = self.opt_params.get("EDIFF", 1e-7)
        ediffg = self.opt_params.get("EDIFFG", -0.01)
        ibrion = self.opt_params.get("IBRION", 2)
        isif = self.opt_params.get("ISIF", 3)
        lines.extend([
            f"KSPACING = {kspacing}",
            f"ISMEAR   = {self.opt_params['ISMEAR']}",
            f"SIGMA    = {self.opt_params['SIGMA']}",
            f"NELM     = {self.opt_params['NELM']}",
            "NELMIN   = 6",
            f"EDIFF    = {ediff}",
            f"EDIFFG   = {ediffg}",
            "NSW      = 500",
            f"IBRION   = {ibrion}",
            f"ISIF     = {isif}",
            f"POTIM    = {self.opt_params['POTIM']}",
            "LWAVE  = .FALSE.",
            "LCHARG = .FALSE.",
            f"PSTRESS  = {float(self.pstress_value)}",
        ])
        self._write_incar(incar_filepath, lines)
        return incar_filepath

    def _get_vasp_execmd(self):
        vasp_config = self.config.get("vasp", {})
        return resolve_vasp_command(vasp_config.get("executable_path"))

    def _get_slurm_header(self):
        slurm_config = self.config.get("slurm", {})
        return slurm_config.get("header", "")

    def _write_job_header(self, submit_file):
        header = self._get_slurm_header()
        if header:
            submit_file.write(header.strip() + "\n\n")
        else:
            submit_file.write("#!/bin/bash\n")

    def fouropt(self, submit_dirpath):
        jobname = "fouropt.sh"
        submit_script_filepath = os.path.join(submit_dirpath, jobname)
        vasp_execmd = self._get_vasp_execmd()
        with open(submit_script_filepath, "w") as submit:
            self._write_job_header(submit)
            submit.write("for i in {1..4}; do\n")
            submit.write("cp INCAR_$i INCAR\n")
            submit.write(f"{vasp_execmd}\n")
            submit.write("cp CONTCAR CONTCAR_$i\n")
            submit.write("cp OUTCAR OUTCAR_$i\n")
            submit.write("cp CONTCAR POSCAR\n")
            submit.write("done\n")
        os.chmod(submit_script_filepath, 0o755)
        return jobname

    def oneopt(self, submit_dirpath):
        jobname = "oneopt.sh"
        submit_script_filepath = os.path.join(submit_dirpath, jobname)
        vasp_execmd = self._get_vasp_execmd()
        with open(submit_script_filepath, "w") as submit:
            self._write_job_header(submit)
            submit.write(f"{vasp_execmd}\n")
        os.chmod(submit_script_filepath, 0o755)
        return jobname

    def fopt(self, submit_dirpath):
        jobname = "fopt.sh"
        submit_script_filepath = os.path.join(submit_dirpath, jobname)
        vasp_execmd = self._get_vasp_execmd()
        with open(submit_script_filepath, "w") as submit:
            self._write_job_header(submit)
            submit.write("num=0\n")
            submit.write("while true;do\n")
            submit.write("        let num+=1\n")
            submit.write("        echo \"run fine vasp opt-$num\"\n")
            submit.write(f"        timeout 14400s {vasp_execmd}\n")
            submit.write("        cp -f CONTCAR CONTCAR-fine &&  cp -f CONTCAR POSCAR\n")
            submit.write("        rows=`sed -n '/F\\=/p' OSZICAR | wc -l`\n")
            submit.write("        echo \"rows-$rows\"\n")
            submit.write("        echo $num >> count_opt_times\n")
            submit.write("        if [ \"$rows\" -eq \"1\" ];then\n")
            submit.write("                break\n")
            submit.write("        fi\n")
            submit.write("        if [ \"$num\" -gt \"50\" ]; then\n")
            submit.write("                break\n")
            submit.write("        fi\n")
            submit.write("done\n")
        os.chmod(submit_script_filepath, 0o755)
        return jobname

    def _prepare_poscar(self):
        os.makedirs(self.work_dir, exist_ok=True)
        target_poscar = os.path.join(self.work_dir, "POSCAR")
        if self.struct_file:
            if not os.path.exists(self.struct_file):
                raise FileNotFoundError(f"找不到结构文件: {self.struct_file}")
            shutil.copy2(self.struct_file, target_poscar)
            print(f"已将 {self.struct_file} 拷贝为 {target_poscar}")
        elif os.path.exists("POSCAR") and os.path.abspath("POSCAR") != os.path.abspath(target_poscar):
            shutil.copy2("POSCAR", target_poscar)
            print(f"已将当前目录下的 POSCAR 拷贝为 {target_poscar}")

    def run(self, mode):
        self._prepare_poscar()
        elements = self.get_elements_from_poscar()
        self.generate_potcar(elements)
        if mode == "rv4":
            self.opt_incar1(self.work_dir)
            self.opt_incar2(self.work_dir)
            self.opt_incar3(self.work_dir)
            self.opt_incar4(self.work_dir)
            self._script_name = self.fouropt(self.work_dir)
        elif mode == "rv1":
            self.opt_incar(self.work_dir)
            self._script_name = self.oneopt(self.work_dir)
        elif mode == "rvf":
            self.opt_incar(self.work_dir)
            self._script_name = self.fopt(self.work_dir)
        else:
            raise ValueError("mode must be one of rv4/rv1/rvf")
        print(f"已生成优化输入于 {self.work_dir}，提交脚本: {self._script_name}")

    def execute(self):
        """执行结构优化"""
        script = getattr(self, "_script_name", "fouropt.sh")
        execute_commands(
            self.work_dir,
            [f"bash {script}"],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 VASP 结构优化出错",
            expected_outputs=["OUTCAR", "CONTCAR"]
        )
        print(f"VASP 结构优化完成，输出已保存至 {self.work_dir}")


def _format_pressure_dirname(pressure_gpa: float) -> str:
    return f"{pressure_gpa:g}"


def _parse_pressure_values(pressure_args: List[str]) -> List[float]:
    values = []
    for raw_value in pressure_args:
        try:
            values.append(float(raw_value))
        except ValueError as exc:
            raise ValueError(f"无法解析压力值: {raw_value}") from exc
    return values


def _create_batch_submit_script(setups: List[VaspOptSetup], script_name: str = "submit_all_vasp_opt.sh") -> str:
    if not setups:
        raise ValueError("没有可用于批量提交的 vasp_opt 任务。")
    slurm_header = setups[0].config.get("slurm", {}).get("header", "")
    script_path = os.path.abspath(script_name)
    commands = []
    for setup in setups:
        case_dir = os.path.abspath(setup.work_dir)
        case_script = getattr(setup, "_script_name", "fouropt.sh")
        commands.append(f'cd "{case_dir}"')
        commands.append(f"bash {case_script}")
    write_job_script(script_path, slurm_header, commands)
    return os.path.basename(script_path)


def _run_multiple_pressures(config_file, struct_file, mode, pressure_values):
    setups = []
    for pressure_gpa in pressure_values:
        work_dir = os.path.join(_format_pressure_dirname(pressure_gpa), "vasp_opt")
        setup = VaspOptSetup(
            config_file=config_file,
            struct_file=struct_file,
            pressure_override=pressure_gpa,
            work_dir=work_dir,
        )
        setup.run(mode=mode)
        setups.append(setup)
    return setups


def main():
    parser = argparse.ArgumentParser(description="VASP structure optimization (rv4/rv1/rvf)")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径 (默认为 input.toml)")
    parser.add_argument("-m", "--mode", choices=["rv4", "rv1", "rvf"], default="rv4", help="优化模式")
    parser.add_argument(
        "-p",
        "--pressure",
        nargs="+",
        help="覆盖 PSTRESS，输入单位为 GPa（支持多个值，写入 INCAR 时自动转换为 kbar）",
    )
    add_run_submit_arguments(parser, "生成文件后直接执行优化", "生成文件后提交优化脚本")
    args = parser.parse_args()

    if args.pressure:
        pressure_values = _parse_pressure_values(args.pressure)
        if len(pressure_values) == 1:
            pressure_value = pressure_values[0]
            work_dir = os.path.join(_format_pressure_dirname(pressure_value), "vasp_opt")
            setup = VaspOptSetup(
                config_file=args.config,
                struct_file=args.input,
                pressure_override=pressure_value,
                work_dir=work_dir,
            )
            setup.run(mode=args.mode)
            script_name = getattr(setup, "_script_name", "fouropt.sh")
            handle_job_action(args.run, args.submit, setup.work_dir, script_name, setup.execute)
            return

        setups = _run_multiple_pressures(args.config, args.input, args.mode, pressure_values)
        batch_script_name = _create_batch_submit_script(setups)

        def execute_all():
            for setup in setups:
                setup.execute()

        handle_job_action(args.run, args.submit, ".", batch_script_name, execute_all)
        return

    setup = VaspOptSetup(
        config_file=args.config,
        struct_file=args.input,
        pressure_override=None,
        work_dir="vasp_opt",
    )
    setup.run(mode=args.mode)
    script_name = getattr(setup, "_script_name", "fouropt.sh")
    handle_job_action(args.run, args.submit, setup.work_dir, script_name, setup.execute)


if __name__ == "__main__":
    main()
