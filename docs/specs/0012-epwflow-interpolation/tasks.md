# 实施任务：Wannier 化与 EPW 插值

**输入**：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据模型](data-model.md)、[契约](contracts/interface.md)。

## 阶段 1：准备

- [ ] T001 核对本项原文及前置契约，记录到 docs/specs/0012-epwflow-interpolation/acceptance.md。

## 阶段 2：基础能力

- [ ] T002 建立或扩展 epwflow/src/interpolation/mod.rs 的模块边界与版本检查；依赖前置已验收接口。

## 阶段 3：US1 — 直接 CLI 与阶段边界

独立验收：执行 FR-001 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T003 [US1] 在 epwflow/tests/interpolation.rs 添加 FR-001 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T004 [US1] 在 epwflow/src/interpolation/cli.rs 实现 FR-001 全部行为，使用统一契约并执行前项测试。

## 阶段 4：US1 — 统一配置与明确的能量参考

独立验收：执行 FR-002 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T005 [US1] 在 epwflow/tests/interpolation.rs 添加 FR-002 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T006 [US1] 在 epwflow/src/interpolation/epw_config.rs 实现 FR-002 全部行为，使用统一契约并执行前项测试。

## 阶段 5：US1 — 绑定 F2 数据与程序版本

独立验收：执行 FR-003 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T007 [US1] 在 epwflow/tests/interpolation.rs 添加 FR-003 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T008 [US1] 在 epwflow/src/interpolation/adapter.rs 实现 FR-003 全部行为，使用统一契约并执行前项测试。

## 阶段 6：US2 — Wannier 数值成功

独立验收：执行 FR-004 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T009 [US2] 在 epwflow/tests/interpolation.rs 添加 FR-004 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T010 [US2] 在 epwflow/src/interpolation/wannier.rs 实现 FR-004 全部行为，使用统一契约并执行前项测试。

## 阶段 7：US2 — 可复用的粗网格数据

独立验收：执行 FR-005 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T011 [US2] 在 epwflow/tests/interpolation.rs 添加 FR-005 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T012 [US2] 在 epwflow/src/interpolation/coarse.rs 实现 FR-005 全部行为，使用统一契约并执行前项测试。

## 阶段 8：US2 — 独立插值质量检查

独立验收：执行 FR-006 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T013 [US2] 在 epwflow/tests/interpolation.rs 添加 FR-006 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T014 [US2] 在 epwflow/src/interpolation/quality.rs 实现 FR-006 全部行为，使用统一契约并执行前项测试。

## 阶段 9：US3 — 细网格与矩阵元交付

独立验收：执行 FR-007 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T015 [US3] 在 epwflow/tests/interpolation.rs 添加 FR-007 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T016 [US3] 在 epwflow/src/interpolation/fine.rs 实现 FR-007 全部行为，使用统一契约并执行前项测试。

## 阶段 10：US3 — α²F、λ 与低频处理

独立验收：执行 FR-008 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T017 [US3] 在 epwflow/tests/interpolation.rs 添加 FR-008 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T018 [US3] 在 epwflow/src/interpolation/spectral.rs 实现 FR-008 全部行为，使用统一契约并执行前项测试。

## 阶段 11：US4 — 质量阈值与数值一致性容差

独立验收：执行 FR-009 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T019 [US4] 在 epwflow/tests/interpolation.rs 添加 FR-009 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T020 [US4] 在 epwflow/src/interpolation/compare.rs 实现 FR-009 全部行为，使用统一契约并执行前项测试。

## 阶段 12：US4 — 分阶段复用、失效与恢复

独立验收：执行 FR-010 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T021 [US4] 在 epwflow/tests/interpolation.rs 添加 FR-010 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T022 [US4] 在 epwflow/src/interpolation/reuse.rs 实现 FR-010 全部行为，使用统一契约并执行前项测试。

## 阶段 13：US4 — 调度、预算与输出位置

独立验收：执行 FR-011 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T023 [US4] 在 epwflow/tests/interpolation.rs 添加 FR-011 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T024 [US4] 在 epwflow/src/interpolation/scheduler.rs 实现 FR-011 全部行为，使用统一契约并执行前项测试。

## 阶段 14：US5 — 交付清单与独立验收

独立验收：执行 FR-012 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T025 [US5] 在 epwflow/tests/interpolation.rs 添加 FR-012 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T026 [US5] 在 epwflow/src/interpolation/manifest.rs 实现 FR-012 全部行为，使用统一契约并执行前项测试。

## 阶段 15：US5 — 文档与兼容边界

独立验收：执行 FR-013 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T027 [US5] 在 epwflow/tests/interpolation.rs 添加 FR-013 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T028 [US5] 在 epwflow/README.md 实现 FR-013 全部行为，使用统一契约并执行前项测试。

## 最终阶段：成功标准与交付

- [ ] T029 [US5] 核验 SC-001 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T030 [US5] 核验 SC-002 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T031 [US5] 核验 SC-003 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T032 [US5] 核验 SC-004 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T033 [US5] 核验 SC-005 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T034 [US5] 核验 SC-006 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T035 [US5] 核验 SC-007 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T036 [US5] 核验 SC-008 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0012-epwflow-interpolation/acceptance.md；真实项目无证据不得勾选。
- [ ] T037 完成 epwflow/README.md、docs/overview/epwflow/ 的已实现行为回写与 quickstart 验证，并在本项 acceptance.md 收敛差距。

## 依赖与执行顺序

T001→T002 后按故事和前置任务顺序执行；同一共享文件串行，不设置无依赖依据的 [P] 标记。先测试、再实现、再真实验收。F1→F2→F3→F4→F5，前置未满足的真实接口不得放行。MVP 为本项 P1 故事的可验证闭环，不能以 MVP 代替所有任务。

## 需求覆盖

| 来源 | 任务 | 设计/文件 | 验证 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/interpolation/cli.rs | epwflow/tests/interpolation.rs |
| FR-002 | T005/T006 | epwflow/src/interpolation/epw_config.rs | epwflow/tests/interpolation.rs |
| FR-003 | T007/T008 | epwflow/src/interpolation/adapter.rs | epwflow/tests/interpolation.rs |
| FR-004 | T009/T010 | epwflow/src/interpolation/wannier.rs | epwflow/tests/interpolation.rs |
| FR-005 | T011/T012 | epwflow/src/interpolation/coarse.rs | epwflow/tests/interpolation.rs |
| FR-006 | T013/T014 | epwflow/src/interpolation/quality.rs | epwflow/tests/interpolation.rs |
| FR-007 | T015/T016 | epwflow/src/interpolation/fine.rs | epwflow/tests/interpolation.rs |
| FR-008 | T017/T018 | epwflow/src/interpolation/spectral.rs | epwflow/tests/interpolation.rs |
| FR-009 | T019/T020 | epwflow/src/interpolation/compare.rs | epwflow/tests/interpolation.rs |
| FR-010 | T021/T022 | epwflow/src/interpolation/reuse.rs | epwflow/tests/interpolation.rs |
| FR-011 | T023/T024 | epwflow/src/interpolation/scheduler.rs | epwflow/tests/interpolation.rs |
| FR-012 | T025/T026 | epwflow/src/interpolation/manifest.rs | epwflow/tests/interpolation.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/interpolation.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |

## 2026-10-16 实现收敛

- [ ] T038 依据 0015 B01～B09 修复本 Feature 的真实生产路径，取消已通过回归的 ignore；核对其余 FR/SC、真实程序输出和来源，更新验收与当前功能文档。原已勾选项不能替代本轮复核。实现与测试均由本助手承担。

## 2026-10-19：叠图数据补充任务（尚未实施）

- [ ] T039 [US2/US5] 在 epwflow/tests/interpolation.rs 增加成对路径、坐标／单位／E_F、共同有效窗、DOS 网格与来源指纹的失败测试；直接 ph.x 点与 QE matdyn 曲线分别检查。对应 FR-014、SC-009。
- [ ] T040 [US2/US5] 在 interpolation 的比较数据适配中复用现有 QE 输入／q2r/matdyn 能力，生成同点能带／声子曲线和冻结的 DOS 网格数据；输出新 comparison 契约，缺态／预算不足阻断。对应 FR-014、SC-009，依赖 T039。
- [ ] T041 [US3/US5] 增加复原子投影规范、非对角项、解纠缠／排除带、键中心误标、按模式声子 DOS 冒充元素、质量／权重／核尾部和积分的负向测试。对应 FR-015、SC-010。
- [ ] T042 [US3/US5] 在 interpolation 适配中实现可验证的原子投影算符变换、质量加权本征矢元素权重和两侧共同 DOS 累积；导出总量、元素和严格轨道数据，明确电子 spilling。对应 FR-015、SC-010，依赖 T040/T041。
- [ ] T043 [US5] 用真实 CLI 验证 comparison 交付、缺投影部分结果、跨目录／篡改拒绝、共享预算和质量失败仅诊断；用契约消费样本独立验证交付，把实际证据写入 acceptance.md；0014 后续联调、0015 后续验证真实数据，不作为本项独立验收前置。对应 FR-014/015、SC-009/010，依赖 T042。

覆盖与设计映射见 [计划补充](plan.md)；实现前先完成目标版本原生格式适配核查。本段所有任务保持未完成，原质量、网格外参考覆盖和超导依赖不变。

## 2026-10-19：比较诊断的已验证子集

- [x] T044 [US2/US5] 实现已有 MgB₂ 原生文件→比较表的只读适配、共同核总 DOS、严格清单与错误拒绝；通过 comparison_plots 的原生样本、非正交坐标、重复网格、哈希、单位与积分测试。对应 FR-014 的部分数据边界；不含新科学调度或 FR-015 严格原子投影。

T039～T043 仍按完整要求保持未完成；T044 不替代投影变换、真实来源与全部独立质量覆盖。


## 2026-10-19：本轮执行范围变更

原子投影算符与声子元素本征矢适配按用户最新指令暂缓，已有未完成项不勾选；恢复时仍遵守原可靠性契约。当前先完成总量叠图、声子 15/2 cm⁻¹ 质量复核、固定补充网格外点，再进入已指定细网格和原生求解。只改最大误差，RMS／虚频／覆盖／求解与资源标准不变；历史失败报告不重写。共用质量阈值与原生报告记录、边界／RMS 拒绝回归和真实续算证据见本次验收追加段。

## 2026-10-19：紫色标注与 50 cm⁻¹ 最大误差

- [x] T045 实现共用 50/2 质量规则和采样／路径最大误差标注数据；核对非正交坐标、排序模式、来源、数值／路径语义与旧包兼容。对应 FR-009/014、SC-009；验证见 [验收记录](acceptance.md) 的最新追加段。当前规则以最新 Spec 为准，前一轮 15/2 作为历史保留；RMS 超限，细网格／超导及整体未完成项不勾选。
