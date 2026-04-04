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
- `vasp_scf`：SCF 默认模板
- `vasp_scf_converge`：收敛测试专用模板，只扫描这个块中的唯一列表参数
- `vasp_eband`：能带计算默认模板
- `vasp_eledos`：电子态密度默认模板
- `vasp_fermi`：费米面默认模板
- `vasp_cohp`：COHP 默认模板
- `vasp_locpot`：LOCPOT 默认模板
- `vasp_charge`：分能窗电荷默认模板
- `vasp_elf`：ELF 默认模板
- `vasp_opt`：结构优化默认模板与 `opt_stage_templates`
- `vasp_lobster`：共享 `slurm` / 基础路径模板

## 4. 说明
- 每个 VASP CLI 现在只读取自己的参数块和共享块，不再读取其它 CLI 的参数块。
- 顶层通用 `kmesh` 已删除且不再兼容；`vasp_scf`、`vasp_eband`、`vasp_opt` 等都需要在各自块内单独设置 `kmesh`，模板默认值统一为 `0.025`。
- `vasp_opt` 的 `rv1/rvf` 使用 `INCAR`，`rv4` 使用 `INCAR_1` 到 `INCAR_4`。
- `rv1/rvf` 和 `rv4` 的最终高精度参数来自 `vasp_opt`。
- `rv4` 前三步的粗优化默认值单独放在 `opt_stage_templates` 中，便于查看和维护。
