# 实施计划：QE 分阶段即时出图

**Issue**：[0047 本地草案](issue-draft.md)；线上由用户管理。
**日期**：2026-10-22
**Spec**：[spec.md](spec.md)

## 摘要

在科学子任务校验完成后绘制 QE 单独图；在复用阶段时补齐缺图。图像和派生表放在阶段内 `plots/`，独立于科学阶段完成证据，避免删除图像触发科学重算。第 5 步电子叠图保持，声子 DOS 对比只画 TDOS。

## 技术上下文

- 语言：现有 Rust 2021 与 Bash；不新增生产 Python。
- 依赖：现有 Plotters、qf-phonon、qf-input、qf-io、serde 和 store；不新增依赖。
- 数据：已校验 QE 原生 XML、TDOS、matdyn 频率及逐原子投影、结构与路径。电子能量使用同一 NSCF Fermi 零点。
- 目标：Linux／Slurm 编号工作区；现有三个公开程序和 -r／-s 流程。
- 测试：Rust 行为测试与现有受控科学程序替身；真实数据只读出图，不提交新科学作业。
- 性能：图像完备且来源相容时直接复用；缺图只补图，不新增科学程序调用。

## 质量门

| 来源 | 要求 | 实现方式 | 状态 |
|---|---|---|---|
| 用户最新指令 | 开始执行并修改代码 | 仅实施 0047 已列范围，不重复索要实施许可 | 通过 |
| AGENTS.md | KISS／DRY／任务提交隔离 | 复用既有绘图和阶段边界；不引入新调度框架 | 通过 |
| FR-003 | 电子不等待声子 | 电子叶任务结束即绘图，读取电子来源 | 通过 |
| FR-005 | 完整可靠投影 | 绘图前要求 DOS 列数等于 2＋结构原子数 | 通过 |
| FR-010 | 补图不重算 | plots 独立派生清单，科学证据排除 plots | 通过 |
| 平台约定 | 线上由用户管理 | 保留本地正文与远程链接待验证，不操作线上 | 记录边界 |

用户直接要求实施优先于重复确认流程；不得把尚未核验的远程链接写成完成。以上为设计门，测试证据另写 acceptance.md。

## 文档与源码结构

- 设计文档：本 Plan、[Research](research.md)、[数据契约](data-model.md)、[接口](contracts/plots.md)、[Tasks](tasks.md)、[验证路径](quickstart.md)。
- `foundation/workspace_qe_plot.rs`：仅负责单侧图来源、派生清单和补图。
- `foundation/workspace_plot.rs`：复用电子绘图逻辑，增加单侧电子渲染。
- `foundation/workspace_electronic.rs`：成功／复用叶任务即触发该任务绘图。
- `foundation/workspace_science.rs`：声子数值后处理检查点与绘图衔接，派生产物不进入科学证据。
- `foundation/workspace.rs`、`native_workspace.rs`：阶段复用时补图，不改提交协议。
- `delivery/comparison_plot.rs` 与 `interpolation/comparison.rs`：声子 TDOS 单面板，不报告未要求的声子投影缺项。
- `tests/support/qe_stage_plots.rs`、`tests/comparison_plots.rs`：公开进程、恢复与 SVG 图层检查。

## 需求到设计的映射

| 来源 | 决策／组件与数据来源 | 任务／证据 |
|---|---|---|
| FR-001～003／SC-001 | D1：电子叶任务，原生 PW XML 与 TDOS，统一 NSCF Fermi | T002／T003，公开 -r 与 -s 测试 |
| FR-004～005／SC-002 | D2：qf-phonon 共用绘图，完整逐原子 DOS＋结构映射 | T004／T005，元素表与缺投影拒绝 |
| FR-006～009／SC-003～004 | D3：保留电子叠图；声子 TDOS 比较单面板 | T006，原电子对比回归与新 SVG 断言 |
| FR-010／SC-005 | D4：plots 派生清单、数值后处理检查点及复用入口 | T003／T005，删除图片和绘图失败恢复时调用计数不增加 |
| FR-011／SC-006 | D1～D4：只读原始数据，单侧表可追溯；检查真图 | T007，来源哈希与图像实看 |

## 阶段与依赖

先完成 T001 文档和基线，再执行 US1、US2、US3，最后集成验证、事实文档和范围提交。同一文件串行编辑。新增独立清单仅管理派生图像，不增加隐藏目录、锁或历史迁移框架。
