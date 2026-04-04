# VASPFlow 参数模板说明

## 1. 模板与 `input.toml` 的关系
- `vaspflow` 现在把“运行时默认模板”和“`input.toml` 用户覆盖”拆开处理。
- 默认模板只放在代码里的模板函数中，例如：
  - `get_vasp_scf_parameters_template()`
  - `get_vasp_eband_parameters_template()`
  - `get_vasp_opt_parameters_template()`
- `input.toml` 只负责覆盖，不再兼做默认值文档。

优先级固定为：
- 模板里有默认值，`input.toml` 没写：用模板默认值
- 模板里有默认值，`input.toml` 写了：用 `input.toml`
- 模板里没有默认值，`input.toml` 写了：用 `input.toml`

## 2. 模板函数位置
- VASP 模板定义：`vaspflow/templates.py`
- 公共递归合并逻辑：`public/config_templates.py`

## 3. 当前模板块
- `vasp_scf`：`scf_params`
- `vasp_scf_converge`：基于 `scf_params` 模板，再对唯一列表参数做扫描
- `vasp_eband`：`eband_params`
- `vasp_eledos`：`eledos_params`
- `vasp_fermi`：`fermisurface_params`
- `vasp_cohp`：`cohp_params`
- `vasp_locpot`：`locpot_params`
- `vasp_charge`：`charge_params`
- `vasp_elf`：`elf_params`
- `vasp_opt`：`opt_params` 与 `opt_stage_templates`
- `vasp_lobster`：共享 `slurm` / 基础路径模板

## 4. 说明
- `vasp_opt` 的 `rv1/rvf` 使用 `INCAR`，`rv4` 使用 `INCAR_1` 到 `INCAR_4`。
- `rv1/rvf` 和 `rv4` 的最终高精度参数来自 `opt_params`。
- `rv4` 前三步的粗优化默认值单独放在 `opt_stage_templates` 中，便于查看和维护。
