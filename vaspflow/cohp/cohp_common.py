#!/usr/bin/env python3
import os
import shutil
import subprocess
from abc import ABC, abstractmethod

from public.config_templates import load_config_file
from public.executables import resolve_vasp_command
from public.job import execute_commands, write_job_script
from vaspflow.templates import apply_vasp_config_overrides


class BaseVaspCohpSetup(ABC):
    block_name = ""
    work_dir = ""

    def __init__(self, config_file, struct_file, parameter_template, allowed_keys):
        self.parameter_template = parameter_template
        self.input_config = load_config_file(config_file)
        self.config = apply_vasp_config_overrides(
            self.parameter_template, self.input_config, allowed_keys
        )
        self.struct_file = struct_file

    def get_elements_from_poscar(self):
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        if not os.path.exists(poscar_path):
            raise FileNotFoundError(f"找不到 {poscar_path} 文件")

        with open(poscar_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
            return lines[5].split()

    def generate_incar(self):
        final_params = self.config[self.block_name].copy()
        for transient_key in ("kmesh", "scf_kmesh", "kpath_kmesh", "kmesh_scheme"):
            final_params.pop(transient_key, None)

        print("正在生成 INCAR (COHP)...")
        incar_path = os.path.join(self.work_dir, "INCAR")
        with open(incar_path, "w", encoding="utf-8") as f:
            for key, value in final_params.items():
                f.write(f"{key:12} = {value}\n")

    def generate_potcar(self, elements):
        potcar_dir = self.config.get("potcar_dir", "")
        if not potcar_dir:
            raise ValueError("未在配置文件中找到 'potcar_dir'")

        potcar_dir = os.path.expanduser(potcar_dir)
        potcar_content = []
        for element in elements:
            path = os.path.join(potcar_dir, element)
            if not os.path.exists(path):
                raise FileNotFoundError(f"找不到元素 {element} 的 POTCAR")
            with open(path, "r", encoding="utf-8") as f:
                potcar_content.append(f.read())

        potcar_path = os.path.join(self.work_dir, "POTCAR")
        with open(potcar_path, "w", encoding="utf-8") as f:
            f.writelines(potcar_content)
        print(f"POTCAR 已在 {self.work_dir} 中准备就绪。")

    @abstractmethod
    def generate_kpoints(self):
        raise NotImplementedError

    def copy_scf_files(self):
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
        vasp_config = self.config.get("vasp", {})
        vasp_path = resolve_vasp_command(vasp_config.get("executable_path"))
        slurm_header = self.config.get("slurm", {}).get("header", "")

        run_script_path = os.path.join(self.work_dir, "run_vasp.sh")
        write_job_script(
            run_script_path,
            slurm_header,
            [
                "echo 'Job started at' `date`",
                vasp_path,
                "echo 'Job finished at' `date`",
            ],
        )

    def calculate_distances(self):
        try:
            from collections import defaultdict
            from itertools import combinations

            import numpy as np
            from pymatgen.core.structure import Structure
        except ImportError:
            print("\n提示: 未找到 pymatgen 或 numpy，跳过距离计算。")
            print("如果需要查看键长，请安装: pip install pymatgen numpy")
            return

        print("\n正在计算原子间距离...")
        poscar_path = os.path.join(self.work_dir, "POSCAR")
        if not os.path.exists(poscar_path):
            return

        struct = Structure.from_file(poscar_path)
        atom_indices = [idx + 1 for idx, _ in enumerate(struct.sites)]
        pairs = list(combinations(struct.sites, r=2))
        idxs = list(combinations(atom_indices, r=2))

        pairs_dist = defaultdict(list)
        for pair, idx in zip(pairs, idxs):
            distance = struct.lattice.get_all_distances(pair[0].frac_coords, pair[1].frac_coords)[0][0]
            pair_name = f"{pair[0].specie}{idx[0]}-{pair[1].specie}{idx[1]}"
            pairs_dist[np.round(distance, decimals=3)].append(pair_name)

        dist_dat_path = os.path.join(self.work_dir, "distance.dat")
        with open(dist_dat_path, "w", encoding="utf-8") as f:
            for distance, pair_names in sorted(pairs_dist.items()):
                if distance > 6.0:
                    continue
                line = f"distance = {distance:.3f}, Number={len(pair_names)}"
                print(line)
                print("    " + "   ".join(pair_names))
                f.write(line + "\n")
                f.write("    " + "   ".join(pair_names) + "\n\n")

        print(f"\n键长信息已保存至: {dist_dat_path}")

    def execute(self):
        vasp_config = self.config.get("vasp", {})
        vasp_path = resolve_vasp_command(vasp_config.get("executable_path"))

        execute_commands(
            self.work_dir,
            [vasp_path],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 VASP 出错",
            expected_outputs=["OUTCAR", "WAVECAR"],
        )
        print(f"VASP 计算完成，输出已保存至 {self.work_dir}")

    def setup(self):
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
            self.calculate_distances()
            print(f"\nCOHP 计算文件已在 {self.work_dir} 目录中准备就绪！")
        except Exception as exc:
            print(f"错误: {exc}")
            raise

    def run(self, run_calc=False):
        self.setup()
        if run_calc:
            self.execute()


def run_vaspkit(cwd, user_input):
    process = subprocess.Popen(
        ["vaspkit"],
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    process.communicate(input=user_input)
    return "", ""
