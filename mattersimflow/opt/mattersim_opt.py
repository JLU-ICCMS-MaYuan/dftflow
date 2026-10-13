#!/usr/bin/env python3
import argparse
import importlib.util
import json
import os
import shutil
from typing import Dict, List, Optional

from mattersimflow._public_compat import load_config_file
from mattersimflow._public_compat import resolve_direct_command
from mattersimflow._public_compat import add_run_submit_arguments, execute_commands, handle_job_action, write_job_script
from mattersimflow.templates import (
    apply_mattersim_config_overrides,
    get_mattersim_opt_allowed_keys,
    get_mattersim_opt_parameters_template,
)


RUNNER_NAME = "run_mattersim_opt.py"
SCRIPT_NAME = "run_mattersim.sh"
RESULT_JSON = "mattersim_opt.json"
RESULT_LOG = "mattersim_opt.log"
SUPPORTED_OPTIMIZERS = {"FIRE", "BFGS"}
SUPPORTED_CELL_FILTERS = {"ExpCellFilter", "FrechetCellFilter"}


class MatterSimOptSetup:
    def __init__(self, config_file="input.toml", struct_file=None, pressure_override=None, work_dir="mattersim_opt"):
        self.work_dir = work_dir
        self.parameter_template = get_mattersim_opt_parameters_template()
        self.input_config = load_config_file(config_file)
        self.raw_opt_params = self.input_config.get("mattersim_opt", {})
        self.config = apply_mattersim_config_overrides(
            self.parameter_template, self.input_config, get_mattersim_opt_allowed_keys()
        )
        self.struct_file = struct_file
        self.opt_params = self._load_opt_params()
        self.pressure_gpa = self._resolve_pressure(pressure_override)

    def _load_opt_params(self):
        opt_params = self.config.get("mattersim_opt")
        if not opt_params:
            raise ValueError("配置缺少 [mattersim_opt] 块")
        return opt_params

    def _get_stage_entries(self, source_params: Dict) -> List[Dict]:
        stage_entries = []
        for key, value in source_params.items():
            if not key.startswith("stage"):
                continue
            suffix = key[5:]
            if not suffix.isdigit():
                continue
            if not isinstance(value, dict):
                raise ValueError(f"[mattersim_opt.{key}] 必须是表结构")
            stage_entries.append((int(suffix), value))
        stage_entries.sort(key=lambda item: item[0])
        return [{"index": index, "name": f"stage{index}", "config": config} for index, config in stage_entries]

    def _normalize_fix_axis(self, fix_axis, stage_name: str):
        if isinstance(fix_axis, bool):
            return fix_axis
        if not isinstance(fix_axis, list):
            raise ValueError(f"[mattersim_opt.{stage_name}].fix_axis 必须是布尔值或长度为 6 的布尔列表")
        if len(fix_axis) != 6:
            raise ValueError(f"[mattersim_opt.{stage_name}].fix_axis 必须是长度为 6 的布尔列表")
        if not all(isinstance(item, bool) for item in fix_axis):
            raise ValueError(f"[mattersim_opt.{stage_name}].fix_axis 只能包含布尔值")
        return fix_axis

    def _normalize_stage_config(self, stage_name: str, config: Dict) -> Dict:
        optimizer = config.get("optimizer", "FIRE")
        if optimizer not in SUPPORTED_OPTIMIZERS:
            raise ValueError(
                f"[mattersim_opt.{stage_name}].optimizer 只支持 {sorted(SUPPORTED_OPTIMIZERS)}，当前为 {optimizer}"
            )

        cell_filter = config.get("cell_filter")
        if cell_filter in ("", None):
            cell_filter = None
        if cell_filter is not None and cell_filter not in SUPPORTED_CELL_FILTERS:
            raise ValueError(
                f"[mattersim_opt.{stage_name}].cell_filter 只支持 {sorted(SUPPORTED_CELL_FILTERS)}，当前为 {cell_filter}"
            )

        ionic_steps = config.get("ionic_steps", config.get("steps"))
        if ionic_steps is None:
            raise ValueError(f"[mattersim_opt.{stage_name}] 缺少 ionic_steps")

        if "fmax" not in config:
            raise ValueError(f"[mattersim_opt.{stage_name}] 缺少 fmax")

        fix_axis = config.get("fix_axis", False)
        if cell_filter is None and "fix_axis" not in config:
            fix_axis = False
        else:
            fix_axis = self._normalize_fix_axis(fix_axis, stage_name)

        return {
            "name": stage_name,
            "optimizer": optimizer,
            "cell_filter": cell_filter,
            "constrain_symmetry": bool(config.get("constrain_symmetry", True)),
            "fix_axis": fix_axis,
            "fmax": float(config["fmax"]),
            "ionic_steps": int(ionic_steps),
        }

    def _build_legacy_stage_configs(self) -> List[Dict]:
        legacy_keys = ("optimizer", "cell_filter", "constrain_symmetry", "fix_axis", "stage1_fmax", "stage1_steps", "stage2_fmax", "stage2_steps")
        if not any(key in self.opt_params for key in legacy_keys):
            raise ValueError("配置缺少 [mattersim_opt.stage1] 阶段定义")

        stage1 = {
            "optimizer": self.opt_params.get("optimizer", "FIRE"),
            "constrain_symmetry": bool(self.opt_params.get("constrain_symmetry", True)),
            "fmax": float(self.opt_params.get("stage1_fmax", 0.10)),
            "ionic_steps": int(self.opt_params.get("stage1_steps", 200)),
        }
        stage2 = {
            "optimizer": self.opt_params.get("optimizer", "FIRE"),
            "cell_filter": self.opt_params.get("cell_filter", "FrechetCellFilter"),
            "constrain_symmetry": bool(self.opt_params.get("constrain_symmetry", True)),
            "fix_axis": self.opt_params.get("fix_axis", False),
            "fmax": float(self.opt_params.get("stage2_fmax", 0.05)),
            "ionic_steps": int(self.opt_params.get("stage2_steps", 300)),
        }
        print("警告: [mattersim_opt] 平面 stage1/stage2 旧配置已废弃，请迁移到 [mattersim_opt.stageN]。")
        return [
            self._normalize_stage_config("stage1", stage1),
            self._normalize_stage_config("stage2", stage2),
        ]

    def _load_stage_configs(self) -> List[Dict]:
        raw_stage_entries = self._get_stage_entries(self.raw_opt_params)
        if raw_stage_entries:
            stage_entries = []
            for entry in raw_stage_entries:
                stage_entries.append(
                    {
                        "index": entry["index"],
                        "name": entry["name"],
                        "config": self.opt_params.get(entry["name"], entry["config"]),
                    }
                )
        elif any(
            key in self.raw_opt_params
            for key in ("optimizer", "cell_filter", "constrain_symmetry", "fix_axis", "stage1_fmax", "stage1_steps", "stage2_fmax", "stage2_steps")
        ):
            return self._build_legacy_stage_configs()
        else:
            stage_entries = self._get_stage_entries(self.opt_params)

        expected_index = 1
        normalized_stages = []
        for entry in stage_entries:
            if entry["index"] != expected_index:
                raise ValueError(
                    f"[mattersim_opt] 阶段编号必须连续，期望 stage{expected_index}，实际找到 {entry['name']}"
                )
            normalized_stages.append(self._normalize_stage_config(entry["name"], entry["config"]))
            expected_index += 1
        return normalized_stages

    def _resolve_pressure(self, pressure_override):
        if pressure_override is not None:
            return float(pressure_override)
        return 0.0

    def _get_mattersim_execmd(self):
        mattersim_config = self.config.get("mattersim", {})
        return resolve_direct_command(mattersim_config.get("executable_path"), "python")

    def _get_slurm_header(self):
        slurm_config = self.config.get("slurm", {})
        return slurm_config.get("header", "")

    def _get_source_path(self):
        source_path = self.input_config.get("mattersim_source_path")
        if not source_path:
            return None
        return os.path.abspath(os.path.expanduser(source_path))

    def _resolve_runner_source_path(self) -> Optional[str]:
        if importlib.util.find_spec("mattersim") is not None:
            return None

        source_path = self._get_source_path()
        if source_path:
            return source_path

        raise ValueError(
            "当前 Python 环境无法直接 import mattersim，且配置中也未提供顶层 mattersim_source_path"
        )

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
        elif not os.path.exists(target_poscar):
            raise FileNotFoundError("找不到可用于 MatterSim 预优化的 POSCAR 结构文件")

    def _json_literal(self, value):
        return json.dumps(value, ensure_ascii=False)

    def _py_literal(self, value):
        return repr(value)

    def _build_runner_content(self):
        opt = self.opt_params
        stages = self._load_stage_configs()
        stage_literal = self._py_literal(stages)
        source_path = self._resolve_runner_source_path()
        source_path_bootstrap = ""
        if source_path:
            source_path_bootstrap = f'sys.path.insert(0, {self._json_literal(source_path)})\n'
        return f"""#!/usr/bin/env python3
import json
import os
import sys

{source_path_bootstrap}

from ase.io import read, write
from ase.units import GPa
from mattersim.applications.relax import Relaxer
from mattersim.forcefield import MatterSimCalculator


def main():
    atoms = read("POSCAR")
    calculator = MatterSimCalculator(
        load_path={self._py_literal(opt.get("model", "mattersim-v1.0.0-1m"))},
        device={self._py_literal(opt.get("device", "cpu"))},
    )
    atoms.calc = calculator
    stages = {stage_literal}
    stage_results = {{}}
    current_atoms = atoms

    for stage in stages:
        current_atoms.calc = calculator
        relaxer = Relaxer(
            optimizer=stage["optimizer"],
            filter=stage["cell_filter"],
            constrain_symmetry=stage["constrain_symmetry"],
            fix_axis=stage["fix_axis"],
        )
        relax_kwargs = {{
            "fmax": stage["fmax"],
            "steps": stage["ionic_steps"],
        }}
        if stage["cell_filter"] is not None:
            relax_kwargs["params_filter"] = {{"scalar_pressure": {float(self.pressure_gpa)} * GPa}}
        converged, current_atoms = relaxer.relax(current_atoms, **relax_kwargs)
        write(f"{{stage['name']}}_relaxed.vasp", current_atoms, format="vasp", direct=True)
        stage_results[stage["name"]] = {{
            "converged": bool(converged),
            "energy": float(current_atoms.get_potential_energy()),
            "optimizer": stage["optimizer"],
            "cell_filter": stage["cell_filter"],
            "constrain_symmetry": bool(stage["constrain_symmetry"]),
            "fix_axis": stage["fix_axis"],
            "fmax": float(stage["fmax"]),
            "ionic_steps": int(stage["ionic_steps"]),
        }}

    write("CONTCAR", current_atoms, format="vasp", direct=True)

    result = {{
        "pressure_gpa": {float(self.pressure_gpa)},
        "stages": stage_results,
    }}
    with open("{RESULT_JSON}", "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
"""

    def _write_runner(self):
        runner_path = os.path.join(self.work_dir, RUNNER_NAME)
        with open(runner_path, "w", encoding="utf-8") as f:
            f.write(self._build_runner_content())
        os.chmod(runner_path, 0o755)
        return runner_path

    def _write_run_script(self):
        script_path = os.path.join(self.work_dir, SCRIPT_NAME)
        header = self._get_slurm_header()
        command = f"{self._get_mattersim_execmd()} {RUNNER_NAME} > {RESULT_LOG} 2>&1"
        write_job_script(script_path, header, [command])
        return SCRIPT_NAME

    def run(self):
        self._prepare_poscar()
        self._write_runner()
        self._script_name = self._write_run_script()
        print(f"已生成 MatterSim 预优化输入于 {self.work_dir}，提交脚本: {self._script_name}")

    def execute(self):
        script = getattr(self, "_script_name", SCRIPT_NAME)
        execute_commands(
            self.work_dir,
            [f"bash {script}"],
            work_label=os.path.basename(os.path.normpath(self.work_dir)),
            error_prefix="执行 MatterSim 预优化出错",
            expected_outputs=[RESULT_JSON, "CONTCAR"],
        )
        print(f"MatterSim 预优化完成，输出已保存至 {self.work_dir}")


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


def _create_batch_submit_script(setups: List[MatterSimOptSetup], script_name: str = "submit_all_mattersim_opt.sh") -> str:
    if not setups:
        raise ValueError("没有可用于批量提交的 mattersim_opt 任务。")
    slurm_header = setups[0].config.get("slurm", {}).get("header", "")
    script_path = os.path.abspath(script_name)
    commands = []
    for setup in setups:
        case_dir = os.path.abspath(setup.work_dir)
        commands.append(f'cd "{case_dir}"')
        commands.append(f"{setup._get_mattersim_execmd()} {RUNNER_NAME} > {RESULT_LOG} 2>&1")
    write_job_script(script_path, slurm_header, commands)
    return os.path.basename(script_path)


def _run_multiple_pressures(config_file, struct_file, pressure_values):
    setups = []
    for pressure_gpa in pressure_values:
        work_dir = os.path.join(_format_pressure_dirname(pressure_gpa), "mattersim_opt")
        setup = MatterSimOptSetup(
            config_file=config_file,
            struct_file=struct_file,
            pressure_override=pressure_gpa,
            work_dir=work_dir,
        )
        setup.run()
        setups.append(setup)
    return setups


def main():
    parser = argparse.ArgumentParser(description="MatterSim 结构预优化")
    parser.add_argument("-i", "--input", help="输入结构文件 (将被拷贝为 POSCAR)")
    parser.add_argument("-c", "--config", default="input.toml", help="配置文件路径 (默认为 input.toml)")
    parser.add_argument(
        "-p",
        "--pressure",
        nargs="+",
        help="目标压力，输入单位为 GPa（支持多个值）",
    )
    add_run_submit_arguments(parser, "生成文件后直接执行优化", "生成文件后提交优化脚本")
    args = parser.parse_args()

    if args.pressure:
        pressure_values = _parse_pressure_values(args.pressure)
        if len(pressure_values) == 1:
            pressure_value = pressure_values[0]
            work_dir = os.path.join(_format_pressure_dirname(pressure_value), "mattersim_opt")
            setup = MatterSimOptSetup(
                config_file=args.config,
                struct_file=args.input,
                pressure_override=pressure_value,
                work_dir=work_dir,
            )
            setup.run()
            handle_job_action(args.run, args.submit, setup.work_dir, SCRIPT_NAME, setup.execute)
            return

        setups = _run_multiple_pressures(args.config, args.input, pressure_values)

        def execute_all():
            for setup in setups:
                setup.execute()

        batch_script_name = _create_batch_submit_script(setups)
        handle_job_action(args.run, args.submit, ".", batch_script_name, execute_all)
        return

    setup = MatterSimOptSetup(
        config_file=args.config,
        struct_file=args.input,
        pressure_override=0.0,
        work_dir="mattersim_opt",
    )
    setup.run()
    handle_job_action(args.run, args.submit, setup.work_dir, SCRIPT_NAME, setup.execute)


if __name__ == "__main__":
    main()
