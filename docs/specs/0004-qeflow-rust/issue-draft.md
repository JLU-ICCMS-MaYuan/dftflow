# Issue 0004 与 P0–P5 子 Issue 草稿

> 本文件仅是本地草稿。现有 Gitee [IKKGLP（本地编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP) 在线上是否已调整为 Epic、P0–P5 是否创建、真实编号及双向链接，均**尚未验证或回填**。线上创建、标签、关系与关闭由用户自管；本次文档修改不操作 Gitee。实际类型标签须以该仓库可用的 Gitee 标签为准，不把 `type:feature` 草稿写成已生效标签。

原始需求逐字内容见 [spec.md](spec.md) 的「原始需求记录」；本文件以下均为经过澄清的摘要，不冒充逐字用户原话。现有 0004 Spec 暂保存跨阶段约束；P1–P4 取得线上真实 Issue 编号后，分别建立编号一致的 Feature Spec，完成文档门才启动相应实现。

## Epic 0004：dftflow Rust 全量迁移

- **预期类型**：Epic；线上标签与状态待用户核验。
- **目标**：五个 Rust 家族二进制覆盖 48 个既有 CLI；功能、配置、状态、文件和绘图对照；最终移除被替换的 Python 包。
- **例外**：`mattersim_opt` 继续作为 Python CLI，允许为去除 `public` 依赖作最小等价调整；启用可选 MatterSim 步骤时仍使用 Python。`test_auto/` pytest 保留。
- **范围外**：不修改外部 QE/VASP/LOBSTER/vaspkit，不引入数据库或 daemon，不自动改主分支。
- **子 Issue**：P0–P5 均为**待创建草稿**，无真实编号与链接。Epic→子 Issue 的线上双向 Markdown 链接由用户建立后回填。
- **Documentation Impact**：本总体计划、各子 Feature Spec、最终 README 与已实现 Overview；不得把未实现规划写入 Overview。

## P0：基线、workspace 与静态可行性

- **预期类型**：maintenance；**真实 Issue 编号**：待用户回填。
- **目标**：纳入被忽略的 11 个 pytest 文件、冻结参考提交与输入哈希；根 workspace 保持 `qeflow-rust/` 过渡命名；验证静态 OpenBLAS、spglib、PDF 与 CIF。
- **范围外**：不删除、不改名 Python `qeflow/`；不实现全量 flow CLI。
- **依赖**：Epic 0004；其结果阻断 P1。
- **验收**：参考 pytest 和 Linux musl 静态样例均可复核；无目录冲突；不存在的 `validate_scflow_isif4.py` 不再作为验收命令。
- **Documentation Impact**：更新 P0 研究和验收证据；Overview 仅记录实际落地事实。

## P1：共享 crate

- **预期类型**：feature；**真实 Issue 编号及 Feature Spec**：待用户回填后创建。
- **目标**：配置、解析、状态、执行、绘图和测试对照器，统一共享边界。
- **范围外**：不删除 Python 包，不以测试骨架宣称各 flow 已迁移。
- **依赖**：P0 验收通过。
- **验收**：真实 QE/VASP/LOBSTER 样例及失败边界通过集成测试；结构与数值对照器可运行。
- **Documentation Impact**：新编号 Feature Spec、契约与实现后的 Overview。

## P2：qeflow 与 scflow

- **预期类型**：feature；**真实 Issue 编号及 Feature Spec**：待用户回填后创建。
- **目标**：17 个 QE 命令、`qf-gen-dyn0` 整合和独立 scflow；覆盖声子、真实 EPC、Tc、CIF、CSV 与 PDF。
- **范围外**：本阶段不删除 Python `qeflow/`、`scflow/`，因 P4 前仍有 Python 反向依赖。
- **依赖**：P1 验收通过。
- **验收**：冻结 Python pytest 与 Rust 集成测试分别通过；零频、ISIF4、下限状态、Slurm 和图片对照完成。
- **Documentation Impact**：新编号 Feature Spec 与实际实现后的 QE/scflow Overview。

## P3：vaspflow

- **预期类型**：feature；**真实 Issue 编号及 Feature Spec**：待用户回填后创建。
- **目标**：19 个 VASP 命令，含 `vasp_elf`、COHPCAR 手写解析、数值产物与图。
- **范围外**：本阶段不删除仍被其他 Python 包引用的 `public/` 或 Python `vaspflow/`。
- **依赖**：P2 验收通过。
- **验收**：输入卡、退出行为、CSV/DAT、PNG 与 Python 参考一致或满足预定浮点容差。
- **Documentation Impact**：新编号 Feature Spec 与实现后的 VASP Overview。

## P4：wannier90flow、substitutionflow 与 MatterSim 接线

- **预期类型**：feature；**真实 Issue 编号及 Feature Spec**：待用户回填后创建。
- **目标**：4 个 Wannier 与 7 个 substitution 命令；覆盖跨 QE/VASP 脚本、OUTCAR 状态、绘图及可选 MatterSim 链路。
- **范围外**：不把 `mattersim_opt` 本身改写为 Rust；Python 改动只为去除 `public` 依赖。
- **依赖**：P2、P3 验收通过。
- **验收**：有/无 MatterSim 两条链路均通过；`mattersim_opt` CLI 与生成脚本保持兼容。
- **Documentation Impact**：新编号 Feature Spec、Python 例外登记与实现后的 Overview。

## P5：集中切换与交付

- **预期类型**：maintenance；**真实 Issue 编号**：待用户回填。
- **目标**：反向依赖清零后统一删除被替换 Python、改名 Rust 目录，发布五个静态二进制与 48 个旧命令兼容入口。
- **范围外**：不删除 `mattersimflow/` 或 `test_auto/`；不自动关闭线上 Epic。
- **依赖**：P0–P4 全部验收通过，所有 Feature Spec 与文档门完成。
- **验收**：Linux 离线静态运行、Python pytest 与 Rust 集成测试、图像/CIF/数值/状态、可选 MatterSim、回滚均通过。
- **Documentation Impact**：根 README、AGENTS、Overview、配置契约与用户安装说明。

## 待回填状态

| 草稿 | 线上编号 | 线上链接 | 双向关系 | 本地 Feature Spec |
| --- | --- | --- | --- | --- |
| P0 | 待创建 | 待创建 | 待建立 | maintenance，按需 |
| P1 | 待创建 | 待创建 | 待建立 | 待编号 |
| P2 | 待创建 | 待创建 | 待建立 | 待编号 |
| P3 | 待创建 | 待创建 | 待建立 | 待编号 |
| P4 | 待创建 | 待创建 | 待建立 | 待编号 |
| P5 | 待创建 | 待创建 | 待建立 | maintenance，按需 |
