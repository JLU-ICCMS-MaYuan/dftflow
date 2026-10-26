# 0057 实施计划

## 实施基线

重新对照 [Spec](spec.md) 的完整原话、FR 与 SC，遵守 [设计依据](research.md)。

## 方案

独立 clear 模块只读取布局、JSON 状态和现用 JSON／TOML／脚本中的引用；不加载 Config，也不调用带账本结算副作用的 check_active。固定三个相对目录，逐级拒绝链接祖先，先检查全部目标、任务、引用和当前目录，再删除。目录遍历使用 symlink_metadata，不跟随链接。保留存在的 history 根，删除其子项。先完整预检，删除前再次检查；删除失败尽可能报告剩余路径并返回错误，不把逻辑字节当作实际回收磁盘空间。不提供任意路径、force、保留周期或自动后台清理。

## 需求与验证映射

- FR-001 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-002 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-003 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-004 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-005 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-006 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-007 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。
- FR-008 → 上述方案 → T001／T002／T003 → [验收](acceptance.md)。

- SC-001 → T003／T004 → [验收](acceptance.md)。
- SC-002 → T003／T004 → [验收](acceptance.md)。
- SC-003 → T003／T004 → [验收](acceptance.md)。
- SC-004 → T003／T004 → [验收](acceptance.md)。
- SC-005 → T003／T004 → [验收](acceptance.md)。
- SC-006 → T003／T004 → [验收](acceptance.md)。

## 验证与交付

公开 CLI 临时工作区测试覆盖三类历史、默认预览、真实删除、缺失目录、幂等、链接逃逸、现用引用、活动／未知任务、拒绝不完整状态、删除失败；真实 MgB₂ 先记录现用文件哈希和只读复用状态再清理、核对。

| 标准 | 生产边界与用例 |
| --- | --- |
| SC-001／002 | `clear_history` 公开 CLI 的默认／显式预览、三类清理、缺失目录及幂等 |
| SC-003 | 临时现场现用文件／预算保护，以及真实 MgB₂ 清理前后 SHA-256、目录 inode 和只读状态核对 |
| SC-004 | 现用记录和符号链接引用、未知／活动作业、独立提交回执、旧尝试快照与当前作业的职责区分 |
| SC-005 | 带空格工作区、链接祖先与外部目标、普通用户权限导致的真实删除失败 |
| SC-006 | `scripts/tests/build-publish.sh`、`scripts/tests/install-bin.sh`、`scripts/tests/env.sh` 与实际 PATH 解析 |

与另一项串行实施：0056 → 0057。共享 workspace、模块注册及文档按任务差异分别提交；保持其他任务和 AGENTS.md 原有改动，不推送。科学证据、安装证据与契约测试分别记录。
