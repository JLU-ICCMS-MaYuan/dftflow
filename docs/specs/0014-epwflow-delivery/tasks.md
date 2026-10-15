# 实施任务：图表、真实对照与安装交付

**输入**：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据模型](data-model.md)、[契约](contracts/interface.md)。

## 阶段 1：准备

- [ ] T001 核对本项原文及前置契约，记录到 docs/specs/0014-epwflow-delivery/acceptance.md。

## 阶段 2：基础能力

- [ ] T002 建立或扩展 epwflow/src/delivery/mod.rs 的模块边界与版本检查；依赖前置已验收接口。

## 阶段 3：US1 — 完整入口与独立结果命令

独立验收：执行 FR-001 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T003 [US1] 在 epwflow/tests/delivery.rs 添加 FR-001 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T004 [US1] 在 epwflow/src/delivery/cli.rs 实现 FR-001 全部行为，使用统一契约并执行前项测试。

## 阶段 4：US1 — 完整结果包

独立验收：执行 FR-002 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T005 [US1] 在 epwflow/tests/delivery.rs 添加 FR-002 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T006 [US1] 在 epwflow/src/delivery/collect.rs 实现 FR-002 全部行为，使用统一契约并执行前项测试。

## 阶段 5：US1 — 图种、格式与物理含义

独立验收：执行 FR-003 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T007 [US1] 在 epwflow/tests/delivery.rs 添加 FR-003 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T008 [US1] 在 epwflow/src/delivery/plot.rs 实现 FR-003 全部行为，使用统一契约并执行前项测试。

## 阶段 6：US2 — 可重复收集与独立重绘

独立验收：执行 FR-004 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T009 [US2] 在 epwflow/tests/delivery.rs 添加 FR-004 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T010 [US2] 在 epwflow/src/delivery/report_store.rs 实现 FR-004 全部行为，使用统一契约并执行前项测试。

## 阶段 7：US2 — 一次入口、结果状态与恢复

独立验收：执行 FR-005 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T011 [US2] 在 epwflow/tests/delivery.rs 添加 FR-005 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T012 [US2] 在 epwflow/src/delivery/scheduler.rs 实现 FR-005 全部行为，使用统一契约并执行前项测试。

## 阶段 8：US2 — 直接手册与算例组织

独立验收：执行 FR-006 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T013 [US2] 在 epwflow/tests/delivery.rs 添加 FR-006 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T014 [US2] 在 epwflow/README.md 实现 FR-006 全部行为，使用统一契约并执行前项测试。

## 阶段 9：US3 — 六组合真实验收

独立验收：执行 FR-007 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T015 [US3] 在 epwflow/tests/delivery.rs 添加 FR-007 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T016 [US3] 在 epwflow/src/delivery/acceptance.rs 实现 FR-007 全部行为，使用统一契约并执行前项测试。

## 阶段 10：US3 — 数值标准继承与证据判定

独立验收：执行 FR-008 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T017 [US3] 在 epwflow/tests/delivery.rs 添加 FR-008 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T018 [US3] 在 epwflow/src/delivery/compare.rs 实现 FR-008 全部行为，使用统一契约并执行前项测试。

## 阶段 11：US4 — 统一预算与验收数据保护

独立验收：执行 FR-009 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T019 [US4] 在 epwflow/tests/delivery.rs 添加 FR-009 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T020 [US4] 在 epwflow/src/delivery/budget.rs 实现 FR-009 全部行为，使用统一契约并执行前项测试。

## 阶段 12：US4 — 六程序、固定入口与平台构建

独立验收：执行 FR-010 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T021 [US4] 在 epwflow/tests/delivery.rs 添加 FR-010 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T022 [US4] 在 scripts/build-native.sh、scripts/linux/package-native-runtime.sh、Cargo.toml 实现 FR-010 全部行为，使用统一契约并执行前项测试。

## 阶段 13：US4 — 安装保护与制品核验

独立验收：执行 FR-011 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T023 [US4] 在 epwflow/tests/delivery.rs 添加 FR-011 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T024 [US4] 在 scripts/install-current.sh、scripts/install-compat.sh、scripts/verify-entries.sh 实现 FR-011 全部行为，使用统一契约并执行前项测试。

## 阶段 14：US5 — 验证方法与回归

独立验收：执行 FR-012 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T025 [US5] 在 epwflow/tests/delivery.rs 添加 FR-012 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T026 [US5] 在 epwflow/src/delivery/regression.rs 实现 FR-012 全部行为，使用统一契约并执行前项测试。

## 阶段 15：US5 — 文档与 Epic 完成条件

独立验收：执行 FR-013 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T027 [US5] 在 epwflow/tests/delivery.rs 添加 FR-013 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T028 [US5] 在 epwflow/README.md 实现 FR-013 全部行为，使用统一契约并执行前项测试。

## 最终阶段：成功标准与交付

- [ ] T029 [US5] 核验 SC-001 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T030 [US5] 核验 SC-002 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T031 [US5] 核验 SC-003 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T032 [US5] 核验 SC-004 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T033 [US5] 核验 SC-005 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T034 [US5] 核验 SC-006 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T035 [US5] 核验 SC-007 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T036 [US5] 核验 SC-008 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0014-epwflow-delivery/acceptance.md；真实项目无证据不得勾选。
- [ ] T037 完成 epwflow/README.md、docs/overview/epwflow/ 的已实现行为回写与 quickstart 验证，并在本项 acceptance.md 收敛差距。

## 依赖与执行顺序

T001→T002 后按故事和前置任务顺序执行；同一共享文件串行，不设置无依赖依据的 [P] 标记。先测试、再实现、再真实验收。F1→F2→F3→F4→F5，前置未满足的真实接口不得放行。MVP 为本项 P1 故事的可验证闭环，不能以 MVP 代替所有任务。

## 需求覆盖

| 来源 | 任务 | 设计/文件 | 验证 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/delivery/cli.rs | epwflow/tests/delivery.rs |
| FR-002 | T005/T006 | epwflow/src/delivery/collect.rs | epwflow/tests/delivery.rs |
| FR-003 | T007/T008 | epwflow/src/delivery/plot.rs | epwflow/tests/delivery.rs |
| FR-004 | T009/T010 | epwflow/src/delivery/report_store.rs | epwflow/tests/delivery.rs |
| FR-005 | T011/T012 | epwflow/src/delivery/scheduler.rs | epwflow/tests/delivery.rs |
| FR-006 | T013/T014 | epwflow/README.md | epwflow/tests/delivery.rs |
| FR-007 | T015/T016 | epwflow/src/delivery/acceptance.rs | epwflow/tests/delivery.rs |
| FR-008 | T017/T018 | epwflow/src/delivery/compare.rs | epwflow/tests/delivery.rs |
| FR-009 | T019/T020 | epwflow/src/delivery/budget.rs | epwflow/tests/delivery.rs |
| FR-010 | T021/T022 | scripts/build-native.sh、scripts/linux/package-native-runtime.sh、Cargo.toml | epwflow/tests/delivery.rs |
| FR-011 | T023/T024 | scripts/install-current.sh、scripts/install-compat.sh、scripts/verify-entries.sh | epwflow/tests/delivery.rs |
| FR-012 | T025/T026 | epwflow/src/delivery/regression.rs | epwflow/tests/delivery.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/delivery.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
