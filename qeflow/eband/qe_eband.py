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
from qeflow.common import normalize_qe_config_paths, resolve_atomic_species_entries
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_eband_allowed_keys,
    get_qe_eband_parameters_template,
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


def parse_kpath_segments(path, tol=1.0e-6):
    points = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) == 4:
                try:
                    kx, ky, kz = map(float, parts[:3])
                except ValueError:
                    continue
                label = parts[3]
                points.append(([kx, ky, kz], label))
            else:
                continue

    if len(points) % 2 != 0:
        raise ValueError(f"KPATH.in 格式异常，点数为奇数: {len(points)}")

    segments = []
    for i in range(0, len(points), 2):
        (k1, l1), (k2, l2) = points[i], points[i + 1]
        if not segments:
            segments.append([(k1, l1), (k2, l2)])
            continue
        prev_end = segments[-1][-1][0]
        if (
            abs(prev_end[0] - k1[0]) < tol
            and abs(prev_end[1] - k1[1]) < tol
            and abs(prev_end[2] - k1[2]) < tol
        ):
            segments[-1].append((k2, l2))
        else:
            segments.append([(k1, l1), (k2, l2)])

    return segments


def flatten_segments(segments):
    path_points = []
    for segment in segments:
        for idx, (coords, label) in enumerate(segment):
            if idx == 0 and path_points and path_points[-1][0] == coords and path_points[-1][1] == label:
                continue
            path_points.append((coords, label))
    return path_points


class QEEBandSetup:
    def __init__(self, config_file="input.toml", struct_file=None):
        self.work_dir = "qe_eband"
        self.scf_dir = "qe_scf"
        self.parameter_template = get_qe_eband_parameters_template()
        self.input_config = load_config_file(config_file)
        self.config = apply_qe_config_overrides(
            self.parameter_template, self.input_config, get_qe_eband_allowed_keys()
        )
        self.config = normalize_qe_config_paths(self.config, config_file=config_file)
        self.struct_file = struct_file
        self.cli_config = copy.deepcopy(self.config["qe_eband"])
        self.k_points_config = self.cli_config.get("k_points", {})
        self.kpath_points = int(self.k_points_config.get("kpath_points", 100))
        self.kpath_file = self.k_points_config.get("kpath_file", "KPATH.in")
        self.qe_params = {
            "CONTROL": copy.deepcopy(self.cli_config["CONTROL"]),
            "SYSTEM": copy.deepcopy(self.cli_config["SYSTEM"]),
            "ELECTRONS": copy.deepcopy(self.cli_config["ELECTRONS"]),
        }

    def parse_poscar(self, poscar_path):
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
        if coord_line.lower().startswith("s"):
            coord_line = lines[8].strip()
            start_line = 9
        else:
            start_line = 8

        positions = []
        total_atoms = sum(counts)
        for i in range(total_atoms):
            positions.append(lines[start_line + i].split()[:3])

        pos_array = np.array(positions, dtype=float)
        if not coord_line.lower().startswith("d"):
            pos_array *= scale

        return {
            "lattice": np.array(lattice),
            "elements": elements,
            "counts": counts,
            "coord_type": coord_line,
            "positions": pos_array,
        }

    def run_vaspkit(self):
        print("正在调用 vaspkit 生成能带路径 (303)...")
        try:
            subprocess.run(
                ["vaspkit"],
                input="3\n303\n",
                text=True,
                check=True,
                cwd=self.work_dir,
            )
        except Exception as e:
            raise RuntimeError(f"vaspkit 运行失败: {e}")

    def write_kpath_labels(self, segments, lattice, prefix):
        bohr_to_ang = 0.52917721092
        lattice_bohr = lattice / bohr_to_ang
        alat_bohr = float(np.linalg.norm(lattice_bohr[0]))
        # QE 的 scf.out 中倒格子以 2π/alat 为单位；此处按同一单位自算
        recip_lat = np.linalg.inv(lattice_bohr).T * alat_bohr

        label_points = []
        for seg_idx, segment in enumerate(segments):
            if seg_idx == 0:
                label_points.extend(segment)
                continue
            prev_end_coords, prev_end_label = segments[seg_idx - 1][-1]
            replaced = [(prev_end_coords, prev_end_label)]
            replaced.extend(segment[1:])
            label_points.extend(replaced)

        total_dist = 0.0
        last_cart = None
        label_path = os.path.join(self.work_dir, f"{prefix}_gun_band.labelinfo.dat")
        with open(label_path, "w") as f:
            f.write("# label dist kx ky kz\n")
            for coords, label in label_points:
                cart = np.dot(coords, recip_lat)
                if last_cart is not None:
                    total_dist += float(np.linalg.norm(cart - last_cart))
                f.write(
                    f"{label:8} {total_dist:12.6f} {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f}\n"
                )
                last_cart = cart
        print(f"已生成高对称点坐标: {label_path}")

    def compute_w90_recip_lattice(self, lattice):
        volume = float(np.linalg.det(lattice))
        if abs(volume) < 1.0e-12:
            raise ValueError("晶胞体积过小，无法计算倒格子。")
        return 2.0 * np.pi * np.linalg.inv(lattice).T

    def write_kpath_labels_with_index(self, segments, lattice, prefix, weights):
        recip_lat = self.compute_w90_recip_lattice(lattice)

        label_points = []
        jump_starts = []
        for seg_idx, segment in enumerate(segments):
            for pt_idx, (coords, label) in enumerate(segment):
                label_points.append((coords, label))
                jump_starts.append(seg_idx > 0 and pt_idx == 0)

        if len(label_points) != len(weights):
            raise ValueError("高对称点数量与 K_POINTS 权重数量不一致。")

        indices = [1]
        for i in range(1, len(weights)):
            indices.append(indices[-1] + int(weights[i - 1]))

        total_dist = 0.0
        last_cart = None
        label_path = os.path.join(self.work_dir, f"{prefix}_band.labelinfo.dat")
        with open(label_path, "w") as f:
            for (idx, (coords, label), is_jump) in zip(indices, label_points, jump_starts):
                cart = np.dot(coords, recip_lat)
                if last_cart is not None and not is_jump:
                    total_dist += float(np.linalg.norm(cart - last_cart))
                f.write(
                    f"{label:8} {idx:12d} {total_dist:16.10f} {coords[0]:16.10f} {coords[1]:16.10f} {coords[2]:16.10f}\n"
                )
                last_cart = cart
        print(f"已生成高对称点坐标: {label_path}")

    def generate_qe_input(self, struct_info):
        print("正在生成 eband.in ...")

        formula = get_formula(struct_info["elements"], struct_info["counts"])
        self.qe_params["CONTROL"]["prefix"] = f"'{formula}'"
        pp_dir = self.config.get("pp_dir")
        if pp_dir:
            self.qe_params["CONTROL"]["pseudo_dir"] = f"'{os.path.expanduser(str(pp_dir))}'"

        if "CONTROL" in self.qe_params:
            pdir = self.qe_params["CONTROL"].get("pseudo_dir", "").strip("'").strip('"')
            if pdir.startswith("~"):
                pdir = os.path.expanduser(pdir)
                self.qe_params["CONTROL"]["pseudo_dir"] = f"'{pdir}'"

        qe_input_path = os.path.join(self.work_dir, "eband.in")
        with open(qe_input_path, "w") as f:
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

            f.write("ATOMIC_SPECIES\n")
            for el, mass, pseudo_file in resolve_atomic_species_entries(
                self.config, struct_info["elements"]
            ):
                f.write(f"  {el:3} {mass:8.4f} {pseudo_file}\n")
            f.write("\n")

            coord_type = struct_info["coord_type"]
            if coord_type.lower().startswith("d"):
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

            f.write("CELL_PARAMETERS {angstrom}\n")
            for vec in struct_info["lattice"]:
                f.write(f"  {vec[0]:12.8f} {vec[1]:12.8f} {vec[2]:12.8f}\n")
            f.write("\n")

            kpath_path = self.kpath_file
            if not os.path.isabs(kpath_path):
                kpath_path = os.path.join(self.work_dir, kpath_path)

            if not os.path.exists(kpath_path):
                raise FileNotFoundError(f"找不到 KPATH.in: {kpath_path}")

            segments = parse_kpath_segments(kpath_path)
            if not segments:
                raise ValueError("KPATH.in 未解析到任何高对称点。")

            for idx, segment in enumerate(segments, start=1):
                labels = "-".join(label for _coords, label in segment)
                print(f"[kpath] segment {idx}: {labels}")

            path_points = flatten_segments(segments)
            self.write_kpath_labels(segments, struct_info["lattice"], formula)

            kpoint_lines = []
            for seg_idx, segment in enumerate(segments):
                for pt_idx, (coords, label) in enumerate(segment):
                    is_last_point = pt_idx == len(segment) - 1
                    is_jump = is_last_point and seg_idx < len(segments) - 1
                    is_final_point = is_last_point and seg_idx == len(segments) - 1
                    if is_jump or is_final_point:
                        weight = 1
                    else:
                        weight = self.kpath_points
                    kpoint_lines.append((coords, label, weight))

            weights = [weight for _coords, _label, weight in kpoint_lines]
            self.write_kpath_labels_with_index(segments, struct_info["lattice"], formula, weights)

            f.write("K_POINTS crystal_b\n")
            f.write(f"{len(kpoint_lines)}\n")
            for coords, label, weight in kpoint_lines:
                f.write(
                    f"  {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f} {weight:6d} ! {label}\n"
                )

        return formula

    def generate_banddata_inputs(self, prefix):
        outdir_val = self.qe_params["CONTROL"].get("outdir", "'./out'").strip("'").strip('"')
        eband_cfg = self.cli_config.get("eband", {})
        bands_filband = f"{prefix}_band"
        proj_filproj = f"{prefix}_band"
        lsym = eband_cfg.get("lsym", False)

        bands_input_path = os.path.join(self.work_dir, "elebanddata.in")
        with open(bands_input_path, "w") as f:
            f.write("&BANDS\n")
            f.write(f" prefix = '{prefix}',\n")
            f.write(f" outdir = '{outdir_val}',\n")
            f.write(f" filband = '{bands_filband}',\n")
            f.write(" lp = .true.\n")
            f.write("/\n")

        proj_input_path = os.path.join(self.work_dir, "elebandprojdata.in")
        with open(proj_input_path, "w") as f:
            f.write("&projwfc\n")
            f.write(f" prefix = '{prefix}',\n")
            f.write(f" outdir = '{outdir_val}',\n")
            f.write(f" lsym = .{str(lsym).lower()}.,\n")
            f.write(f" filproj = '{proj_filproj}'\n")
            f.write("/\n")

    def create_run_script(self):
        qe_config = self.config.get("qe", {})
        qe_bin = qe_config.get("executable_path")
        pw_path = resolve_qe_command(qe_bin, "pw.x")
        bands_cfg = self.cli_config.get("bands", {})
        bands_exec = resolve_qe_command(bands_cfg.get("executable_path", qe_bin), "bands.x")
        bands_args = bands_cfg.get("args", "-pd .true.")
        proj_cfg = self.cli_config.get("projwfc", {})
        proj_exec = resolve_qe_command(proj_cfg.get("executable_path", qe_bin), "projwfc.x")
        proj_args = proj_cfg.get("args", "-pd .true.")
        slurm_header = self.config.get("slurm", {}).get("header", "")
        bands_cmd = f"{bands_exec} {bands_args}".strip() if bands_args else bands_exec
        proj_cmd = f"{proj_exec} {proj_args}".strip() if proj_args else proj_exec

        run_script_path = os.path.join(self.work_dir, "run_qe.sh")
        write_job_script(
            run_script_path,
            slurm_header,
            [
                "echo 'Job started at' `date` ",
                f"{pw_path} < eband.in > eband.out",
                f"{bands_cmd} < elebanddata.in > elebanddata.out",
                f"{proj_cmd} < elebandprojdata.in > elebandprojdata.out",
                "echo 'Job finished at' `date` ",
            ],
        )

    def execute(self):
        qe_config = self.config.get("qe", {})
        qe_bin = qe_config.get("executable_path")
        pw_path = resolve_qe_command(qe_bin, "pw.x")
        bands_cfg = self.cli_config.get("bands", {})
        bands_exec = resolve_qe_command(bands_cfg.get("executable_path", qe_bin), "bands.x")
        bands_args = bands_cfg.get("args", "-pd .true.")
        proj_cfg = self.cli_config.get("projwfc", {})
        proj_exec = resolve_qe_command(proj_cfg.get("executable_path", qe_bin), "projwfc.x")
        proj_args = proj_cfg.get("args", "-pd .true.")
        bands_cmd = f"{bands_exec} {bands_args}".strip() if bands_args else bands_exec
        proj_cmd = f"{proj_exec} {proj_args}".strip() if proj_args else proj_exec
        execute_commands(
            self.work_dir,
            [
                f"{pw_path} < eband.in > eband.out",
                f"{bands_cmd} < elebanddata.in > elebanddata.out",
                f"{proj_cmd} < elebandprojdata.in > elebandprojdata.out",
            ],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 QE eband 出错",
        )
        print(f"QE eband 计算完成，输出已保存至 {self.work_dir}")

    def setup(self):
        try:
            os.makedirs(self.work_dir, exist_ok=True)

            if self.struct_file:
                struct_path = self.struct_file
            elif os.path.exists("POSCAR"):
                struct_path = "POSCAR"
            else:
                raise FileNotFoundError("找不到结构文件。请在当前目录准备 POSCAR 或使用 -i 指定。")

            target_poscar = os.path.join(self.work_dir, "POSCAR")
            if os.path.abspath(struct_path) != os.path.abspath(target_poscar):
                shutil.copy2(struct_path, target_poscar)

            struct_info = self.parse_poscar(struct_path)
            formula = get_formula(struct_info["elements"], struct_info["counts"])
            self.qe_params["CONTROL"]["prefix"] = f"'{formula}'"

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
                print(f"已从 {src_save_path} 准备好 eband 所需的关键数据。")
            else:
                print(f"警告: 找不到 SCF 的数据目录 {src_save_path}，计算可能会失败。")

            self.run_vaspkit()

            prefix = self.generate_qe_input(struct_info)
            self.generate_banddata_inputs(prefix)
            self.create_run_script()

            if outdir_val:
                full_outdir = os.path.join(self.work_dir, outdir_val)
                os.makedirs(full_outdir, exist_ok=True)
                print(f"确保 outdir 存在: {full_outdir}")

            print(f"\n所有 QE eband 输入文件及数据已在 {self.work_dir} 目录中准备就绪！")

        except Exception as e:
            print(f"准备 eband 环境出错: {e}")
            raise


def main():
    parser = argparse.ArgumentParser(description="Quantum ESPRESSO eband Setup Script")
    parser.add_argument("-i", "--input", help="输入结构文件 (POSCAR 格式)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径")
    add_run_submit_arguments(parser, "生成文件后直接执行计算", "生成文件后提交 run_qe.sh")

    args = parser.parse_args()
    setup_obj = QEEBandSetup(config_file=args.config, struct_file=args.input)

    setup_obj.setup()
    handle_job_action(args.run, args.submit, setup_obj.work_dir, "run_qe.sh", setup_obj.execute)


if __name__ == "__main__":
    main()
