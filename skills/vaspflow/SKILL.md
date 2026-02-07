---
name: vaspflow
description: 面向 dftflow/vaspflow 的通用工作流技能，尤其用于新增 vasp_locpot 等子功能时，要求目录结构、脚本入口、配置字段与输出规范对齐 vaspflow/scf。
---

# vaspflow 通用工作流（以 vasp_locpot 为例）

## 目标
- 新增 vaspflow 子功能（如 `vasp_locpot`）时，复用 `vaspflow/scf` 的结构与交互方式。
- 明确目录结构、脚本骨架、配置字段与输出文件约定。

## 适用场景
- 需要在 `vaspflow/` 下新增一个与 `vasp_scf` 类似的脚本（如 `vasp_locpot.py`）。
- 需要统一输入结构、工作目录与运行脚本的生成方式。

## 目录结构模板
- 在 `vaspflow/locpot/` 下创建：
  - `__init__.py`
  - `vasp_locpot.py`
- 工作目录名称遵循 `vasp_xxx` 规范：`work_dir = "vasp_locpot"`

## 脚本结构模板（对齐 vaspflow/scf）
- 使用 class 组织逻辑，入口 `main()` 调用 `run()`。
- 必备方法：
  - `__init__(config_file="input.toml", struct_file=None)` 读取配置
  - `get_elements_from_poscar()` 解析 POSCAR 第 6 行元素
  - `generate_incar()` 合并默认模板与 `[incar_params]`
  - `generate_potcar(elements)` 依据 `potcar_dir` 拼 POTCAR
  - `generate_kpoints()` 使用 vaspkit 生成 KPOINTS
  - `create_run_script()` 生成 `run_vasp.sh`（支持 Slurm 头）
  - `execute_vasp()` 在 `--run` 下执行
  - `run(run_calc=False)` 统一流程

## locpot 相关逻辑建议
- 在默认 INCAR 模板中设置生成电势文件的参数（按实际需求选择）：
  - `LVTOT` 或 `LVHAR`（二选一或同时开）
  - 视情况保留 `LCHARG`、`LWAVE`
- 仍允许 `[incar_params]` 覆盖默认值。

## 配置字段约定（input.toml）
- `potcar_dir`: 赝势目录（必填）
- `kmesh`: 形如 `"25 25 25"` 的字符串（可选）
- `[vasp]`
  - `executable_path`: 运行命令
- `[slurm]`
  - `header`: Slurm 脚本头（可选）
- `[incar_params]`
  - 覆盖默认 INCAR 参数
- 可新增 `[locpot]` 分组保存专用参数（如需要）

## 输出规范
- 工作目录：`vasp_locpot/`
- 生成文件：`INCAR`、`POTCAR`、`KPOINTS`、`POSCAR`、`run_vasp.sh`
- 计算输出：`LOCPOT`、`CHGCAR`、`OUTCAR` 等（依 VASP 运行结果）

## 命令入口约定
- 参照 `vasp_scf.py` 的 argparse 结构：
  - `-c/--config` 指定 `input.toml`
  - `-i/--input` 指定结构文件
  - `--run` 触发本地执行

## 验收清单
- 目录与命名符合 `vaspflow/scf` 的习惯
- `--run` 与非 `--run` 均可用
- 配置字段与输出文件清晰可追溯
