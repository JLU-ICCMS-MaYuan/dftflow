---
name: vaspflow
description: 面向 dftflow 当前 vasp_* CLI 的开发规范技能。用于新增或修改 vaspflow 功能时，遵循现有 CLI 分类、模板配置体系、后处理边界、脚本入口、测试与文档同步约定。
---

# vaspflow 开发规范技能

## 何时使用
- 当任务涉及新增、重构或修复任意 `vasp_*` CLI 时使用。
- 当任务涉及 `vaspflow/templates.py`、`public/config_templates.py`、`pyproject.toml` 中的 `vasp_*` 入口、`test_auto/MgB2/test_vaspflow.py`、`vaspflow/README.md` 时使用。
- 本技能只覆盖 `vasp_*` 能力，不包含 `mattersim_opt`、`substitutionflow`、`qeflow`、`wannier90flow`。

## 先判断功能属于哪一类
新增或修改前，先按当前仓库真实实现把功能归类，不要直接套旧模板。

### 1. 生成型计算 CLI
这些命令通常读取 `input.toml` 模板、生成工作目录、写 `run_vasp.sh`，并支持 `-r/--run`、`-s/--submit` 中的一部分或全部：
- `vasp_scf`
- `vasp_scf_converge`
- `vasp_eband`
- `vasp_eledos`
- `vasp_fermi`
- `vasp_cohp_uniform`
- `vasp_cohp_fatband`
- `vasp_lobsterin`
- `vasp_locpot`
- `vasp_charge`
- `vasp_opt`
- `vasp_elf`

这类功能优先复用：
- `vaspflow/templates.py`
- `public/config_templates.py`
- 现有同类模块的目录结构、运行脚本和提交接线

### 2. 纯后处理 CLI
这些命令通常不读取 `input.toml` 模板，而是直接消费已有 VASP 或 LOBSTER 输出：
- `vasp_process_eledos`
- `vasp_idos_computed`
- `vasp_process_eband`
- `vasp_process_locpot`
- `vasp_plot_cohp`
- `vasp_plot_kcohp`

这类功能的实现原则：
- 直接围绕已有输出文件建模
- 优先复用现有解析逻辑
- 输出命名、CSV/PNG 风格、终端摘要与同类脚本保持一致
- 不要为了“统一”强行接入模板系统

### 3. 辅助处理 CLI
- `vasp_charge_diff`

这类命令通常是轻量组合或文件后处理。保持接口简洁，不要为了形式化引入无用模板或运行脚本。

## 当前配置与模板体系
当前仓库的稳定约定不是“每个模块自由读配置”，而是“模板 + 白名单过滤 + 覆盖合并”。

### 共享键
当前 VASP 共享键固定为：
- `potcar_dir`
- `chgcar_path`
- `wavecar_path`
- `lobster_executable_path`
- `[vasp]`
- `[slurm]`

不要新增跨模块乱读的共享字段。若确实需要新增共享键，必须同时更新：
- `vaspflow/templates.py`
- 相关 CLI 的 allowed keys
- README 与测试

### 模板与合并机制
- 模板定义在 `vaspflow/templates.py`
- 配置读取与递归合并在 `public/config_templates.py`
- 统一入口是 `build_cli_config()`

必须遵守：
- 每个 CLI 只读取自己的参数块和共享块
- 不允许跨模块读取别的参数块
- 新参数优先进入该 CLI 对应块，而不是顶层堆字段

### 当前块命名规则
参数块名称直接对应 CLI 名：
- `[vasp_scf]`
- `[vasp_scf_converge]`
- `[vasp_eband]`
- `[vasp_eledos]`
- `[vasp_fermi]`
- `[vasp_cohp_uniform]`
- `[vasp_cohp_fatband]`
- `[vasp_locpot]`
- `[vasp_charge]`
- `[vasp_elf]`
- `[vasp_opt]`

不要再引入旧风格的：
- `[incar_params]`
- `[<module>_params]`

### 已经稳定的特殊约定
- 顶层通用 `kmesh` 已废弃，默认 `kmesh = 0.025` 由各 CLI 模板单独提供。
- `vasp_opt` 不生成也不复制 `KPOINTS`，其最终高精度步通过 `KSPACING` 控制；若未显式设置 `KSPACING`，则使用 `[vasp_opt].kmesh * 2π` 换算。
- `vasp_cohp_fatband` 使用 `scf_kmesh`、`kpath_kmesh`、`kmesh_scheme`，其中 `kmesh_scheme` 仅允许 `Monkhorst-Pack` 或 `Gamma`。
- `lobster_executable_path` 是顶层键，不放在任何 `[xxx]` 模块下。

## 实现时的结构约定
不要再假设“所有 CLI 都必须有同一组方法”。当前仓库更强调“按类别保持一致”。

### 生成型 CLI
通常应包含这些能力：
- 读取结构文件
- 读取并合并模板配置
- 解析元素顺序并生成 `POTCAR`
- 生成工作目录中的输入文件
- 生成 `run_vasp.sh` 或等效脚本
- 需要时接线 `-r/--run` 与 `-s/--submit`

但不要机械要求所有模块都必须生成：
- `KPOINTS`
- 相同命名的类
- 完全相同的方法名

以当前同类实现为准，保持一致性优先于抽象整齐。

### 纯后处理 CLI
通常应包含这些能力：
- 直接读取现有输出文件
- 在必要时先做前置修正，例如费米能同步
- 输出结构化数据文件，例如 CSV / DAT
- 输出图片，例如 PNG
- 在终端打印关键摘要

这类 CLI 默认不应读 `input.toml`，除非当前问题确实是“生成型流程的一部分”。

### 新增功能前先检查是否已有近邻能力
避免继续堆重复命令。新增前先看能否扩展已有脚本：
- DOS 类后处理优先看 `vasp_process_eledos` / `vasp_idos_computed`
- 能带类后处理优先看 `vasp_process_eband`
- COHP / LOBSTER 相关优先看 `vasp_cohp_uniform`、`vasp_cohp_fatband`、`vasp_lobsterin`、`vasp_plot_cohp`、`vasp_plot_kcohp`
- 电荷 / 势能 / ELF 类优先看现有 `charge`、`locpot`、`elf`

如果只是扩展已有命令的参数、输出或解析逻辑，优先改现有 CLI，不要新开一个名字非常接近的命令。

## 文档与测试同步
修改 `vasp_*` 功能时，通常至少要同步以下位置中的一部分：
- `pyproject.toml`
- `vaspflow/README.md`
- `README.md`
- `test_auto/MgB2/test_vaspflow.py`

### 测试要求
优先在 `test_auto/MgB2/test_vaspflow.py` 中补回归测试。

至少覆盖与变更直接相关的行为：
- 配置块读取与模板覆盖
- 工作目录与关键输入文件生成
- `--run/--submit` 接线
- 后处理输入解析
- 关键数值行为，例如费米能修正、轨道聚合、绘图范围、区间积分、KSPACING 换算、vaspkit 输入序列

### 文档要求
当变更会影响对外语义时，同步更新 README：
- 新 CLI
- 配置块名
- 参数约定
- 纯后处理 / 不读模板这类行为边界

不要让代码、测试、README 三者脱节。

## 常见易错点
- 不要跨模块读取别的配置块。
- 不要恢复顶层通用 `kmesh`。
- 不要把纯后处理 CLI 强行接入 `input.toml`。
- 不要假设所有生成型 CLI 都必须有 `KPOINTS`。
- 不要把 `lobster_executable_path` 放进某个 `[vasp_xxx]` 块。
- 不要为现有能力的轻微变体新增重复 CLI。

## 查阅顺序
如果需要具体事实，按这个顺序查：
1. `pyproject.toml`：确认当前公开 CLI
2. `vaspflow/templates.py`：确认模板块、共享键、默认值
3. `public/config_templates.py`：确认过滤与合并机制
4. `vaspflow/README.md`：确认已公开的特殊约定
5. `test_auto/MgB2/test_vaspflow.py`：确认当前行为边界与回归覆盖方式
