#!/usr/bin/env python3
import copy
import os
import shutil
from dataclasses import dataclass
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple

import numpy as np

from public.config_templates import load_config_file
from public.executables import resolve_qe_command
from public.job import execute_commands, write_job_script
from qeflow.common import normalize_qe_config_paths, resolve_atomic_species_entries
from qeflow.qe_input.kpath import (
    compute_reciprocal_lattice,
    parse_kpath_segments,
    run_vaspkit_for_kpath,
)
from qeflow.qe_input.lattice import (
    validate_no_manual_lattice_params,
    write_atomic_positions,
    write_cell_parameters_if_needed,
    write_system_lattice,
)
from qeflow.qe_input.structure import get_formula, get_kpoints_from_spacing, parse_poscar
from qeflow.templates import (
    apply_qe_config_overrides,
    get_qe_eband_allowed_keys,
    get_qe_eband_parameters_template,
    get_qe_eledos_allowed_keys,
    get_qe_eledos_parameters_template,
    get_qe_nscf_allowed_keys,
    get_qe_nscf_parameters_template,
    get_qe_scf_allowed_keys,
    get_qe_scf_parameters_template,
)


PW_SECTIONS = ("CONTROL", "SYSTEM", "ELECTRONS")


@dataclass(frozen=True)
class QECalculationSpec:
    kind: str
    work_dir: str
    config_section: str
    template_factory: Callable[[], Dict[str, Any]]
    allowed_keys_factory: Callable[[], Iterable[str]]
    input_filename: str
    success_message: str
    error_prefix: str
    input_writer: Callable[["QECalculationSetup"], None]
    run_commands_factory: Callable[["QECalculationSetup"], List[str]]
    dependency_preparer: Optional[Callable[["QECalculationSetup"], None]] = None


class QECalculationSetup:
    def __init__(self, spec: QECalculationSpec, config_file: str = "input.toml", struct_file: Optional[str] = None):
        self.spec = spec
        self.work_dir = spec.work_dir
        self.config_file = config_file
        self.struct_file = struct_file
        self.input_config = load_config_file(config_file)
        validate_no_manual_lattice_params(self.input_config)
        self.config = apply_qe_config_overrides(
            spec.template_factory(),
            self.input_config,
            spec.allowed_keys_factory(),
        )
        self.config = normalize_qe_config_paths(self.config, config_file=config_file)
        self.cli_config = copy.deepcopy(self.config[spec.config_section])
        self.qe_params = {
            section: copy.deepcopy(self.cli_config.get(section, {}))
            for section in PW_SECTIONS
        }
        self.struct_info: Optional[Dict[str, Any]] = None
        self.prefix = ""

    def prepare(self) -> None:
        try:
            os.makedirs(self.work_dir, exist_ok=True)
            struct_path = self._prepare_structure_file()
            self.struct_info = parse_poscar(struct_path)
            self.prefix = get_formula(self.struct_info["elements"], self.struct_info["counts"])
            if "CONTROL" in self.qe_params:
                self.qe_params["CONTROL"]["prefix"] = f"'{self.prefix}'"
                pp_dir = self.config.get("pp_dir")
                if pp_dir and self.spec.kind in {"scf", "nscf", "eband"}:
                    self.qe_params["CONTROL"]["pseudo_dir"] = f"'{os.path.expanduser(str(pp_dir))}'"
                self._expand_control_pseudo_dir()
            if self.spec.dependency_preparer:
                self.spec.dependency_preparer(self)
            self.spec.input_writer(self)
            self.write_run_script()
            self._ensure_outdir()
            print(f"\n所有 QE {self.spec.kind} 输入文件及数据已在 {self.work_dir} 目录中准备就绪！")
        except Exception as exc:
            print(f"准备 QE {self.spec.kind} 环境出错: {exc}")
            raise

    def execute(self) -> None:
        execute_commands(
            self.work_dir,
            self.spec.run_commands_factory(self),
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix=self.spec.error_prefix,
        )
        print(self.spec.success_message)

    def run(self) -> None:
        self.execute()

    def submit(self) -> None:
        from public.job import submit_script

        submit_script(self.work_dir, "run_qe.sh")

    def write_run_script(self) -> None:
        write_job_script(
            os.path.join(self.work_dir, "run_qe.sh"),
            str(self.config.get("slurm", {}).get("header", "")),
            [
                "echo 'Job started at' `date` ",
                *self.spec.run_commands_factory(self),
                "echo 'Job finished at' `date` ",
            ],
        )

    def _prepare_structure_file(self) -> str:
        if self.struct_file:
            struct_path = self.struct_file
        elif os.path.exists("POSCAR"):
            struct_path = "POSCAR"
        else:
            raise FileNotFoundError("找不到结构文件。请在当前目录准备 POSCAR 或使用 -i 指定。")
        target_poscar = os.path.join(self.work_dir, "POSCAR")
        if os.path.abspath(struct_path) != os.path.abspath(target_poscar):
            shutil.copy2(struct_path, target_poscar)
        return target_poscar

    def _expand_control_pseudo_dir(self) -> None:
        pdir = str(self.qe_params["CONTROL"].get("pseudo_dir", "")).strip("'").strip('"')
        if pdir.startswith("~"):
            self.qe_params["CONTROL"]["pseudo_dir"] = f"'{os.path.expanduser(pdir)}'"

    def _ensure_outdir(self) -> None:
        outdir_val = self.control_outdir
        if outdir_val:
            os.makedirs(os.path.join(self.work_dir, outdir_val), exist_ok=True)
            print(f"确保 outdir 存在: {os.path.join(self.work_dir, outdir_val)}")

    @property
    def control_outdir(self) -> str:
        return str(self.qe_params.get("CONTROL", {}).get("outdir", "'./tmp'")).strip("'").strip('"')

    @property
    def qe_bin(self) -> Any:
        return self.config.get("qe", {}).get("executable_path")


def create_qe_calculation_setup(
    kind: str,
    config_file: str = "input.toml",
    struct_file: Optional[str] = None,
) -> QECalculationSetup:
    if kind not in SPECS:
        allowed = ", ".join(sorted(SPECS))
        raise ValueError(f"未知 QE Calculation Setup 类型: {kind}，允许值: {allowed}")
    return QECalculationSetup(SPECS[kind], config_file=config_file, struct_file=struct_file)


def _write_pw_input(setup: QECalculationSetup, filename: str, kpoints_writer: Callable[[QECalculationSetup, Any], None], precision: int = 17) -> None:
    if setup.struct_info is None:
        raise ValueError("缺少结构信息，无法写 QE 输入。")
    path = os.path.join(setup.work_dir, filename)
    with open(path, "w", encoding="utf-8") as handle:
        for section in PW_SECTIONS:
            handle.write(f"&{section}\n")
            if section == "SYSTEM":
                handle.write(f"  nat  = {setup.struct_info['positions'].shape[0]}\n")
                handle.write(f"  ntyp = {len(setup.struct_info['elements'])}\n")
                write_system_lattice(handle, setup.config, setup.struct_info)
            for key, value in setup.qe_params.get(section, {}).items():
                handle.write(f"  {key} = {value}\n")
            handle.write("/\n\n")
        handle.write("ATOMIC_SPECIES\n")
        for element, mass, pseudo_file in resolve_atomic_species_entries(
            setup.config,
            setup.struct_info["elements"],
        ):
            handle.write(f"  {element:3} {mass:8.4f} {pseudo_file}\n")
        handle.write("\n")
        write_atomic_positions(handle, setup.struct_info, precision=precision)
        write_cell_parameters_if_needed(handle, setup.config, setup.struct_info, precision=precision)
        kpoints_writer(setup, handle)


def _write_scf_inputs(setup: QECalculationSetup) -> None:
    print("正在生成 scf.in...")
    _write_pw_input(setup, "scf.in", _write_scf_kpoints, precision=17)


def _write_scf_kpoints(setup: QECalculationSetup, handle) -> None:
    k_points_config = setup.cli_config.get("k_points", {})
    kpoints = k_points_config.get("kpoints")
    if isinstance(kpoints, list) and len(kpoints) == 3:
        kmesh = [int(kpoints[0]), int(kpoints[1]), int(kpoints[2])]
    else:
        kmesh = get_kpoints_from_spacing(setup.struct_info["lattice"], k_points_config.get("kmesh"), 0.04)
    handle.write("K_POINTS automatic\n")
    handle.write(f"  {int(kmesh[0])} {int(kmesh[1])} {int(kmesh[2])} 0 0 0\n")


def _write_nscf_inputs(setup: QECalculationSetup) -> None:
    print("正在生成 nscf.in...")
    _write_pw_input(setup, "nscf.in", _write_nscf_kpoints, precision=8)


def _write_nscf_kpoints(setup: QECalculationSetup, handle) -> None:
    k_points_config = setup.cli_config.get("k_points", {})
    kpoints = k_points_config.get("kpoints")
    kmesh = kpoints if kpoints is not None else get_kpoints_from_spacing(
        setup.struct_info["lattice"],
        k_points_config.get("kmesh"),
        0.02,
    )
    if kmesh is None:
        raise ValueError("k_points 需要提供 kmesh 或 kpoints")
    coords, total = _uniform_crystal_kpoints(kmesh, bool(k_points_config.get("wan", False)))
    handle.write("K_POINTS crystal\n")
    handle.write(f"{total}\n")
    for line in coords:
        handle.write(f" {line}\n")


def _uniform_crystal_kpoints(kpoints: Sequence[int], wan: bool) -> Tuple[List[str], int]:
    n1, n2, n3 = [int(value) for value in kpoints]
    if n1 <= 0 or n2 <= 0 or n3 <= 0:
        raise ValueError("kpoints 中的 n1/n2/n3 必须大于 0")
    total = n1 * n2 * n3
    lines = []
    for x in range(n1):
        for y in range(n2):
            for z in range(n3):
                if wan:
                    lines.append(f"{x/n1:21.17f}{y/n2:21.17f}{z/n3:21.17f}")
                else:
                    lines.append(f"{x/n1:21.17f}{y/n2:21.17f}{z/n3:21.17f}{1/total:14.6e}")
    return lines, total


def _write_eband_inputs(setup: QECalculationSetup) -> None:
    print("正在生成 eband.in ...")
    _write_pw_input(setup, "eband.in", _write_eband_kpoints, precision=8)
    _write_banddata_inputs(setup)


def _write_eband_kpoints(setup: QECalculationSetup, handle) -> None:
    k_points_config = setup.cli_config.get("k_points", {})
    kpath_points = int(k_points_config.get("kpath_points", 100))
    kpath_file = str(k_points_config.get("kpath_file", "KPATH.in"))
    kpath_path = kpath_file if os.path.isabs(kpath_file) else os.path.join(setup.work_dir, kpath_file)
    if not os.path.exists(kpath_path):
        raise FileNotFoundError(f"找不到 KPATH.in: {kpath_path}")
    segments = parse_kpath_segments(kpath_path)
    if not segments:
        raise ValueError("KPATH.in 未解析到任何高对称点。")
    for index, segment in enumerate(segments, start=1):
        labels = "-".join(label for _coords, label in segment)
        print(f"[kpath] segment {index}: {labels}")
    _write_gun_kpath_labels(setup.work_dir, setup.prefix, segments, setup.struct_info["lattice"])
    kpoint_lines = []
    for seg_idx, segment in enumerate(segments):
        for point_idx, (coords, label) in enumerate(segment):
            is_last_point = point_idx == len(segment) - 1
            is_jump = is_last_point and seg_idx < len(segments) - 1
            is_final_point = is_last_point and seg_idx == len(segments) - 1
            weight = 1 if is_jump or is_final_point else kpath_points
            kpoint_lines.append((coords, label, weight))
    _write_indexed_kpath_labels(
        setup.work_dir,
        setup.prefix,
        segments,
        setup.struct_info["lattice"],
        [weight for _coords, _label, weight in kpoint_lines],
    )
    handle.write("K_POINTS crystal_b\n")
    handle.write(f"{len(kpoint_lines)}\n")
    for coords, label, weight in kpoint_lines:
        handle.write(f"  {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f} {weight:6d} ! {label}\n")


def _write_gun_kpath_labels(work_dir: str, prefix: str, segments, lattice: Sequence[Sequence[float]]) -> str:
    lattice_bohr = np.asarray(lattice, dtype=float) / 0.52917721092
    alat_bohr = float(np.linalg.norm(lattice_bohr[0]))
    reciprocal = np.linalg.inv(lattice_bohr).T * alat_bohr
    label_points = []
    for seg_idx, segment in enumerate(segments):
        if seg_idx == 0:
            label_points.extend(segment)
            continue
        previous_end_coords, previous_end_label = segments[seg_idx - 1][-1]
        label_points.extend([(previous_end_coords, previous_end_label), *segment[1:]])
    total_dist = 0.0
    last_cart = None
    label_path = os.path.join(work_dir, f"{prefix}_gun_band.labelinfo.dat")
    with open(label_path, "w", encoding="utf-8") as handle:
        handle.write("# label dist kx ky kz\n")
        for coords, label in label_points:
            cart = np.dot(coords, reciprocal)
            if last_cart is not None:
                total_dist += float(np.linalg.norm(cart - last_cart))
            handle.write(f"{label:8} {total_dist:12.6f} {coords[0]:12.8f} {coords[1]:12.8f} {coords[2]:12.8f}\n")
            last_cart = cart
    print(f"已生成高对称点坐标: {label_path}")
    return label_path


def _write_indexed_kpath_labels(
    work_dir: str,
    prefix: str,
    segments,
    lattice: Sequence[Sequence[float]],
    weights: Sequence[int],
) -> str:
    reciprocal = compute_reciprocal_lattice(lattice)
    label_points = []
    jump_starts = []
    for seg_idx, segment in enumerate(segments):
        for point_idx, (coords, label) in enumerate(segment):
            label_points.append((coords, label))
            jump_starts.append(seg_idx > 0 and point_idx == 0)
    if len(label_points) != len(weights):
        raise ValueError("高对称点数量与 K_POINTS 权重数量不一致。")
    indices = [1]
    for index in range(1, len(weights)):
        indices.append(indices[-1] + int(weights[index - 1]))
    total_dist = 0.0
    last_cart = None
    label_path = os.path.join(work_dir, f"{prefix}_band.labelinfo.dat")
    with open(label_path, "w", encoding="utf-8") as handle:
        for idx, (coords, label), is_jump in zip(indices, label_points, jump_starts):
            cart = np.dot(coords, reciprocal)
            if last_cart is not None and not is_jump:
                total_dist += float(np.linalg.norm(cart - last_cart))
            handle.write(f"{label:8} {idx:12d} {total_dist:16.10f} {coords[0]:16.10f} {coords[1]:16.10f} {coords[2]:16.10f}\n")
            last_cart = cart
    print(f"已生成高对称点坐标: {label_path}")
    return label_path


def _write_banddata_inputs(setup: QECalculationSetup) -> None:
    outdir_val = setup.control_outdir
    eband_cfg = setup.cli_config.get("eband", {})
    lsym = eband_cfg.get("lsym", False)
    band_prefix = f"{setup.prefix}_band"
    with open(os.path.join(setup.work_dir, "elebanddata.in"), "w", encoding="utf-8") as handle:
        handle.write("&BANDS\n")
        handle.write(f" prefix = '{setup.prefix}',\n")
        handle.write(f" outdir = '{outdir_val}',\n")
        handle.write(f" filband = '{band_prefix}',\n")
        handle.write(" lp = .true.\n")
        handle.write("/\n")
    with open(os.path.join(setup.work_dir, "elebandprojdata.in"), "w", encoding="utf-8") as handle:
        handle.write("&projwfc\n")
        handle.write(f" prefix = '{setup.prefix}',\n")
        handle.write(f" outdir = '{outdir_val}',\n")
        handle.write(f" lsym = .{str(lsym).lower()}.,\n")
        handle.write(f" filproj = '{band_prefix}'\n")
        handle.write("/\n")


def _write_eledos_inputs(setup: QECalculationSetup) -> None:
    eledos_cfg = setup.cli_config.get("eledos", {})
    outdir_val = setup.control_outdir
    if eledos_cfg.get("outdir"):
        outdir_val = str(eledos_cfg.get("outdir")).strip("'").strip('"')
    fildos = eledos_cfg.get("fildos") or f"{setup.prefix}.tdos"
    filpdos = eledos_cfg.get("filpdos") or f"{setup.prefix}.pdos"
    filproj = eledos_cfg.get("filproj") or f"{setup.prefix}.proj"
    with open(os.path.join(setup.work_dir, "eletdos.in"), "w", encoding="utf-8") as handle:
        handle.write("&dos\n")
        handle.write(f"   prefix = '{setup.prefix}',\n")
        handle.write(f"   outdir = '{outdir_val}',\n")
        handle.write(f"   fildos = '{fildos}',\n")
        handle.write(f"   DeltaE = {eledos_cfg.get('DeltaE')},\n")
        handle.write(f"   emin = {eledos_cfg.get('emin')},\n")
        handle.write(f"   emax = {eledos_cfg.get('emax')}\n")
        handle.write("/\n")
    with open(os.path.join(setup.work_dir, "elepdos.in"), "w", encoding="utf-8") as handle:
        handle.write("&projwfc\n")
        handle.write(f"   prefix = '{setup.prefix}',\n")
        handle.write(f"   outdir = '{outdir_val}',\n")
        handle.write(f"   filpdos = '{filpdos}',\n")
        handle.write(f"   filproj = '{filproj}',\n")
        handle.write(f"   ngauss = {eledos_cfg.get('ngauss')},\n")
        handle.write(f"   DeltaE = {eledos_cfg.get('DeltaE')},\n")
        handle.write(f"   degauss = {eledos_cfg.get('degauss')},\n")
        handle.write(f"   emin = {eledos_cfg.get('emin')},\n")
        handle.write(f"   emax = {eledos_cfg.get('emax')}\n")
        handle.write("/\n")


def _copy_scf_save_files(setup: QECalculationSetup, filenames: Optional[Sequence[str]] = None, copy_tree: bool = False) -> None:
    source = os.path.join("qe_scf", setup.control_outdir, f"{setup.prefix}.save")
    target = os.path.join(setup.work_dir, setup.control_outdir, f"{setup.prefix}.save")
    if not os.path.exists(source):
        print(f"警告: 找不到 SCF 的数据目录 {source}，计算可能会失败。")
        return
    if copy_tree:
        os.makedirs(os.path.dirname(target), exist_ok=True)
        shutil.copytree(source, target, dirs_exist_ok=True)
        print(f"已从 {source} 拷贝 SCF 数据到 {target}")
        return
    os.makedirs(target, exist_ok=True)
    for filename in filenames or ():
        source_file = os.path.join(source, filename)
        target_file = os.path.join(target, filename)
        if os.path.exists(source_file):
            shutil.copy2(source_file, target_file)
            print(f"已拷贝 SCF 数据: {filename}")
    print(f"已从 {source} 准备好 {setup.spec.kind} 所需的关键数据。")


def _prepare_nscf_dependencies(setup: QECalculationSetup) -> None:
    _copy_scf_save_files(setup, ["data-file-schema.xml", "charge-density.dat", "paw.txt"])


def _prepare_eband_dependencies(setup: QECalculationSetup) -> None:
    _copy_scf_save_files(setup, ["data-file-schema.xml", "charge-density.dat", "paw.txt"])
    print("正在调用 vaspkit 生成能带路径 (303)...")
    try:
        run_vaspkit_for_kpath(setup.work_dir)
    except Exception as exc:
        raise RuntimeError(f"vaspkit 运行失败: {exc}") from exc


def _prepare_eledos_dependencies(setup: QECalculationSetup) -> None:
    _copy_scf_save_files(setup, copy_tree=True)


def _pw_run_commands(setup: QECalculationSetup, input_name: str, output_name: str) -> List[str]:
    return [f"{resolve_qe_command(setup.qe_bin, 'pw.x')} < {input_name} > {output_name}"]


def _scf_run_commands(setup: QECalculationSetup) -> List[str]:
    return _pw_run_commands(setup, "scf.in", "scf.out")


def _nscf_run_commands(setup: QECalculationSetup) -> List[str]:
    return _pw_run_commands(setup, "nscf.in", "nscf.out")


def _eband_run_commands(setup: QECalculationSetup) -> List[str]:
    bands_cfg = setup.cli_config.get("bands", {})
    proj_cfg = setup.cli_config.get("projwfc", {})
    bands_exec = resolve_qe_command(bands_cfg.get("executable_path", setup.qe_bin), "bands.x")
    proj_exec = resolve_qe_command(proj_cfg.get("executable_path", setup.qe_bin), "projwfc.x")
    bands_args = bands_cfg.get("args", "-pd .true.")
    proj_args = proj_cfg.get("args", "-pd .true.")
    bands_cmd = f"{bands_exec} {bands_args}".strip() if bands_args else bands_exec
    proj_cmd = f"{proj_exec} {proj_args}".strip() if proj_args else proj_exec
    return [
        f"{resolve_qe_command(setup.qe_bin, 'pw.x')} < eband.in > eband.out",
        f"{bands_cmd} < elebanddata.in > elebanddata.out",
        f"{proj_cmd} < elebandprojdata.in > elebandprojdata.out",
    ]


def _eledos_run_commands(setup: QECalculationSetup) -> List[str]:
    dos_cfg = setup.cli_config.get("dos", {})
    proj_cfg = setup.cli_config.get("projwfc", {})
    dos_exec = resolve_qe_command(dos_cfg.get("executable_path", setup.qe_bin), "dos.x")
    proj_exec = resolve_qe_command(proj_cfg.get("executable_path", setup.qe_bin), "projwfc.x")
    dos_args = dos_cfg.get("args", "-pd .true.")
    proj_args = proj_cfg.get("args", "-pd .true.")
    dos_cmd = f"{dos_exec} {dos_args}".strip() if dos_args else dos_exec
    proj_cmd = f"{proj_exec} {proj_args}".strip() if proj_args else proj_exec
    return [
        f"{dos_cmd} < eletdos.in > eletdos.out",
        f"{proj_cmd} < elepdos.in > elepdos.out",
    ]


SPECS: Dict[str, QECalculationSpec] = {
    "scf": QECalculationSpec(
        kind="scf",
        work_dir="qe_scf",
        config_section="qe_scf",
        template_factory=get_qe_scf_parameters_template,
        allowed_keys_factory=get_qe_scf_allowed_keys,
        input_filename="scf.in",
        success_message="QE 计算完成，输出已保存至 qe_scf/scf.out",
        error_prefix="执行 QE 出错",
        input_writer=_write_scf_inputs,
        run_commands_factory=_scf_run_commands,
    ),
    "nscf": QECalculationSpec(
        kind="nscf",
        work_dir="qe_nscf",
        config_section="qe_nscf",
        template_factory=get_qe_nscf_parameters_template,
        allowed_keys_factory=get_qe_nscf_allowed_keys,
        input_filename="nscf.in",
        success_message="QE NSCF 计算完成，输出已保存至 qe_nscf/nscf.out",
        error_prefix="执行 QE NSCF 出错",
        input_writer=_write_nscf_inputs,
        run_commands_factory=_nscf_run_commands,
        dependency_preparer=_prepare_nscf_dependencies,
    ),
    "eband": QECalculationSpec(
        kind="eband",
        work_dir="qe_eband",
        config_section="qe_eband",
        template_factory=get_qe_eband_parameters_template,
        allowed_keys_factory=get_qe_eband_allowed_keys,
        input_filename="eband.in",
        success_message="QE eband 计算完成，输出已保存至 qe_eband",
        error_prefix="执行 QE eband 出错",
        input_writer=_write_eband_inputs,
        run_commands_factory=_eband_run_commands,
        dependency_preparer=_prepare_eband_dependencies,
    ),
    "eledos": QECalculationSpec(
        kind="eledos",
        work_dir="qe_eledos",
        config_section="qe_eledos",
        template_factory=get_qe_eledos_parameters_template,
        allowed_keys_factory=get_qe_eledos_allowed_keys,
        input_filename="eletdos.in",
        success_message="QE eledos 计算完成，输出已保存至 qe_eledos",
        error_prefix="执行 QE eledos 出错",
        input_writer=_write_eledos_inputs,
        run_commands_factory=_eledos_run_commands,
        dependency_preparer=_prepare_eledos_dependencies,
    ),
}
