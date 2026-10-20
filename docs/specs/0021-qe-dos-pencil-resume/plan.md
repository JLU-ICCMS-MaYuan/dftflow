# 实施计划：DOS pencil 与重提提示

**Issue**：[本地 0021](issue-draft.md)。**Spec**：[spec.md](spec.md)。日期：2026-10-20。

## 技术上下文与职责

Rust workspace／Bash、anyhow／serde；沿用 foundation/workspace_science.rs 的 post_program 单一策略和 workspace.rs 的 Attempt。Slurm／Linux x86_64，32 MPI；无新存储或外部接口。
tests/three_stage_execution.rs 通过公开 CLI、实际生成脚本及受控求解器验证，不增加科学作业。

## 质量门与需求映射

| 来源 | 事实及设计 | 任务／验证 |
| --- | --- | --- |
| FR-001/002、SC-001 | post_program 仅 bands 和 dos 使用 pencil，执行和键共用 | T002/T003 |
| FR-003、SC-002 | 延用原生成功任务收据，公开 -s 创建新尝试 | T002/T004 |
| FR-004、SC-003 | 保留作业绑定与脚本哈希，冲突消息包含两个作业号和恢复命令 | T003/T004 |
| FR-005、SC-004 | 来源哈希、规范构建安装、文档及分层验收 | T001/T005 |
| AGENTS／0019 | 无严格 shell 选项、显式失败传播、只提交本任务 | T004/T005 |

## 阶段与文件

T001 保存证据和完整文档，T002 先建失败复现测试，T003 最小源代码修复，T004 串行回归，T005 更新文档、bin/epwflow、核验并提交。
本目录保存规格、计划、研究、数据说明、任务、quickstart、checklists 和 acceptance；不新增公共 CLI 契约文件，因为接口完全不变。

## 复杂度和边界

不增加重试状态机、不克隆尝试、不修改旧绑定；旧脚本继续失败，指导新命令创建新尝试。KISS／DRY 通过单一参数入口保持生成、执行和收据一致。
