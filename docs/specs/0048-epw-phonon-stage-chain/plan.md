# 实施计划：声子阶段一次提交完整交付

**Issue**：[0048 本地草案](issue-draft.md)；线上登记由用户管理。
**日期**：2026-10-22
**Spec**：[spec.md](spec.md)

## 摘要

复用现有编号工作区尝试、固定快照与预算登记；仅声子提交改为一个分配内自动顺序执行。复用 native_workspace 的阶段依赖、native_jobs 原生任务回执和 0047 QE 绘图。所有科学调用依然生成可读脚本与独立 .out/.err，但在已登记分配中同步运行，不再次 sbatch。

## 技术上下文

- 语言：Rust 2021、Bash；测试替身用现有 Python 例外。
- 依赖：现有 qeflow、qf-phonon、Slurm、serde；不引入新依赖。
- 数据：epw-state/attempts、native-progress、native-jobs 及阶段 completion.json，全部可见。
- 测试：epwflow/tests/three_stage_execution.rs 的公开进程测试及真实脚本执行；构建使用 scripts/build-native.sh。
- 平台：Linux／Slurm；单分配按配置的 MPI／节点／时限执行，不在登录节点跑科学程序。
- 性能与规模：一次 sbatch，成功子任务不重跑，无轮询守护进程。
- 约束：固定配置及输入、预算不重置、不注入 shell 严格选项；0046 固定脚本与日志格式；无新公共 CLI。

## 质量门

| 来源 | 要求 | 方案 | 状态 |
|---|---|---|---|
| 用户最新授权 | 一次声子命令完成全部 | 单分配自动调用既有阶段与图像 | 设计通过 |
| 0026／0048 | 其他目标保持逐任务提交 | 仅 Phonon 提交路由使用自动链 | 设计通过 |
| 0047 | QE 单侧真实元素 PDOS | 复用 workspace_qe_plot；缺投影拒绝 | 设计通过 |
| AGENTS.md | 不混入已有改动、验证后自动提交 | 保存任务起始共享文件，精确暂存任务增量 | 设计通过 |
| 平台约定 | 本地文档不冒充线上完成 | 远程链接单列未完成 | 线上门待验证 |

## Feature 文档结构

spec.md、plan.md、research.md、data-model.md、contracts/cli.md、quickstart.md、tasks.md、checklists/requirements.md、acceptance.md。

## 源代码结构

- foundation/program.rs：仅声子目标提交路由与帮助。
- foundation/workspace.rs：复用 Attempt 快照、分配登记、完成与失败状态。
- foundation/native_workspace.rs：复用阶段遍历，加入已登记分配执行入口。
- foundation/native_jobs.rs：已登记分配内执行原生脚本，保存每项可复用回执。
- tests/support/phonon_chain.rs：公开进程单提交、失败恢复、旧结果与状态测试。

## 需求到设计的映射

| 来源 | 设计／事实来源 | 任务 | 验证 |
|---|---|---|---|
| FR-001／006／011，SC-001／006 | program 声子路由、Attempt.job／state、单分配 worker | T002／T003 | 单次调用，执行脚本后不再推进即完整；其他目标回归 |
| FR-002／003，SC-001／004 | 既有 workspace_science 与 QeManifest.verify | T003／T004 | 完整 save 清单和 q 映射 |
| FR-004／005，SC-004 | 0047 workspace_qe_plot 与真实逐原子列 | T004 | 图表与缺投影拒绝 |
| FR-007～010，SC-002／003／005／006 | native-progress generation、native-jobs 输出指纹、预算及原始输入 | T003／T004 | 失败后恢复、合法检查点不重跑、防重复与来源破坏拒绝 |
| FR-012，SC-007 | README、Overview、quickstart 流程图 | T005 | 使用说明及相对链接检查 |

## 阶段与依赖

T001 文档分析 → T002 公开入口及分配 → T003 原生连续执行 → T004 集成回归与修复 → T005 文档与本地提交。

## 复杂度说明

复用现有尝试和子任务记录，避免再建独立工作流状态机。必须由分配内 Rust 工作程序执行数据校验和绘图；单纯 shell 串联科学程序不能自动发布严格交付。拒绝后台轮询／嵌套提交方案，避免额外生命周期及预算身份。
