#!/usr/bin/env python3
import os
import shutil
import argparse
import math
import subprocess
from public.config_templates import load_config_file
from public.executables import resolve_vasp_command
from public.job import add_run_submit_arguments, handle_job_action, write_job_script, execute_commands
from vaspflow.templates import (
    apply_vasp_config_overrides,
    get_vasp_scf_allowed_keys,
    get_vasp_scf_parameters_template,
)

try:
    import tomllib  # Python 3.11+
except ImportError:
    try:
        import toml
    except ImportError:
        print("Error: 'toml' or 'tomllib' (Python 3.11+) is required.")
        print("Please install it using: pip install toml")
        exit(1)

class VaspSetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.work_dir = "vasp_scf"
        self.parameter_template = get_vasp_scf_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_vasp_config_overrides(
            self.parameter_template, self.input_config, get_vasp_scf_allowed_keys()
        )
        self.struct_file = struct_file

    def get_elements_from_poscar(self):
        """从 POSCAR 中读取元素名称和顺序"""
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        if not os.path.exists(poscar_path):
            raise FileNotFoundError(f"找不到 {poscar_path} 文件")
        
        with open(poscar_path, "r") as f:
            lines = f.readlines()
            # 这里的逻辑假设 POSCAR 符合 VASP 5 标准（第6行是元素名称）
            # 第1行: 注释
            # 第2行: 缩放因子
            # 第3-5行: 晶格矢量
            # 第6行: 元素名称
            elements = lines[5].split()
            # 如果第6行全是数字，说明是旧版 POSCAR，元素名可能在第1行或者缺失
            if elements[0].isdigit():
                print("警告: 检测到可能不包含元素名称的 POSCAR (VASP 4 格式)。")
                print("请确保 POSCAR 第6行包含元素名称。")
            return elements

    def generate_incar(self):
        """生成 INCAR 文件"""
        final_params = self.config["vasp_scf"].copy()
        final_params.pop("kmesh", None)
        
        print(f"正在生成 INCAR...")
        incar_path = os.path.join(self.work_dir, "INCAR")
        with open(incar_path, "w") as f:
            for key, value in final_params.items():
                # 处理字符串布尔值或特殊格式
                f.write(f"{key:12} = {value}\n")

    def generate_potcar(self, elements):
        """根据 POSCAR 顺序从指定目录合并 POTCAR"""
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

    def generate_kpoints(self):
        """使用 vaspkit 生成 KPOINTS 文件"""
        kmesh = self.config["vasp_scf"].get("kmesh")
        if not kmesh:
            print("未在配置文件中找到 'kmesh'，跳过生成 KPOINTS。")
            return

        print(f"检测到 kmesh = {kmesh}，正在使用 vaspkit 生成 KPOINTS...")
        try:
            process = subprocess.Popen(
                ["vaspkit"],
                cwd=self.work_dir,
                stdin=subprocess.PIPE,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                text=True
            )
            # vaspkit 102 -> 2 -> kmesh
            process.communicate(input=f"102\n2\n{kmesh}\n")
            
            kpoints_file = os.path.join(self.work_dir, "KPOINTS")
            if os.path.exists(kpoints_file):
                print(f"KPOINTS 已通过 vaspkit 生成完成。")
            else:
                print(f"错误: vaspkit 未生成 KPOINTS。")
        except Exception as e:
            print(f"调用 vaspkit 出错: {e}")

    def create_run_script(self):
        """创建一个提交脚本 (支持 Slurm)"""
        vasp_config = self.config.get("vasp", {})
        vasp_path = resolve_vasp_command(vasp_config.get("executable_path"))

        slurm_config = self.config.get("slurm", {})
        slurm_header = slurm_config.get("header", "")

        run_script_path = os.path.join(self.work_dir, "run_vasp.sh")
        write_job_script(
            run_script_path,
            slurm_header,
            [
                "echo 'Job started at' `date`",
                vasp_path,
                "echo 'Job finished at' `date`"
            ]
        )

    def execute(self):
        """执行 VASP 计算"""
        vasp_config = self.config.get("vasp", {})
        vasp_path = resolve_vasp_command(vasp_config.get("executable_path"))

        execute_commands(
            self.work_dir,
            [vasp_path],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 VASP 出错",
            expected_outputs=["OUTCAR", "CHGCAR", "WAVECAR"]
        )
        print(f"VASP 计算完成，输出已保存至 {self.work_dir}")

    def setup(self):
        """准备所有 VASP 输入文件"""
        try:
            # 确保工作目录存在
            os.makedirs(self.work_dir, exist_ok=True)

            # 0. 如果指定了输入结构文件，先拷贝为 POSCAR
            target_poscar = os.path.join(self.work_dir, "POSCAR")
            if self.struct_file:
                if not os.path.exists(self.struct_file):
                    raise FileNotFoundError(f"找不到结构文件: {self.struct_file}")
                shutil.copy2(self.struct_file, target_poscar)
                print(f"已将 {self.struct_file} 拷贝为 {target_poscar}")
            elif os.path.exists("POSCAR") and os.path.abspath("POSCAR") != os.path.abspath(target_poscar):
                shutil.copy2("POSCAR", target_poscar)
                print(f"已将当前目录下的 POSCAR 拷贝为 {target_poscar}")

            elements = self.get_elements_from_poscar()
            self.generate_incar()
            self.generate_potcar(elements)
            self.generate_kpoints()
            self.create_run_script()
            print(f"\n所有 VASP 输入文件已在 {self.work_dir} 目录中准备就绪！")

        except Exception as e:
            print(f"错误: {e}")
            raise

    def run(self, run_calc=False):
        """向后兼容的方法"""
        self.setup()
        if run_calc:
            self.execute()

def main():
    parser = argparse.ArgumentParser(description="VASP SCF Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径 (默认为 input.toml)")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_vasp.sh")

    args = parser.parse_args()

    setup = VaspSetup(config_file=args.config, struct_file=args.input)
    setup.setup()
    handle_job_action(args.run, args.submit, setup.work_dir, "run_vasp.sh", setup.execute)

if __name__ == "__main__":
    main()
