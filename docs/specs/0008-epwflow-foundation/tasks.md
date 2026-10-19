# 实施任务：配置、环境检查与受控执行基础

**输入**：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据模型](data-model.md)、[契约](contracts/interface.md)。

## 阶段 1：准备

- [ ] T001 核对本项原文及前置契约，记录到 docs/specs/0008-epwflow-foundation/acceptance.md。

## 阶段 2：基础能力

- [ ] T002 建立或扩展 epwflow/src/foundation/mod.rs 的模块边界与版本检查；依赖前置已验收接口。

## 阶段 3：US1 — 独立入口与公开操作

独立验收：执行 FR-001 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T003 [US1] 在 epwflow/tests/foundation.rs 添加 FR-001 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T004 [US1] 在 epwflow/src/foundation/cli.rs 实现 FR-001 全部行为，使用统一契约并执行前项测试。

## 阶段 4：US1 — 配置范围与严格校验

独立验收：执行 FR-002 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T005 [US1] 在 epwflow/tests/foundation.rs 添加 FR-002 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T006 [US1] 在 epwflow/src/foundation/config.rs 实现 FR-002 全部行为，使用统一契约并执行前项测试。

## 阶段 5：US1 — 路径与保存目录

独立验收：执行 FR-003 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T007 [US1] 在 epwflow/tests/foundation.rs 添加 FR-003 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T008 [US1] 在 epwflow/src/foundation/paths.rs 实现 FR-003 全部行为，使用统一契约并执行前项测试。

## 阶段 6：US2 — 输入绑定与指纹

独立验收：执行 FR-004 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T009 [US2] 在 epwflow/tests/foundation.rs 添加 FR-004 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T010 [US2] 在 epwflow/src/foundation/snapshot.rs 实现 FR-004 全部行为，使用统一契约并执行前项测试。

## 阶段 7：US2 — 环境证据分层

独立验收：执行 FR-005 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T011 [US2] 在 epwflow/tests/foundation.rs 添加 FR-005 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T012 [US2] 在 epwflow/src/foundation/environment.rs 实现 FR-005 全部行为，使用统一契约并执行前项测试。

## 阶段 8：US2 — 阶段成功与依赖

独立验收：执行 FR-006 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T013 [US2] 在 epwflow/tests/foundation.rs 添加 FR-006 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T014 [US2] 在 epwflow/src/foundation/state.rs 实现 FR-006 全部行为，使用统一契约并执行前项测试。

## 阶段 9：US3 — 严格提交与未知状态

独立验收：执行 FR-007 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T015 [US3] 在 epwflow/tests/foundation.rs 添加 FR-007 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T016 [US3] 在 epwflow/src/foundation/slurm.rs 实现 FR-007 全部行为，使用统一契约并执行前项测试。

## 阶段 10：US3 — 并发与持久恢复

独立验收：执行 FR-008 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T017 [US3] 在 epwflow/tests/foundation.rs 添加 FR-008 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T018 [US3] 在 epwflow/src/foundation/store.rs 实现 FR-008 全部行为，使用统一契约并执行前项测试。

## 阶段 11：US4 — 节点时间与并发预算

独立验收：执行 FR-009 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T019 [US4] 在 epwflow/tests/foundation.rs 添加 FR-009 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T020 [US4] 在 epwflow/src/foundation/budget.rs 实现 FR-009 全部行为，使用统一契约并执行前项测试。

## 阶段 12：US4 — 存储与空间检查

独立验收：执行 FR-010 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T021 [US4] 在 epwflow/tests/foundation.rs 添加 FR-010 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T022 [US4] 在 epwflow/src/foundation/storage.rs 实现 FR-010 全部行为，使用统一契约并执行前项测试。

## 阶段 13：US4 — 执行、参数与制品一致性

独立验收：执行 FR-011 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T023 [US4] 在 epwflow/tests/foundation.rs 添加 FR-011 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T024 [US4] 在 epwflow/src/foundation/process.rs 实现 FR-011 全部行为，使用统一契约并执行前项测试。

## 阶段 14：US5 — 状态、错误与完成语义

独立验收：执行 FR-012 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T025 [US5] 在 epwflow/tests/foundation.rs 添加 FR-012 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T026 [US5] 在 epwflow/src/foundation/status.rs 实现 FR-012 全部行为，使用统一契约并执行前项测试。

## 阶段 15：US5 — F1 独立交付与共享责任

独立验收：执行 FR-013 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T027 [US5] 在 epwflow/tests/foundation.rs 添加 FR-013 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T028 [US5] 在 epwflow/src/foundation/stage.rs 实现 FR-013 全部行为，使用统一契约并执行前项测试。

## 最终阶段：成功标准与交付

- [ ] T029 [US5] 核验 SC-001 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T030 [US5] 核验 SC-002 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T031 [US5] 核验 SC-003 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T032 [US5] 核验 SC-004 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T033 [US5] 核验 SC-005 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T034 [US5] 核验 SC-006 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T035 [US5] 核验 SC-007 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0008-epwflow-foundation/acceptance.md；真实项目无证据不得勾选。
- [ ] T036 完成 epwflow/README.md、docs/overview/epwflow/ 的已实现行为回写与 quickstart 验证，并在本项 acceptance.md 收敛差距。

## 依赖与执行顺序

T001→T002 后按故事和前置任务顺序执行；同一共享文件串行，不设置无依赖依据的 [P] 标记。先测试、再实现、再真实验收。F1→F2→F3→F4→F5，前置未满足的真实接口不得放行。MVP 为本项 P1 故事的可验证闭环，不能以 MVP 代替所有任务。

## 需求覆盖

| 来源 | 任务 | 设计/文件 | 验证 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/foundation/cli.rs | epwflow/tests/foundation.rs |
| FR-002 | T005/T006 | epwflow/src/foundation/config.rs | epwflow/tests/foundation.rs |
| FR-003 | T007/T008 | epwflow/src/foundation/paths.rs | epwflow/tests/foundation.rs |
| FR-004 | T009/T010 | epwflow/src/foundation/snapshot.rs | epwflow/tests/foundation.rs |
| FR-005 | T011/T012 | epwflow/src/foundation/environment.rs | epwflow/tests/foundation.rs |
| FR-006 | T013/T014 | epwflow/src/foundation/state.rs | epwflow/tests/foundation.rs |
| FR-007 | T015/T016 | epwflow/src/foundation/slurm.rs | epwflow/tests/foundation.rs |
| FR-008 | T017/T018 | epwflow/src/foundation/store.rs | epwflow/tests/foundation.rs |
| FR-009 | T019/T020 | epwflow/src/foundation/budget.rs | epwflow/tests/foundation.rs |
| FR-010 | T021/T022 | epwflow/src/foundation/storage.rs | epwflow/tests/foundation.rs |
| FR-011 | T023/T024 | epwflow/src/foundation/process.rs | epwflow/tests/foundation.rs |
| FR-012 | T025/T026 | epwflow/src/foundation/status.rs | epwflow/tests/foundation.rs |
| FR-013 | T027/T028 | epwflow/src/foundation/stage.rs | epwflow/tests/foundation.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |

## 2026-10-16 实现收敛

- [ ] T037 依据 0015 B01～B09 修复本 Feature 的真实生产路径，取消已通过回归的 ignore；核对其余 FR/SC、真实程序输出和来源，更新验收与当前功能文档。原已勾选项不能替代本轮复核。实现与测试均由本助手承担。

## 2026-10-19：本机输入文件名

- [x] T038 按最新答复保留 `inputepw.toml` 与原 qeflow `input.toml`，同步 Issue、Spec、接口、流程图、手册与 Overview；以实际只读配置检查、基础回归及文档校验记录验收，不修改科学实现。证据见 acceptance.md 的同日输入文件名记录。

## 三个独立程序整合

- [x] T039 提取 qeflow 现有实现为库；修复 nosplit 本地 q2r／matdyn 漏执行；实现可选 EPW 数据保留和完整性归档，验证既有行为。
- [x] T040 新增仅三个独立 EPW 程序及各阶段检查／脚本／提交／恢复，迁移测试和构建安装入口。
- [ ] T041 拆开粗网格拟合与细网格 EPC，保证第二阶段四类图及质量门、第三阶段原生两模式与图表。
- [ ] T042 在指定 local_run 编号目录验证实际数据、脚本和科学链；完成文档与逐要求审计，缺真实证据不勾选。

- [x] T043 独立粗网格清单与细网格交接，验证粗阶段不执行细网格、清单不变及篡改阻断；自动参考／四类图仍归 T041。
