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

## 2026-10-16 实现收敛

- [ ] T038 依据 0015 B01～B09 修复本 Feature 的真实生产路径，取消已通过回归的 ignore；核对其余 FR/SC、真实程序输出和来源，更新验收与当前功能文档。原已勾选项不能替代本轮复核。实现与测试均由本助手承担。

## 2026-10-19：四类叠图与独立重绘（尚未实施）

- [ ] T039 [US2/US3] 在 epwflow/tests/delivery.rs 增加成对数据读取、总量／元素／轨道标签、单位、缺投影、图层／线型／断点和诊断状态测试；错误数据先复现拒绝。对应 FR-014/015、SC-009/010。
- [ ] T040 [US1/US3] 扩展 delivery/collect 与结果检查，消费 0012 comparison 契约并导出自包含比较包；允许无 F4 清单的诊断结果，保留来源、差异、哈希和部分状态。对应 FR-015、SC-010，依赖 0012 T043 和本项 T039。
- [ ] T041 [US2] 复用绘图基础实现四张独立叠图及 2×2 总览，QE 实线先绘、EPW 虚线最后绘，保留元素／轨道与独立 ph.x 点，生成三格式与图表清单。对应 FR-014、SC-009，依赖 T040。
- [ ] T042 [US2/US3] 验证完全重合／偏离／多元素／缺项样本，实际打开 PNG／SVG／PDF 检查可读性；将结果包移到其他目录重绘，核对科学源和表格哈希不变、外部程序／sbatch 调用为零及未知输出冲突拒绝。对应 FR-014/015、SC-009/010，依赖 T041。
- [ ] T043 [US1/US5] 连通正常工作流自动比较交付、仅插值诊断和图表失败恢复；质量失败不得放行细网格 EPC／超导。与 0015 T020～T023 逐来源联调并更新验收，图表通过不得替代六组合科学验收。对应 FR-014/015、SC-011，依赖 T042。

设计与 FR/SC 映射见 [计划](plan.md)。本段任务全部待实现／验证，不能以本次文档提交勾选科学交付。

## 2026-10-19：独立比较绘图的已验证子集

- [x] T044 [US2] 接通 collect／plot／results check 比较包子集；生成四类独立图和 2×2 总览、三格式，QE 先实线／EPW 后虚线；验证路径裁剪、缺项分面、无 F4 时导出、移动重绘、篡改和未知覆盖拒绝。证据见 comparison_plots.rs、绘图模块单测和 acceptance.md。

该任务不包含严格投影数据生成、三来源全部覆盖、完整结果包和自动入口；T039～T043 及 SC-009～011 的完整验收保持未完成。


## 2026-10-19：本轮执行范围变更

元素／轨道分面真实验收按用户最新指令暂缓，已有未完成项不勾选；恢复时仍遵守原可靠性契约。当前先完成总量叠图、声子 15/2 cm⁻¹ 质量复核、固定补充网格外点，再进入已指定细网格和原生求解。只改最大误差，RMS／虚频／覆盖／求解与资源标准不变；历史失败报告不重写。共用质量阈值与原生报告记录、边界／RMS 拒绝回归和真实续算证据见本次验收追加段。

## 2026-10-19：紫色标注与 50 cm⁻¹ 最大误差

- [x] T045 在声子独立图与总览中绘制紫色路径标注及路径外最大误差诊断区，列出两侧频率与 q／模式，保持科学数据和失败状态。对应 FR-014/015、SC-009/010；验证见 [验收记录](acceptance.md) 的最新追加段。当前规则以最新 Spec 为准，前一轮 15/2 作为历史保留；RMS 超限，细网格／超导及整体未完成项不勾选。

- [x] T046 [US2/US3] 按实际 dis_win 外窗重绘独立电子能带、DOS 和总览，核验坐标与标题、原数据不变、三格式及旧包兼容；完成编号目的目录迁移与入口引用更新。对应 FR-014/015、SC-009/010。

本轮 T046 已通过窗口回归、真实重绘和迁移核验；完整证据见 [0015 外窗与目录验收](../0015-epwflow-mgb2-real-validation/window-directory-validation.md)。其余整体科学验收状态不变。

- [x] T047 [US2/US3] 将独立能带、声子图及总览的高对称点符号移到各自横轴下方，移除路径数值及轴标题，保留 DOS 物理刻度；验证实际 SVG 坐标、三格式、真实重绘和科学数据不变。对应 FR-014、SC-009/010。
