# vaspflow 架构边界

- 状态：Accepted
- 日期：2026-05-29
- 相关模块：`vaspflow/`、`public/`

## 背景

`vaspflow` 目前覆盖 VASP 输入生成、运行/提交脚本、结构优化、VASP 后处理和 LOBSTER/COHP 分析。主要入口通过 `pyproject.toml` 暴露为 `vasp_*` console scripts，代码按计算类型分布在 `scf`、`opt`、`eband`、`eledos`、`cohp`、`locpot`、`charge`、`elf` 和 `fermisurface` 子目录。

源码呈现出三类职责：

1. 运行型 VASP setup CLI：生成 `POSCAR/INCAR/POTCAR/KPOINTS`，写 `run_vasp.sh`，并可通过 `-r/-s` 直接运行或提交。
2. 纯后处理 CLI：读取已有 VASP、VASPKIT 或 LOBSTER 产物，输出 `CSV/DAT/PNG`，不读取 `input.toml` 模板。
3. 公共基础设施：`vaspflow/templates.py` 管理参数模板和允许覆盖的配置块，`public/job.py` 管理运行/提交脚本，`public/config_templates.py` 管理配置加载与模板合并，`public/executables.py` 管理可执行命令解析。

运行型 CLI 中存在较多重复逻辑，例如 POSCAR 拷贝、POTCAR 合并、INCAR 写出、SCF 文件拷贝、VASPKIT KPOINTS 调用和 `run_vasp.sh` 写出。`cohp` 已通过 `BaseVaspCohpSetup` 局部抽出公共生命周期，但 SCF/DOS/EBAND/LOCPOT/CHARGE/ELF/FERMI 仍以各自类内实现为主。ADR 需要先固化真实边界，避免后续新增能力继续扩大重复，也避免把纯后处理逻辑混入运行 setup 生命周期。

## 决策

vaspflow 的依赖方向固定为：

```text
CLI Adapter
  -> calculation-specific setup / postprocess object
  -> vaspflow.templates / cohp shared helpers
  -> public.config_templates / public.job / public.executables
```

模块职责如下：

| 模块 | 职责 | 不应承担的职责 |
| --- | --- | --- |
| `vaspflow/templates.py` | 保存 VASP CLI 的默认参数模板、共享键、允许覆盖键和模板合并入口 | 不写输入文件，不执行外部命令 |
| `vaspflow/scf` | SCF 输入准备、`KPOINTS` 生成、`run_vasp.sh` 和 SCF 输出存在性校验 | 不承载其它计算类型的公共生命周期 |
| `vaspflow/kpoints.py` | VASP automatic mesh `KPOINTS` 的解析、写出和整数倍网格推导 | 不调用 VASPKIT，不读取结构或执行计算 |
| `vaspflow/opt` | 结构优化 `rv1/rv4/rvf` 模式、压力 GPa 到 kbar 转换、多压力目录和批量脚本 | 不生成 KPOINTS；k 点精度通过 `KSPACING` 管理 |
| `vaspflow/eband` | 能带输入准备和 `vasp_process_eband` 后处理 | setup 不解析绘图数据；后处理不读取模板配置 |
| `vaspflow/eledos` | DOS 输入准备、`vasp_process_eledos` 与 `vasp_idos_computed` 后处理 | setup 不承担 PDOS 聚合；后处理不生成 VASP 输入 |
| `vaspflow/cohp` | COHP VASP 输入准备、LOBSTER 输入目录、orbital interaction 解析和 COHP/KCOHP 后处理 | 不把 LOBSTER 后处理逻辑放入 VASP setup 基类 |
| `vaspflow/locpot` | LOCPOT 输入准备和体数据沿路径采样后处理 | 采样/绘图逻辑不进入运行型 setup |
| `vaspflow/charge` | 分能窗电荷输入准备和电荷密度差分 | 差分工具不依赖 `input.toml` |
| `vaspflow/elf`、`vaspflow/fermisurface` | 对应 VASP 计算的薄 setup CLI | 不新增跨模块公共 helper 的局部副本 |
| `public/job.py` | 脚本写出、`-r/-s` 行为、Slurm 提交和预期输出检查 | 不包含 VASP 参数、目录命名或物理语义 |
| `public/config_templates.py` | TOML/JSON 加载、按 allowed keys 过滤、递归合并模板 | 不处理具体 CLI 默认参数 |
| `public/executables.py` | VASP/QE/直接命令解析 | 不拼接运行脚本内容 |

## 架构约束

1. 用户可见命令名保持兼容。内部类名和模块可以重构，但 `pyproject.toml` 中现有 `vasp_*` console script 名称不应轻易变化。
2. 运行型 CLI 文件应只保留参数解析、特定计算参数和对 setup 对象的调用；重复的 POSCAR/POTCAR/INCAR/KPOINTS/SCF 文件拷贝逻辑后续应优先收敛到共享 setup helper，而不是继续复制。
3. 纯后处理 CLI 不读取 `input.toml` 模板。其输入来自命令行参数和已有产物文件，例如 `DOSCAR`、`BAND.dat`、`PDOS_*.dat`、`COHPCAR.lobster`、`KspaceCOHPBand*.lobster`、`LOCPOT/PARCHG/CHGCAR`。
4. `vaspflow/templates.py` 是默认参数和配置覆盖边界的唯一入口。新增运行型 CLI 时必须新增模板函数和 allowed keys，避免从其它 CLI 参数块读取配置。
5. `potcar_dir`、`chgcar_path`、`wavecar_path`、`lobster_executable_path`、`vasp`、`slurm` 仍是共享键；计算专属参数必须放入对应 `[vasp_*]` 块。
6. VASPKIT 调用保持静默输出，失败时保留明确错误状态或缺失文件报错，不把 VASPKIT banner/menu 回显给用户。
7. `vasp_opt` 继续把命令行 `-p/--pressure` 视为 GPa，写入 VASP `PSTRESS` 前转换为 kbar；多压力模式目录名保留用户输入的 GPa 数值。
8. `vasp_opt` 不生成或复制 `KPOINTS`；如需改变优化 k 点策略，应新增 ADR 或更新本 ADR。
9. COHP 交互语法统一使用 `Element:orbital-list|Element:orbital-list`，`vasp_plot_cohp` 与 `vasp_plot_kcohp` 复用 `orbital_interactions.py`，避免两套解析规则分叉。
10. 文档同步要求：改变模块边界、模板覆盖规则、运行/提交语义或后处理输入输出时，同步更新本 ADR 或新增 ADR，并检查 `vaspflow/README.md` 和相关测试。

## 影响

收益：

- 把 `vaspflow` 当前真实形态分成运行 setup、纯后处理和公共基础设施三层，后续重构可以按职责推进。
- 为收敛重复 setup 生命周期提供方向，同时不要求一次性重写全部 CLI。
- 明确纯后处理工具不读取模板配置，降低批量工作流中配置真源和已有产物之间的耦合。
- 将 COHP interaction 解析、`vasp_opt` 压力单位、VASPKIT 静默调用等既有约定固化为后续兼容约束。

代价：

- 新增 VASP 能力时需要先判断归属，不能直接把通用逻辑写进最近的 CLI 文件。
- 后续抽共享 setup helper 时，需要保护现有 `VaspSetup`、`VaspDosSetup` 等测试中直接导入的类，或同步迁移测试和兼容入口。
- 当前 ADR 记录的是现状和约束，不等同于已经完成所有重复逻辑的消除。

## 验证

涉及 vaspflow 架构边界的变更至少检查：

- `rg "vaspflow\\.|vasp_" pyproject.toml vaspflow test_auto/MgB2/test_vaspflow.py`
- 聚焦测试：`pytest test_auto/MgB2/test_vaspflow.py test_auto/MgB2/test_execution_semantics.py`
- 若影响 substitution 批量链路，再运行或检查 `test_auto/MgB2/test_substitutionflow.py` 中的 `vasp_opt/vasp_scf/vasp_eledos/vasp_process_eledos` 相关场景。
- 手工检查 `vaspflow/README.md` 是否仍描述真实模板块、后处理输入输出和 CLI 行为。

## 后续

- 评估新增 `vaspflow/setup/` 或等价共享 helper，逐步收敛 POSCAR/POTCAR/INCAR/KPOINTS/SCF 依赖文件和脚本写出逻辑。
- 评估新增 `vaspflow/io_reader/`，集中承载 OUTCAR、DOSCAR、PBAND、TDOS/PDOS、体数据和 LOBSTER 文本读取，避免后处理 CLI 内继续膨胀解析函数。
- 为 `vaspflow` 补充 Mermaid CLI 流程图索引，使用户文档与 ADR 分工和 qeflow 保持一致。
