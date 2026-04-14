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
- 顶层共享键：`potcar_dir`、`chgcar_path`、`wavecar_path`、`lobster_executable_path`
- `vasp_scf`：SCF 默认模板
- `vasp_scf_converge`：收敛测试专用模板，只扫描这个块中的唯一列表参数
- `vasp_eband`：能带计算默认模板
- `vasp_eledos`：电子态密度默认模板
- `vasp_fermi`：费米面默认模板
- `vasp_cohp_uniform`：均匀网格 COHP 默认模板
- `vasp_cohp_fatband`：fatband COHP 默认模板
- `vasp_locpot`：LOCPOT 默认模板
- `vasp_charge`：分能窗电荷默认模板
- `vasp_elf`：ELF 默认模板
- `vasp_opt`：结构优化默认模板与 `opt_stage_templates`

## 4. 说明
- 每个 VASP CLI 现在只读取自己的参数块和共享块，不再读取其它 CLI 的参数块。
- 顶层通用 `kmesh` 已删除且不再兼容；`vasp_scf`、`vasp_eband`、`vasp_opt` 等都需要在各自块内单独设置 `kmesh`，模板默认值统一为 `0.025`。
- `vasp_cohp_uniform` 使用均匀网格 `KPOINTS`；`vasp_cohp_fatband` 使用 `vaspkit 303 + 251` 生成包含均匀网格和零权重路径点的混合 `KPOINTS`。
- `[vasp_cohp_fatband]` 使用 `scf_kmesh`、`kpath_kmesh`、`kmesh_scheme`，不再使用单一 `kmesh`；其中 `kmesh_scheme` 仅接受 `Monkhorst-Pack` 或 `Gamma`。
- `lobster_executable_path` 是顶层键，不放在任何 `[xxx]` 模块下；`vasp_lobsterin` 只读取这个顶层键和 `[slurm]`。
- `vasp_plot_cohp` 是一个纯后处理 CLI，用于读取 `COHPCAR.lobster` 并输出 orbital-resolved `COHP / ICOHP` 数据和图片；它不读取 `input.toml` 模板。
- `vasp_plot_cohp` 的 `--interaction` 语法与 `vasp_plot_kcohp` 保持一致，固定为 `"元素A:轨道列表|元素B:轨道列表"`，例如 `"Mg:3s,3p|B:2s,2p"`；其中 `3p` 会自动聚合 `3p_x/3p_y/3p_z` 等分量。
- `vasp_plot_cohp` 默认同时输出 `proj_cohp.dat`、`average_cohp.dat`、`proj_cohp.png` 和 `average_cohp.png`。
- `vasp_plot_kcohp` 是一个纯后处理 CLI，不读取 `input.toml` 模板；它直接读取 LOBSTER 目录中的 `KspaceCOHPBand*.lobster`、`POSCAR`、`KPOINTS`、`KPATH.in` 和 `OUTCAR`。
- `vasp_plot_kcohp` 的 `--interaction` 语法固定为 `"元素A:轨道列表|元素B:轨道列表"`，例如 `"Mg:3s,3p|B:2s,2p"`；每个 `--interaction` 对应一个子图。
- `vasp_plot_kcohp` 默认从 `OUTCAR` 读取费米能，将能量统一绘制为 `E - E_F`，并同时输出 `PNG + DAT`。
- `vasp_process_eledos` 是一个纯后处理 CLI，不读取 `input.toml` 模板；它默认在 `vasp_eledos` 目录中先同步 `DOSCAR` 的费米能到 `../vasp_scf/OUTCAR`，再调用 `vaspkit` 生成 `TDOS.dat` 与 `PDOS_<元素>.dat`，按 `--elements "元素:轨道"` 聚合后输出 `CSV + PNG`。
- `vasp_opt` 的 `rv1/rvf` 使用 `INCAR`，`rv4` 使用 `INCAR_1` 到 `INCAR_4`。
- `rv1/rvf` 和 `rv4` 的最终高精度参数来自 `vasp_opt`。
- `vasp_opt` 不生成也不复制 `KPOINTS`；其 k 点精度完全通过 `KSPACING` 控制。若 `[vasp_opt].KSPACING` 未设置，则仅对最终步用 `[vasp_opt].kmesh * 2π` 换算。
- `rv4` 前三步的粗优化默认值单独放在 `opt_stage_templates` 中，便于查看和维护。
