# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

dftflow 是一套 DFT（密度泛函理论）计算工作流自动化工具集，用于自动生成 VASP、Quantum ESPRESSO、Wannier90 的计算输入文件并管理任务提交与执行。

## 常用命令

### 安装
```bash
pip install .                    # 基础安装
pip install ".[analysis]"        # 含 pymatgen 分析依赖
```

### 已注册的 CLI 入口（安装后可直接使用）
VASP 系列: `vasp_scf`, `vasp_scf_converge`, `vasp_eband`, `vasp_eledos`, `vasp_fermi`, `vasp_cohp_uniform`, `vasp_cohp_fatband`, `vasp_lobsterin`, `vasp_locpot`, `vasp_process_locpot`, `vasp_charge`, `vasp_charge_diff`, `vasp_opt`, `vasp_elf`

QE 系列: `qe_scf`, `qe_nscf`, `qe_eband`, `qe_eledos`, `qe_process_eledos`, `qe_process_eband`

Wannier90 系列: `wannier_init`, `wannier_post`, `wannier_window`, `wannier_plot`

### 通用 CLI 参数模式
```bash
<command> -i POSCAR -c input.toml     # 生成输入文件
<command> -i POSCAR -c input.toml -r  # 生成并本地执行
<command> -i POSCAR -c input.toml -s  # 生成并 sbatch 提交
```
`-r/--run` 与 `-s/--submit` 互斥。

### 测试
项目没有自动化测试框架。`test_manual/` 包含各模块的手动测试目录和示例配置（`inputvasp.toml`、`inputqe.toml`、`inputw90init.toml`、`inputw90post.toml`）。`test_auto/` 包含自动测试用结构文件。

## 架构

### 三大工作流引擎

- **vaspflow/** — VASP 计算工作流，按功能拆分子模块：`scf`、`eband`、`eledos`、`fermisurface`、`cohp`、`locpot`、`charge`、`opt`、`elf`
- **qeflow/** — Quantum ESPRESSO 工作流：`scf`、`nscf`、`eband`、`eledos`
- **wannier90flow/** — Wannier90 工作流：`wannier_init.py`（生成 .win 输入）、`wannier_post.py`（postw90 多投影执行）、`wannier_window.py`（自动能窗估计）、`wannier90_plot.py`（QE 与 W90 能带对比绘图）

### 公共模块

- **public/job.py** — 统一的任务脚本生成（`write_job_script`）、本地执行（`execute_commands`）、Slurm 提交（`submit_script`）及 CLI 互斥参数注册（`add_run_submit_arguments`）。所有工作流脚本的 `-r/-s` 功能通过此模块实现。

### 配置驱动模式

所有工作流通过 `input.toml`（TOML 格式）驱动，典型结构：
- 顶层字段：`potcar_dir`、`chgcar_path`、`wavecar_path`、`lobster_executable_path`
- `[vasp]` / `[qe]`：可执行命令或路径配置
- `[slurm]`：Slurm 提交脚本头
- `[vasp_scf]`、`[vasp_eband]`、`[vasp_cohp_uniform]`、`[vasp_cohp_fatband]` 等：按 CLI 命名的模块参数块
- 各 VASP CLI 在自己的参数块内单独设置 `kmesh`

各模块示例配置：`vaspflow/input.toml`、`qeflow/input.toml`、`wannier90flow/input.toml`。

### 新增 vaspflow 子模块的规范

遵循 `skills/vaspflow/SKILL.md` 定义的模板：
- 目录：`vaspflow/<module>/`，包含 `__init__.py` 和 `vasp_<module>.py`
- 类名：`Vasp<Module>Setup`（PascalCase），参照 `vaspflow/scf/vasp_scf.py` 的结构化类 + `main()` 入口
- 工作目录：`vasp_<module>`
- 在 `pyproject.toml` 的 `[project.scripts]` 注册入口

### 外部依赖

- Python >= 3.8，`numpy`，`toml`（Python < 3.11）或内置 `tomllib`（Python >= 3.11）
- 可选：`pymatgen`（分析功能）、`matplotlib`（绘图）
- 外部工具：`vaspkit`（生成 KPOINTS 和 KPATH.in）

## 规范

- 所有问答和代码注释使用中文
- Git 提交格式：`[YYYY-MM-DD HH:MM:SS] <需求类型>：<修改内容>`
- 命名规则：组件用 PascalCase，工具函数用 camelCase/snake_case
- 变更历史记录在 `AGENTS.md` 的"需求处理记录"中
