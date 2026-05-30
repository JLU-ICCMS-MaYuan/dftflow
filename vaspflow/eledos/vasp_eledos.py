#!/usr/bin/env python3
import os
import shutil
import argparse
import subprocess
from public.config_templates import load_config_file
from public.executables import resolve_vasp_command
from public.job import add_run_submit_arguments, handle_job_action, write_job_script, execute_commands
from vaspflow.templates import (
    apply_vasp_config_overrides,
    get_vasp_eledos_allowed_keys,
    get_vasp_eledos_parameters_template,
    get_vasp_scf_parameters_template,
)
from vaspflow.kpoints import (
    AutomaticKpoints,
    derive_integer_kmesh_multiplier,
    expected_scaled_mesh,
    read_automatic_kpoints,
    write_automatic_kpoints,
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

class VaspDosSetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.work_dir = "vasp_eledos"
        self.parameter_template = get_vasp_eledos_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_vasp_config_overrides(
            self.parameter_template, self.input_config, get_vasp_eledos_allowed_keys()
        )
        self.struct_file = struct_file

    def get_elements_from_poscar(self):
        """从 POSCAR 中读取元素名称和顺序"""
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        if not os.path.exists(poscar_path):
            raise FileNotFoundError(f"找不到 {poscar_path} 文件")
        
        with open(poscar_path, "r") as f:
            lines = f.readlines()
            elements = lines[5].split()
            if elements[0].isdigit():
                print("警告: 检测到可能不包含元素名称的 POSCAR (VASP 4 格式)。")
            return elements

    def generate_incar(self):
        """生成 INCAR 文件"""
        final_params = self.config["vasp_eledos"].copy()
        final_params.pop("kmesh", None)
        
        print(f"正在生成 INCAR (DOS)...")
        incar_path = os.path.join(self.work_dir, "INCAR")
        with open(incar_path, "w") as f:
            for key, value in final_params.items():
                f.write(f"{key:12} = {value}\n")

    def generate_potcar(self, elements):
        """根据 POSCAR 顺序合并 POTCAR"""
        potcar_dir = self.config.get("potcar_dir", "")
        if not potcar_dir:
            raise ValueError("未在配置文件中找到 'potcar_dir'")
        
        potcar_dir = os.path.expanduser(potcar_dir)
        potcar_content = []
        for el in elements:
            path = os.path.join(potcar_dir, el)
            if not os.path.exists(path):
                raise FileNotFoundError(f"找不到元素 {el} 的 POTCAR")
            with open(path, "r") as f:
                potcar_content.append(f.read())
        
        potcar_path = os.path.join(self.work_dir, "POTCAR")
        with open(potcar_path, "w") as f:
            f.writelines(potcar_content)
        print(f"POTCAR 已在 {self.work_dir} 中准备就绪。")

    def generate_kpoints(self):
        """使用 vaspkit 生成 KPOINTS 文件，并校准到 SCF 网格的整数倍。"""
        kmesh = self.config["vasp_eledos"].get("kmesh")
        if kmesh:
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
        else:
            print("未在配置文件中找到 'kmesh'，将直接根据 vasp_scf/KPOINTS 生成 KPOINTS。")

        self.ensure_kpoints_integer_multiple_of_scf()

    def ensure_kpoints_integer_multiple_of_scf(self):
        """确保 DOS KPOINTS 是 SCF KPOINTS 按 kmesh 比例得到的整数倍。"""
        scf_kpoints_path = self.get_scf_kpoints_path()
        eledos_kpoints_path = os.path.join(self.work_dir, "KPOINTS")
        scf_kpoints = read_automatic_kpoints(scf_kpoints_path)

        multiplier = self.get_eledos_kmesh_multiplier()
        expected_mesh = expected_scaled_mesh(scf_kpoints.mesh, multiplier)

        if os.path.exists(eledos_kpoints_path):
            eledos_kpoints = read_automatic_kpoints(eledos_kpoints_path)
            if eledos_kpoints.mesh == expected_mesh:
                print(
                    "KPOINTS 网格已满足 "
                    f"vasp_scf {scf_kpoints.mesh} -> vasp_eledos {eledos_kpoints.mesh} "
                    f"的 {multiplier} 倍关系。"
                )
                return
            print(
                "KPOINTS 网格未满足按 kmesh 推导的整数倍关系，"
                f"将从 {eledos_kpoints.mesh} 修正为 {expected_mesh}。"
            )

        corrected = AutomaticKpoints(
            title=f"Automatic mesh scaled from {os.path.basename(os.path.dirname(scf_kpoints_path))}",
            style=scf_kpoints.style,
            mesh=expected_mesh,
            shift=scf_kpoints.shift,
        )
        write_automatic_kpoints(eledos_kpoints_path, corrected)
        print(
            f"KPOINTS 已按 vasp_scf 网格 {scf_kpoints.mesh} 的 {multiplier} 倍写入: {expected_mesh}"
        )

    def get_scf_kpoints_path(self):
        """优先从 CHGCAR 路径定位同目录 KPOINTS，失败时使用默认 vasp_scf/KPOINTS。"""
        chgcar_src = self.config.get("chgcar_path")
        candidates = []
        if chgcar_src:
            candidates.append(os.path.join(os.path.dirname(os.path.expanduser(chgcar_src)), "KPOINTS"))
        candidates.append(os.path.join("vasp_scf", "KPOINTS"))

        for candidate in candidates:
            if candidate and os.path.exists(candidate):
                return candidate
        raise FileNotFoundError("找不到 vasp_scf 的 KPOINTS，无法校准 vasp_eledos 网格。")

    def get_eledos_kmesh_multiplier(self):
        """根据 vasp_scf.kmesh 与 vasp_eledos.kmesh 推导整数网格倍数。"""
        scf_default = get_vasp_scf_parameters_template()["vasp_scf"]["kmesh"]
        scf_kmesh = self.input_config.get("vasp_scf", {}).get("kmesh", scf_default)
        eledos_kmesh = self.config["vasp_eledos"].get("kmesh")
        if not eledos_kmesh:
            return 1
        return derive_integer_kmesh_multiplier(scf_kmesh, eledos_kmesh)

    def copy_scf_files(self):
        """从配置文件指定的路径拷贝 CHGCAR 和 WAVECAR"""
        chgcar_src = self.config.get("chgcar_path")
        wavecar_src = self.config.get("wavecar_path")
        
        if chgcar_src and os.path.exists(os.path.expanduser(chgcar_src)):
            shutil.copy2(os.path.expanduser(chgcar_src), os.path.join(self.work_dir, "CHGCAR"))
            print(f"已从 {chgcar_src} 拷贝 CHGCAR")
        else:
            print(f"警告: 找不到 CHGCAR 源文件 {chgcar_src}")

        if wavecar_src and os.path.exists(os.path.expanduser(wavecar_src)):
            shutil.copy2(os.path.expanduser(wavecar_src), os.path.join(self.work_dir, "WAVECAR"))
            print(f"已从 {wavecar_src} 拷贝 WAVECAR")
        else:
            print(f"警告: 找不到 WAVECAR 源文件 {wavecar_src}")

    def create_run_script(self):
        """创建一个提交脚本"""
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
            expected_outputs=["OUTCAR", "DOSCAR"]
        )
        print(f"VASP 计算完成，输出已保存至 {self.work_dir}")

    def setup(self):
        """准备所有 VASP 输入文件"""
        try:
            os.makedirs(self.work_dir, exist_ok=True)
            target_poscar = os.path.join(self.work_dir, "POSCAR")
            if self.struct_file:
                shutil.copy2(self.struct_file, target_poscar)
            elif os.path.exists("POSCAR") and os.path.abspath("POSCAR") != os.path.abspath(target_poscar):
                shutil.copy2("POSCAR", target_poscar)

            elements = self.get_elements_from_poscar()
            self.generate_incar()
            self.generate_potcar(elements)
            self.generate_kpoints()
            self.copy_scf_files()
            self.create_run_script()
            print(f"\nDOS 计算文件已在 {self.work_dir} 目录中准备就绪！")
        except Exception as e:
            print(f"错误: {e}")


    def run(self, run_calc=False):
        """向后兼容的方法"""
        self.setup()
        if run_calc:
            self.execute()

def main():
    parser = argparse.ArgumentParser(description="VASP DOS Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_vasp.sh")

    args = parser.parse_args()

    setup = VaspDosSetup(config_file=args.config, struct_file=args.input)
    setup.setup()
    handle_job_action(args.run, args.submit, setup.work_dir, "run_vasp.sh", setup.execute)

if __name__ == "__main__":
    main()
