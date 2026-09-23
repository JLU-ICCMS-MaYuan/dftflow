# 规格：将 A2F QE 后处理迁移为 scflow

**Issue**：[IKHZT0](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHZT0)（`type-feature`）

## 背景与目标

`external/a2f` 已包含从 QE `*.dyn*.elph*` 和 `*.ph.out` 计算 A2F、McMillan、Allen-Dynes 和 Eliashberg 超导转变温度的完整实现。需要把代码复制到 dftflow 的 `scflow` 包中，使其成为 dftflow 自己的 CLI，运行时不依赖外部子模块。

## 用户故事

### US1：用 input.toml 处理 QE EPC 输出

用户提供 `input.toml` 和 QE EPC 输出目录后，执行 `scflow -c input.toml`，即可得到 Tc 和 A2F 的完整结果。

## 功能需求

- **FR-001**：新增独立 `scflow` 包，包含 A2F 全部 Python 计算模块；运行时不得导入 `external/a2f`。
- **FR-002**：CLI 必须读取 `[scflow]` 配置，并按配置文件目录解析相对路径。
- **FR-003**：支持 `*.dyn*.elph*`、可选 `*.dyn*` 和可选 `*.ph.out` 输入，保留原有权重回退逻辑。
- **FR-004**：保留 direct/a2f CSV、`result.json`、PDF 图、summary、CIF 和 VASP 结构等原有输出。
- **FR-005**：保留计算公式和结果字段；允许现代依赖兼容和有明确记录的精确零频处理；正频模公式保持不变。
- **FR-006**：在 pyproject 中注册 `scflow` 命令和独立 `scflow` 可选依赖组。

## 成功标准

- **SC-001**：公开 QE EPC 归档可通过 `scflow -c` 完整处理并写出 Tc 结果。
- **SC-002**：配置路径、默认值、非法输入和缺失文件均有自动化测试。
- **SC-003**：当前 Conda 环境下不再因 `simps`、CIF 文本模式或 NumPy 标量序列化失败。
- **SC-004**：真实用户归档运行后，结果可从 `result.json` 复核，原始 QE 文件保持不变。

## 范围外

- 不运行 QE，不重新计算 SCF、phonon 或 EPC。
- 不修改 `external/a2f` 子模块及其公开样例。
- 不改变现有 qeflow、vaspflow 和 substitutionflow 工作流。

实现计划见 [plan.md](plan.md)，任务见 [tasks.md](tasks.md)，输入输出契约见 [contracts/config-and-output.md](contracts/config-and-output.md)。

## 2026-09-23：ISIF4 完整真实数据验收

用户指定 SiP2-19p2/vasp_opt_ISIF4/Tc-236 作为本轮验收样例。本轮沿用原 Python 实现，完成全部 10 个展宽的 CLI、结构、CSV/JSON/PDF、汇总及输入不变性检查，不代表其他相的待办已通过。

- **FR-007**：直接识别 elph_dir/elph.inp_lambda.N、真实 *.dynN 和 split_ph.out.N；不能把 matdyn.out、备份或目录当作动力学矩阵。
- **FR-008**：精确 ω²=0 且 λ=0 的模按零贡献处理，保留原频率和原 γ 数组，在结果与日志中记录排除项。若该零频模 λ 非零则报错，不能自动丢弃。
- **FR-009**：各展宽计算状态彼此隔离，不共享结果字典。
- **SC-005**：直接读取指定原始目录，在 μ*=0.10 和现存 lambda.in 的 μ*=0.16 两组条件下均完成 10 个展宽；输出必须为有限值，并与独立公式和明确标注的历史诊断基线核对。
- **SC-006**：读取文件哈希不变，不编辑源数据、不替换原版 A2F、不调用 QE；交付可复现的测试记录。

处理精确零频是显式约定：该数据零频模的 λ=0，但 γ 非零，不能直接做 γ/ω²。此处不宣称恢复了真实低频谱，也不宣称所得 Tc 已对 q/k 网格及平滑参数收敛。

- **FR-010**：McMillan / Allen–Dynes 公式计算前将参数提升为 float64，避免 NumPy 2 在 float16 频率数组上计算 ωlog/1.2 引入额外舍入；不修改公式。独立 Python 标量公式必须通过数值核对。

- **FR-011**：Eliashberg 数值触及 0.001 K 搜索下限时，结果另标 resolution_limit；保留旧数值字段，不宣称得到实际 Tc。
