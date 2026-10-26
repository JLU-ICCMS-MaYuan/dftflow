# 0056 实施计划

## 实施基线

重新对照 [Spec](spec.md) 的完整原话、FR 与 SC，遵守 [设计依据](research.md)。

## 方案

公共 archive_step 将阶段目录的直接子项逐个移动到历史的 calculation/，不重命名阶段根。先保存现用阶段脚本；所有移动成功后恢复脚本并继续原来的状态失效过程。当前 Slurm 的日志通过硬链接备份到归档，原位置不移动；其余条目移动前检查目标不存在。错误通过上下文给出源／目标，向上传播阻止计算。归档中断保留已移动内容；下次显式恢复保留现存历史并归档剩余内容，不自动合并或覆盖历史。

## 需求与验证映射

- FR-001 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-002 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-003 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-004 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-005 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-006 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。

- SC-001 → T003／T004 → [验收](acceptance.md)。
- SC-002 → T003／T004 → [验收](acceptance.md)。
- SC-003 → T003／T004 → [验收](acceptance.md)。
- SC-004 → T003／T004 → [验收](acceptance.md)。
- SC-005 → T003／T004 → [验收](acceptance.md)。

## 验证与交付

持有阶段目录句柄的子 shell、打开日志持续写入、目录与日志 inode、旧文件哈希、冲突及权限错误、已有 SCF 显式重跑与阶段链契约。

| 标准 | 生产边界与用例 |
| --- | --- |
| SC-001／002 | `archive_contents` 的驻留 shell、目录 inode、旧输入输出字节测试；实际调用来自 `archive_step` |
| SC-003 | 打开的写入句柄与历史硬链接测试；公开 `numbered_stage_archive_preserves_the_open_slurm_log` |
| SC-004 | `explicit_scf_resume_recomputes_success_each_time_and_blocks_running_jobs`、声子及粗细网格公开回归 |
| SC-005 | `destination_conflict_stops_before_moving_any_source`、`numbered_archive_failure_does_not_start_scientific_programs` |

与另一项串行实施：0056 → 0057。共享 workspace、模块注册及文档按任务差异分别提交；保持其他任务和 AGENTS.md 原有改动，不推送。科学证据、安装证据与契约测试分别记录。
