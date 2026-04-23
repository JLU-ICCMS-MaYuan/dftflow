#!/usr/bin/env python3
import os
import shutil
import argparse
import subprocess
import sys
import copy
import numpy as np
from public.config_templates import load_config_file
from public.executables import resolve_qe_command
from public.job import add_run_submit_arguments, execute_commands, handle_job_action, write_job_script
from qeflow.common import get_atomic_mass, get_pseudo_entry
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_nscf_allowed_keys,
    get_qe_nscf_parameters_template,
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

def get_formula(elements, counts):
    formula = ""
    for el, count in zip(elements, counts):
        formula += el
        if count >= 1:
            formula += str(count)
    return formula

class QENSCFSetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.work_dir = "qe_nscf"
        self.scf_dir = "qe_scf"
        self.parameter_template = get_qe_nscf_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_qe_config_overrides(
            self.parameter_template, self.input_config, get_qe_nscf_allowed_keys()
        )
        self.struct_file = struct_file
        self.cli_config = copy.deepcopy(self.config["qe_nscf"])
        self.k_points_config = self.cli_config.get("k_points", {})
        self.wan = self.k_points_config.get("wan", False)
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
            
        scale = float(lines[1].strip())
        lattice = []
        for i in range(2, 5):
            lattice.append([float(x) * scale for x in lines[i].split()])
            
        elements = lines[5].split()
        if elements[0].isdigit():
            elements = lines[0].split()
            if not elements:
                raise ValueError("无法在 POSCAR 中找到元素名称。")
        
        counts = [int(x) for x in lines[6].split()]
        
        coord_line = lines[7].strip()
        if coord_line.lower().startswith('s'):
            coord_line = lines[8].strip()
            start_line = 9
        else:
            start_line = 8
            
        positions = []
        total_atoms = sum(counts)
        for i in range(total_atoms):
            positions.append(lines[start_line + i].split()[:3])
        
        pos_array = np.array(positions, dtype=float)
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
        
        b = 2 * np.pi * np.linalg.inv(lattice).T
        b_lengths = np.linalg.norm(b, axis=1)
        
        # NSCF 默认使用更密的网格
        spacing = kmesh if isinstance(kmesh, float) else 0.02
        kpts = [int(max(1, np.ceil(l / (spacing * 2 * np.pi)))) for l in b_lengths]
        return kpts

    def get_kmesh_justlike_kmesh_pl(self, kpoints):
        """
        将均匀网格 [n1, n2, n3] 转化为倒空间均匀网格坐标
        """
        # 获取输入的 n1, n2, n3
        n1, n2, n3 = kpoints

        # 参数检查：确保 n1, n2, n3 都大于 0
        if n1 <= 0:
            print("n1 must be > 0")
            sys.exit()
        if n2 <= 0:
            print("n2 must be > 0")
            sys.exit()
        if n3 <= 0:
            print("n3 must be > 0")
            sys.exit()

        # 计算总的 k 点数量
        totpts = n1 * n2 * n3

        kpoints_coords = []
        if not self.wan: # 前三列写k点倒空间分数坐标，第四列写其权重
            for x in range(n1):
                for y in range(n2):
                    for z in range(n3):
                        # 格式化输出 k 点信息
                        kpoints_coords.append(f"{x/n1:21.17f}{y/n2:21.17f}{z/n3:21.17f}{1/totpts:14.6e}")
        else:  # 只写前三列写k点倒空间分数坐标
            for x in range(n1):
                for y in range(n2):
                    for z in range(n3):
                        # 格式化输出 k 点信息（没有权重）
                        kpoints_coords.append(f"{x/n1:21.17f}{y/n2:21.17f}{z/n3:21.17f}")
        return kpoints_coords, totpts

    def generate_qe_input(self, struct_info):
        """生成 QE pw.x 输入文件"""
        print(f"正在生成 nscf.in...")
        
        # 1. 自动设置 prefix (由化学配比决定)
        formula = get_formula(struct_info["elements"], struct_info["counts"])
        self.qe_params["CONTROL"]["prefix"] = f"'{formula}'"
        pp_dir = self.config.get("pp_dir")
        if pp_dir:
            self.qe_params["CONTROL"]["pseudo_dir"] = f"'{os.path.expanduser(str(pp_dir))}'"

        self.qe_params["CONTROL"]["prefix"] = f"'{formula}'"

        # 3. 处理 pseudo_dir 中的 ~ 符号
        if "CONTROL" in self.qe_params:
            pdir = self.qe_params["CONTROL"].get("pseudo_dir", "").strip("'").strip('"')
            if pdir.startswith("~"):
                pdir = os.path.expanduser(pdir)
                self.qe_params["CONTROL"]["pseudo_dir"] = f"'{pdir}'"

        qe_input_path = os.path.join(self.work_dir, "nscf.in")
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
                    f.write(f"  {el:3} {pos[0]:12.8f} {pos[1]:12.8f} {pos[2]:12.8f}\n")
                    atom_idx += 1
            f.write("\n")
            
            # CELL_PARAMETERS
            f.write("CELL_PARAMETERS {angstrom}\n")
            for vec in struct_info["lattice"]:
                f.write(f"  {vec[0]:12.8f} {vec[1]:12.8f} {vec[2]:12.8f}\n")
            f.write("\n")
            
            # K_POINTS：仅从 [k_points] 读取
            kpoints_list = self.k_points_config.get("kpoints")
            kmesh_val = self.k_points_config.get("kmesh")

            if kpoints_list is not None:
                final_kmesh = kpoints_list
            elif kmesh_val is not None:
                final_kmesh = self.get_kpoints(struct_info["lattice"], kmesh_val)
            else:
                raise ValueError("k_points 需要提供 kmesh 或 kpoints")

            kpoints_coords, totpts = self.get_kmesh_justlike_kmesh_pl(final_kmesh)
            f.write("K_POINTS crystal\n")
            f.write(f"{totpts}\n")
            for kinfo in kpoints_coords:
                f.write(f" {kinfo}\n")

    def create_run_script(self):
        """创建运行脚本"""
        qe_config = self.config.get("qe", {})
        pw_path = resolve_qe_command(qe_config.get("executable_path"), "pw.x")
        slurm_header = self.config.get("slurm", {}).get("header", "")

        run_script_path = os.path.join(self.work_dir, "run_qe.sh")
        write_job_script(
            run_script_path,
            slurm_header,
            ["echo 'Job started at' `date` ", f"{pw_path} < nscf.in > nscf.out", "echo 'Job finished at' `date` "],
        )

    def execute(self):
        """进入工作目录并执行 Quantum ESPRESSO NSCF 计算"""
        qe_config = self.config.get("qe", {})
        pw_path = resolve_qe_command(qe_config.get("executable_path"), "pw.x")
        execute_commands(
            self.work_dir,
            [f"{pw_path} < nscf.in > nscf.out"],
            error_prefix="执行 QE NSCF 出错",
        )
        print(f"QE NSCF 计算完成，输出已保存至 {self.work_dir}/nscf.out")

    def setup(self):
        """准备 NSCF 环境，包括拷贝 SCF 数据"""
        try:
            os.makedirs(self.work_dir, exist_ok=True)

            # 1. 确定结构文件路径 (只在输入或当前目录查找)
            if self.struct_file:
                struct_path = self.struct_file
            elif os.path.exists("POSCAR"):
                struct_path = "POSCAR"
            else:
                raise FileNotFoundError("找不到结构文件。请在当前目录准备 POSCAR 或使用 -i 指定。")

            # 2. 解析结构并自动设置 prefix
            struct_info = self.parse_poscar(struct_path)
            formula = get_formula(struct_info["elements"], struct_info["counts"])
            self.qe_params["CONTROL"]["prefix"] = f"'{formula}'"

            # 3. prefix 是运行时派生值

            # 4. 拷贝 SCF 生成的数据 (此时 prefix 已确定)
            outdir_val = self.qe_params["CONTROL"]["outdir"].strip("'").strip('"')
            prefix_val = self.qe_params["CONTROL"]["prefix"].strip("'").strip('"')

            save_dir_name = f"{prefix_val}.save"
            src_save_path = os.path.join(self.scf_dir, outdir_val, save_dir_name)
            dst_save_path = os.path.join(self.work_dir, outdir_val, save_dir_name)

            if os.path.exists(src_save_path):
                os.makedirs(dst_save_path, exist_ok=True)
                needed_files = ["data-file-schema.xml", "charge-density.dat", "paw.txt"]
                for filename in needed_files:
                    src_file = os.path.join(src_save_path, filename)
                    dst_file = os.path.join(dst_save_path, filename)
                    if os.path.exists(src_file):
                        shutil.copy2(src_file, dst_file)
                        print(f"已拷贝 SCF 数据: {filename}")
                print(f"已从 {src_save_path} 准备好 NSCF 所需的关键数据。")
            else:
                print(f"警告: 找不到 SCF 的数据目录 {src_save_path}，计算可能会失败。")

            # 5. 生成输入文件
            self.generate_qe_input(struct_info)
            self.create_run_script()
            
            # 确保 outdir 目录存在
            if outdir_val:
                full_outdir = os.path.join(self.work_dir, outdir_val)
                os.makedirs(full_outdir, exist_ok=True)
                print(f"确保 outdir 存在: {full_outdir}")

            print(f"\n所有 QE NSCF 输入文件及数据已在 {self.work_dir} 目录中准备就绪！")
                
        except Exception as e:
            print(f"准备 NSCF 环境出错: {e}")
            raise

def main():
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO NSCF Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")
    
    args = parser.parse_args()
    setup_obj = QENSCFSetup(config_file=args.config, struct_file=args.input)
    
    # 1. 环境准备 (包含数据拷贝)
    setup_obj.setup()
    handle_job_action(args.run, args.submit, setup_obj.work_dir, "run_qe.sh", setup_obj.execute)

if __name__ == "__main__":
    main()
