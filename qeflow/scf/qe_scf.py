#!/usr/bin/env python3
import os
import sys
import copy
import shutil
import argparse
import subprocess
import numpy as np
from public.config_templates import load_config_file
from public.executables import resolve_qe_command
from public.job import add_run_submit_arguments, handle_job_action, write_job_script, execute_commands
from qeflow.common import get_atomic_mass, get_pseudo_entry
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_scf_allowed_keys,
    get_qe_scf_parameters_template,
)

try:
    import tomllib  # Python 3.11+
except ImportError:
    try:
        import toml
    except ImportError:
        print("Error: 'toml' or 'tomllib' (Python 3.11+) is required.")
        print("Please install it using: pip install toml")
        sys.exit(1)

def get_formula(elements, counts):
    formula = ""
    for el, count in zip(elements, counts):
        formula += el
        if count >= 1:
            formula += str(count)
    return formula

class QESetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.work_dir = "qe_scf"
        self.parameter_template = get_qe_scf_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_qe_config_overrides(
            self.parameter_template, self.input_config, get_qe_scf_allowed_keys()
        )
        self.struct_file = struct_file
        self.cli_config = copy.deepcopy(self.config["qe_scf"])
        self.k_points_config = self.cli_config.get("k_points", {})
        self.qe_params = {
            "CONTROL": copy.deepcopy(self.cli_config["CONTROL"]),
            "SYSTEM": copy.deepcopy(self.cli_config["SYSTEM"]),
            "ELECTRONS": copy.deepcopy(self.cli_config["ELECTRONS"]),
        }

    def parse_poscar(self, poscar_path):
        """从 POSCAR 中读取结构信息"""
        if not os.path.exists(poscar_path):
            raise FileNotFoundError(f"找不到 {poscar_path} 文件")
        
        with open(poscar_path, "r") as f:
            lines = f.readlines()
            
        # 第2行: 缩放因子
        scale = float(lines[1].strip())
        
        # 第3-5行: 晶格矢量
        lattice = []
        for i in range(2, 5):
            lattice.append([float(x) * scale for x in lines[i].split()])
            
        # 第6行: 元素名称
        elements = lines[5].split()
        if elements[0].isdigit():
            # 简单处理 VASP 4 格式，假设元素名在第1行注释中
            print("警告: POSCAR 可能是 VASP 4 格式，尝试从第1行提取元素。")
            elements = lines[0].split()
            if not elements:
                raise ValueError("无法在 POSCAR 中找到元素名称。")
        
        # 第7行: 原子数量
        counts = [int(x) for x in lines[6].split()]
        
        # 第8行: 坐标类型 (Direct 或 Cartesian)
        coord_line = lines[7].strip()
        if coord_line.lower().startswith('s'): # Selective dynamics
            coord_line = lines[8].strip()
            start_line = 9
        else:
            start_line = 8
            
        # 第9行开始: 坐标
        positions = []
        total_atoms = sum(counts)
        for i in range(total_atoms):
            positions.append(lines[start_line + i].split()[:3])
        
        pos_array = np.array(positions, dtype=float)
        # 如果是笛卡尔坐标，也需要乘以缩放因子
        if not coord_line.lower().startswith('d'):
            pos_array *= scale
            
        return {
            "lattice": np.array(lattice),
            "elements": elements,
            "counts": counts,
            "coord_type": coord_line,
            "positions": pos_array
        }

    def get_kpoints(self, lattice, kmesh):
        """计算 K 点网格"""
        if isinstance(kmesh, list) and len(kmesh) == 3:
            return kmesh
        
        # 计算倒格矢长度
        b = 2 * np.pi * np.linalg.inv(lattice).T
        b_lengths = np.linalg.norm(b, axis=1)
        
        # 如果 kmesh 是密度 (spacing)
        # 默认 spacing 0.04 * 2pi / Angstrom
        spacing = kmesh if isinstance(kmesh, float) else 0.04
        kpts = [int(max(1, np.ceil(l / (spacing * 2 * np.pi)))) for l in b_lengths]
        return kpts

    def generate_qe_input(self, struct_info):
        """生成 QE pw.x 输入文件"""
        print(f"正在生成 scf.in...")
        
        formula = get_formula(struct_info["elements"], struct_info["counts"])
        self.qe_params["CONTROL"]["prefix"] = f"'{formula}'"
        pp_dir = self.config.get("pp_dir")
        if pp_dir:
            self.qe_params["CONTROL"]["pseudo_dir"] = f"'{os.path.expanduser(str(pp_dir))}'"

        # 3. 处理 pseudo_dir 中的 ~ 符号 (QE 不支持 ~)
        if "CONTROL" in self.qe_params:
            pdir = self.qe_params["CONTROL"].get("pseudo_dir", "").strip("'").strip('"')
            if pdir.startswith("~"):
                pdir = os.path.expanduser(pdir)
                self.qe_params["CONTROL"]["pseudo_dir"] = f"'{pdir}'"

        qe_input_path = os.path.join(self.work_dir, "scf.in")
        with open(qe_input_path, "w") as f:
            # Namelists
            for section in ["CONTROL", "SYSTEM", "ELECTRONS"]:
                f.write(f"&{section}\n")
                if section == "SYSTEM":
                    f.write(f"  nat  = {struct_info['positions'].shape[0]}\n")
                    f.write(f"  ntyp = {len(struct_info['elements'])}\n")
                
                params = self.qe_params.get(section, {})
                for key, value in params.items():
                    f.write(f"  {key} = {value}\n")
                f.write("/")
                f.write("\n\n")
            
            # ATOMIC_SPECIES
            f.write("ATOMIC_SPECIES\n")
            pseudo_map = self.config.get("pseudo_map", {})
            for el in struct_info["elements"]:
                info = get_pseudo_entry(pseudo_map, el)
                mass = get_atomic_mass(el, info.get("mass"))
                pseudo_file = str(info["pseudo"]).strip()
                f.write(f"  {el:3} {mass:8.4f} {pseudo_file}\n")
            f.write("\n")
            
            # ATOMIC_POSITIONS
            coord_type = struct_info["coord_type"]
            if coord_type.lower().startswith('d'):
                f.write("ATOMIC_POSITIONS {crystal}\n")
            else:
                f.write("ATOMIC_POSITIONS {angstrom}\n")
                
            atom_idx = 0
            for el_idx, el in enumerate(struct_info["elements"]):
                for _ in range(struct_info["counts"][el_idx]):
                    pos = struct_info["positions"][atom_idx]
                    f.write(f"  {el:3} {pos[0]:21.17f} {pos[1]:21.17f} {pos[2]:21.17f}\n")
                    atom_idx += 1
            f.write("\n")
            
            # CELL_PARAMETERS
            f.write("CELL_PARAMETERS {angstrom}\n")
            for vec in struct_info["lattice"]:
                f.write(f"  {vec[0]:21.17f} {vec[1]:21.17f} {vec[2]:21.17f}\n")
            f.write("\n")
            
            # K_POINTS：支持显式 kpoints 或按 kmesh 生成自动网格
            kpoints_list = self.k_points_config.get("kpoints")
            kmesh_val = self.k_points_config.get("kmesh")

            if isinstance(kpoints_list, list) and len(kpoints_list) == 3:
                kpts = kpoints_list
            else:
                kpts = self.get_kpoints(struct_info["lattice"], kmesh_val)

            f.write("K_POINTS automatic\n")
            f.write(f"  {int(kpts[0])} {int(kpts[1])} {int(kpts[2])} 0 0 0\n")

    def create_run_script(self):
        """创建运行脚本"""
        qe_config = self.config.get("qe", {})
        pw_path = resolve_qe_command(qe_config.get("executable_path"), "pw.x")
        slurm_header = self.config.get("slurm", {}).get("header", "")

        run_script_path = os.path.join(self.work_dir, "run_qe.sh")
        write_job_script(
            run_script_path,
            slurm_header,
            [
                "echo 'Job started at' `date`",
                f"{pw_path} < scf.in > scf.out",
                "echo 'Job finished at' `date`"
            ]
        )

    def execute(self):
        """执行 Quantum ESPRESSO 计算"""
        qe_config = self.config.get("qe", {})
        pw_path = resolve_qe_command(qe_config.get("executable_path"), "pw.x")

        execute_commands(
            self.work_dir,
            [f"{pw_path} < scf.in > scf.out"],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 QE 出错"
        )
        print(f"QE 计算完成，输出已保存至 {self.work_dir}/scf.out")

    def setup(self):
        """准备所有输入文件和计算环境"""
        try:
            os.makedirs(self.work_dir, exist_ok=True)

            target_poscar = os.path.join(self.work_dir, "POSCAR")
            if self.struct_file:
                shutil.copy2(self.struct_file, target_poscar)
                print(f"已将 {self.struct_file} 拷贝为 {target_poscar}")
            elif os.path.exists("POSCAR"):
                shutil.copy2("POSCAR", target_poscar)
                print(f"已将当前目录下的 POSCAR 拷贝为 {target_poscar}")
            else:
                raise FileNotFoundError("找不到 POSCAR 文件，请使用 -i 指定或在当前目录准备。")

            struct_info = self.parse_poscar(target_poscar)
            self.generate_qe_input(struct_info)
            self.create_run_script()
            
            # 确保 outdir 目录存在，否则 QE 会报错
            outdir_val = self.qe_params["CONTROL"].get("outdir", "").strip("'").strip('"')
            if outdir_val:
                full_outdir = os.path.join(self.work_dir, outdir_val)
                os.makedirs(full_outdir, exist_ok=True)
                print(f"确保 outdir 存在: {full_outdir}")

            print(f"\n所有 QE 输入文件已在 {self.work_dir} 目录中准备就绪！")
                
        except Exception as e:
            print(f"准备环境出错: {e}")
            raise

def main():
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO SCF Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")

    args = parser.parse_args()
    setup_obj = QESetup(config_file=args.config, struct_file=args.input)

    setup_obj.setup()
    handle_job_action(args.run, args.submit, setup_obj.work_dir, "run_qe.sh", setup_obj.execute)

if __name__ == "__main__":
    main()
