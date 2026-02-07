---
name: vaspflow
description: 面向 dftflow/vaspflow 的通用开发规范技能，用于新增任意 vaspflow 子功能时统一目录结构、脚本入口、配置字段、输出规范、依赖说明、测试与校验流程。
---

# vaspflow 通用子功能工作流模板

## 目标
- 任何新增 vaspflow 子功能都遵循一致的结构与调用方式。
- 强制统一配置字段、输出文件与命令入口，便于维护与复用。

## 目录结构（模板）
- 在 `vaspflow/<module>/` 下创建：
  - `__init__.py`
  - `vasp_<module>.py`
- 工作目录命名规则：`work_dir = "vasp_<module>"`

## 脚本骨架（必须包含）
- 结构化类 + `main()` 入口，参考 `vaspflow/scf` 风格。
- 必备方法：
  - `__init__(config_file="input.toml", struct_file=None)` 读取配置
  - `get_elements_from_poscar()` 解析 POSCAR 元素顺序
  - `generate_incar()` 合并默认模板与配置参数
  - `generate_potcar(elements)` 合并 POTCAR
  - `generate_kpoints()` 可选调用 vaspkit
  - `create_run_script()` 生成 `run_vasp.sh`（支持 Slurm）
  - `execute_vasp()` 在 `--run` 下执行
  - `run(run_calc=False)` 串联流程

## 命令入口（统一参数）
- `-i/--input`：结构文件
- `-c/--config`：配置文件（默认 `input.toml`）
- `--run`：生成文件后直接执行

## 配置字段规范（input.toml）
通用字段（所有模块必须支持）：
- `potcar_dir`：赝势目录
- `kmesh`：k 点密度或网格（可选）
- `[vasp].executable_path`：运行命令
- `[slurm].header`：提交脚本头（可选）
- `[incar_params]`：覆盖默认 INCAR 参数

模块字段（按模块自行定义）：
- `[<module>_params]`：模块专用 INCAR 覆盖
- 其他模块专用字段需在脚本内显式读取与说明

## 输出规范
- 统一输出路径：`vasp_<module>/`
- 必须生成：`INCAR`、`POTCAR`、`KPOINTS`（若适用）、`POSCAR`、`run_vasp.sh`
- 运行输出文件依赖 VASP 本身，不在脚本中硬编码

## 依赖说明
- Python：`tomllib` 或 `toml`
- 外部工具：`vaspkit`（仅当模块需要生成 KPOINTS）
- 可选依赖必须写明缺失时的提示语

## 代码风格与命名约束
- 文件名：`vasp_<module>.py`
- 类名：`Vasp<Module>Setup`（PascalCase）
- 变量/函数：`snake_case`
- 所有输出路径使用 `os.path.join`

## 测试与校验（必做清单）
- 仅生成模式：`python vasp_<module>.py -i POSCAR -c input.toml`
- 运行模式（可选）：`python vasp_<module>.py -i POSCAR -c input.toml --run`
- 确认 `vasp_<module>/` 下文件齐全
- 配置字段缺失时提示清晰

## 变更记录（模板）
- 需求：新增 <module>
- 方案：复用通用模板，补充模块参数与输出说明
- 文件：`vaspflow/<module>/vasp_<module>.py`、`vaspflow/input.toml`

