# Issue 0004 与 P0–P5 子 Issue 草稿

> **命名说明**：本 Spec 目录原为 `docs/specs/0004-qeflow-rust/`（0004 最初仅覆盖 qeflow 的 Rust 重写）。2026-10-10 范围扩大为 dftflow 全量 Rust 迁移后，2026-10-12 将目录更名为 `0004-dftflow-rust`、`docs/qeflow-rust/` 更名为 `docs/dftflow-rust/`、`docs/overview/qeflow-rust.md` 更名为 `docs/overview/dftflow-rust.md`，使命名与 Epic 实际范围（dftflow 全量迁移）一致。历史条目中的 `qeflow-rust/` 旧目录与 `qeflow-rust` 分支名为当时事实，不作改写。

## 原始需求（用户原话）

### [2026-10-12] phase0-restructure 任务下发

> 请开始完成任务 phase0-restructure：
> 重排 Rust workspace：包名改 dftflow、多二进制骨架、引入 spglib/plotters 与 musl CI（用 [skill:big-project-issue-manager] 更新 Issue 0004）

### [2026-10-12] 追问答复：「包名改 dftflow」含义

> 仅文档统称(不改任何包/分支名，仅在 Issue 0004 文档中把交付物家族统称 dftflow，本次只补 musl.yml 与 Issue 状态)

> 本文件仅是本地草稿。现有 Gitee [IKKGLP（本地编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP) 在线上是否已调整为 Epic、P0–P5 是否创建、真实编号及双向链接，均**尚未验证或回填**。线上创建、标签、关系与关闭由用户自管；本次文档修改不操作 Gitee。实际类型标签须以该仓库可用的 Gitee 标签为准，不把 `type:feature` 草稿写成已生效标签。

原始需求逐字内容见 [spec.md](spec.md) 的「原始需求记录」；本文件以下均为经过澄清的摘要，不冒充逐字用户原话。现有 0004 Spec 暂保存跨阶段约束；P1–P4 取得线上真实 Issue 编号后，分别建立编号一致的 Feature Spec，完成文档门才启动相应实现。

## Epic 0004：dftflow Rust 全量迁移

- **预期类型**：Epic；线上标签与状态待用户核验。
- **目标**：五个 Rust 家族二进制覆盖 48 个既有 CLI；功能、配置、状态、文件和绘图对照；最终移除被替换的 Python 包。
- **交付物家族统称**：文档与沟通层面将五个家族二进制（qeflow/vaspflow/scflow/wannier90/substitution）统称 dftflow；不改任何包名、目录名与分支名（用户 2026-10-12 澄清「仅文档统称」）。
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
- **现状（2026-10-12）**：T001–T003 已完成（11 个 pytest 文件纳管、冻结标签 `python-baseline-0004`@`9396b3a`、共享 crate 归位根 `crates/` 且多二进制骨架齐备：qeflow/vaspflow/scflow/wannier90/substitution）；spglib/plotters 已入 workspace 依赖。T004/T005（Linux musl 静态样例、spglib/PDF 可行性）待 Linux 节点验证，本机 macOS 无法完成。musl.yml 的 strip 列表已由 2 个补齐为 5 个二进制并同步 `ls -lh` 清单；分支触发保持 `qeflow-rust`（用户澄清不改分支名）。

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

## 收尾状态（2026-10-13）

P0–P5 已在本机（macOS）完成：48 个旧命令入口就绪（`scripts/install-compat.sh` + argv[0] 分派）、被替换 Python 全部删除、目录改名完成、pyproject 仅保留 `mattersim_opt`；Rust 集成测试 75 项全绿，关键后处理与 Python 参考逐字节对照通过。**待 Linux 节点项**：musl 静态构建/strip/离线运行（T004/T005/T030）、pytest 黄金基线在参考工作树（`python-reference-0004`）运行。线上 Gitee Issue（IKKGLP 及子 Issue 编号）仍由用户自管，本文档仅作本地双向链接记录。


## [2026-10-14] 缺口补齐原始需求

> 填补完成所有实质缺口：完成 qe 基础命令完整迁移，scflow 完成 openbls 接入测试，openblas 我已经安装好了。完成数值验收测试。完成命令环境切换。我只能在 macos 上完成 linux 交付。我不明白为什么一定要在 linux 上测试。他们只要编译环境一样，能保证 rust 编译为二进制文件。我直接把二进制文件拷贝到 linux 机器上运行不就行了。完成其余二进制的收敛。
>
> 测试体系依然是 MgB2. 继续使用 issue 和 spec 完成。

### 本轮实施结论

用户已授权补齐并测试：沿用 0004 与 MgB2 基线，Rust 实现 SCF/NSCF 输入、运行、提交及参数兼容；接入 OpenBLAS 并去除数值豁免；安装五个家族二进制和 48 个旧入口加 dyn0 别名。macOS 上通过 Linux amd64 容器构建 musl 产物并作离线验证，无需远程 Linux 开发机。真实 Slurm/QE/VASP 集群运行与容器离线验证分别记录。线上 Gitee 仍由用户管理。


### 2026-10-14 收敛结果（供用户同步线上 Issue）

已完成本轮 T032–T036：QE 四个基础计算命令完整执行；OpenBLAS 接入并去除 Tc/派生量豁免；Cargo 收敛为五个程序、52 个入口检查通过；当前 zsh 环境切换至 Rust。MgB2 Python/Rust 真实 SCF/NSCF/能带/DOS 跑通，16 张数值表解析值完全一致。

macOS 和 Linux musl 各 88 项 Rust 测试通过，Linux 断网重跑 88 项通过。五个 Linux x86_64 静态 ELF 已 strip，无动态依赖；交付包及哈希清单在 `dist/`。最新完整证据见 `acceptance-20261014.md`。本次没有操作或关闭线上 Gitee Issue。
