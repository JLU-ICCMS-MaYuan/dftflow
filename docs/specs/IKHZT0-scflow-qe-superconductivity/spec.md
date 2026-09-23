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
- **FR-005**：保留计算公式和结果字段；仅增加 SciPy、pymatgen 和 NumPy 现代版本兼容处理。
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
