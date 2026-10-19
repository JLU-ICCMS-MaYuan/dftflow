# 实施任务：两种超导求解与受限 Tc 搜索

**输入**：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据模型](data-model.md)、[契约](contracts/interface.md)。

## 阶段 1：准备

- [ ] T001 核对本项原文及前置契约，记录到 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md。

## 阶段 2：基础能力

- [ ] T002 建立或扩展 epwflow/src/superconductivity/mod.rs 的模块边界与版本检查；依赖前置已验收接口。

## 阶段 3：US1 — 公开命令与阶段边界

独立验收：执行 FR-001 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T003 [US1] 在 epwflow/tests/superconductivity.rs 添加 FR-001 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T004 [US1] 在 epwflow/src/superconductivity/cli.rs 实现 FR-001 全部行为，使用统一契约并执行前项测试。

## 阶段 4：US1 — 固定求解配置

独立验收：执行 FR-002 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T005 [US1] 在 epwflow/tests/superconductivity.rs 添加 FR-002 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T006 [US1] 在 epwflow/src/superconductivity/solver_config.rs 实现 FR-002 全部行为，使用统一契约并执行前项测试。

## 阶段 5：US1 — 一次尝试只处理一个模式和一个温度

独立验收：执行 FR-003 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T007 [US1] 在 epwflow/tests/superconductivity.rs 添加 FR-003 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T008 [US1] 在 epwflow/src/superconductivity/attempt.rs 实现 FR-003 全部行为，使用统一契约并执行前项测试。

## 阶段 6：US2 — 逐温度数值成功

独立验收：执行 FR-004 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T009 [US2] 在 epwflow/tests/superconductivity.rs 添加 FR-004 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T010 [US2] 在 epwflow/src/superconductivity/parse.rs 实现 FR-004 全部行为，使用统一契约并执行前项测试。

## 阶段 7：US2 — 能隙与重整化函数的含义

独立验收：执行 FR-005 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T011 [US2] 在 epwflow/tests/superconductivity.rs 添加 FR-005 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T012 [US2] 在 epwflow/src/superconductivity/gap.rs 实现 FR-005 全部行为，使用统一契约并执行前项测试。

## 阶段 8：US2 — 各向同性 Tc 判据

独立验收：执行 FR-006 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T013 [US2] 在 epwflow/tests/superconductivity.rs 添加 FR-006 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T014 [US2] 在 epwflow/src/superconductivity/iso.rs 实现 FR-006 全部行为，使用统一契约并执行前项测试。

## 阶段 9：US3 — 各向异性分类与双初值复核

独立验收：执行 FR-007 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T015 [US3] 在 epwflow/tests/superconductivity.rs 添加 FR-007 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T016 [US3] 在 epwflow/src/superconductivity/aniso.rs 实现 FR-007 全部行为，使用统一契约并执行前项测试。

## 阶段 10：US3 — 有界温度搜索

独立验收：执行 FR-008 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T017 [US3] 在 epwflow/tests/superconductivity.rs 添加 FR-008 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T018 [US3] 在 epwflow/src/superconductivity/search.rs 实现 FR-008 全部行为，使用统一契约并执行前项测试。

## 阶段 11：US4 — 次数、调度与资源预算

独立验收：执行 FR-009 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T019 [US4] 在 epwflow/tests/superconductivity.rs 添加 FR-009 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T020 [US4] 在 epwflow/src/superconductivity/scheduler.rs 实现 FR-009 全部行为，使用统一契约并执行前项测试。

## 阶段 12：US4 — 恢复、来源与结果身份

独立验收：执行 FR-010 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T021 [US4] 在 epwflow/tests/superconductivity.rs 添加 FR-010 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T022 [US4] 在 epwflow/src/superconductivity/reuse.rs 实现 FR-010 全部行为，使用统一契约并执行前项测试。

## 阶段 13：US4 — 状态与输出交付

独立验收：执行 FR-011 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T023 [US4] 在 epwflow/tests/superconductivity.rs 添加 FR-011 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T024 [US4] 在 epwflow/src/superconductivity/manifest.rs 实现 FR-011 全部行为，使用统一契约并执行前项测试。

## 阶段 14：US5 — 预先固定的数值与判定标准

独立验收：执行 FR-012 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T025 [US5] 在 epwflow/tests/superconductivity.rs 添加 FR-012 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T026 [US5] 在 epwflow/src/superconductivity/compare.rs 实现 FR-012 全部行为，使用统一契约并执行前项测试。

## 阶段 15：US5 — 验证与文档

独立验收：执行 FR-013 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T027 [US5] 在 epwflow/tests/superconductivity.rs 添加 FR-013 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T028 [US5] 在 epwflow/README.md 实现 FR-013 全部行为，使用统一契约并执行前项测试。

## 最终阶段：成功标准与交付

- [ ] T029 [US5] 核验 SC-001 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T030 [US5] 核验 SC-002 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T031 [US5] 核验 SC-003 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T032 [US5] 核验 SC-004 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T033 [US5] 核验 SC-005 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T034 [US5] 核验 SC-006 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T035 [US5] 核验 SC-007 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T036 [US5] 核验 SC-008 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0013-epwflow-superconductivity-tc/acceptance.md；真实项目无证据不得勾选。
- [ ] T037 完成 epwflow/README.md、docs/overview/epwflow/ 的已实现行为回写与 quickstart 验证，并在本项 acceptance.md 收敛差距。

## 依赖与执行顺序

T001→T002 后按故事和前置任务顺序执行；同一共享文件串行，不设置无依赖依据的 [P] 标记。先测试、再实现、再真实验收。F1→F2→F3→F4→F5，前置未满足的真实接口不得放行。MVP 为本项 P1 故事的可验证闭环，不能以 MVP 代替所有任务。

## 需求覆盖

| 来源 | 任务 | 设计/文件 | 验证 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/superconductivity/cli.rs | epwflow/tests/superconductivity.rs |
| FR-002 | T005/T006 | epwflow/src/superconductivity/solver_config.rs | epwflow/tests/superconductivity.rs |
| FR-003 | T007/T008 | epwflow/src/superconductivity/attempt.rs | epwflow/tests/superconductivity.rs |
| FR-004 | T009/T010 | epwflow/src/superconductivity/parse.rs | epwflow/tests/superconductivity.rs |
| FR-005 | T011/T012 | epwflow/src/superconductivity/gap.rs | epwflow/tests/superconductivity.rs |
| FR-006 | T013/T014 | epwflow/src/superconductivity/iso.rs | epwflow/tests/superconductivity.rs |
| FR-007 | T015/T016 | epwflow/src/superconductivity/aniso.rs | epwflow/tests/superconductivity.rs |
| FR-008 | T017/T018 | epwflow/src/superconductivity/search.rs | epwflow/tests/superconductivity.rs |
| FR-009 | T019/T020 | epwflow/src/superconductivity/scheduler.rs | epwflow/tests/superconductivity.rs |
| FR-010 | T021/T022 | epwflow/src/superconductivity/reuse.rs | epwflow/tests/superconductivity.rs |
| FR-011 | T023/T024 | epwflow/src/superconductivity/manifest.rs | epwflow/tests/superconductivity.rs |
| FR-012 | T025/T026 | epwflow/src/superconductivity/compare.rs | epwflow/tests/superconductivity.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/superconductivity.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |

## 2026-10-16 实现收敛

- [ ] T038 依据 0015 B01～B09 修复本 Feature 的真实生产路径，取消已通过回归的 ignore；核对其余 FR/SC、真实程序输出和来源，更新验收与当前功能文档。原已勾选项不能替代本轮复核。实现与测试均由本助手承担。

## 2026-10-19：原生 Tc 收敛任务

以下细化已有 FR／SC，不改变已确认设计；对应[完成度核查](../0007-epwflow-superconductivity/completion-audit.md)。当前 F3 声子 RMS 仍失败，代码与隔离测试可推进，真实求解必须等待质量及模式专用数据就绪；本轮没有勾选实现完成项。

- [ ] T039 [US1/US2] 按现有 Plan／契约实现目标 EPW 5.9 单模式、单温度、单初值输入与执行适配，记录程序／F3／温度／求解类型身份及原生 stdout／stderr／退出状态；以独立进程回归替换旧经验路径的验收用途。对应 FR-002～006、SC-001/002，细化 T006/T008/T010/T014。
- [ ] T040 [US2/US3] 在 parse.rs／gap.rs 读取目标版本原生完整 Δ／Z 与 Matsubara／k／band 映射，核对残差、迭代、缺失与非有限值；未收敛小能隙、平均抵消、零解和残缺输出不得得到有效分类。对应 FR-004/005/007、SC-003/005，细化 T010/T012/T016。
- [ ] T041 [US2/US3] 实现 iso 原生线性本征值分类与必需非线性能隙复核、aniso 两固定正初值的独立求解与逐元素一致性；移除 iso.rs／aniso.rs／attempt.rs 中用 Allen–Dynes／BCS 代替正式求解的路径，旧辅助测试不再标成原生能力证据。对应 FR-006/007/012、SC-002/003/006，依赖 T039/T040。
- [ ] T042 [US3/US4] 在 search.rs／attempt.rs／scheduler.rs 按 centikelvin 实现两模式独立区间、中点与四分点规则、分类矛盾检测、次数预留及持久恢复；全部外部尝试和失败计数，未知任务不重提。对应 FR-008～010、SC-002/004/007，细化 T018/T020/T022，依赖 T041。
- [ ] T043 [US4/US5] 按 data-model.md／contracts 实现两模式分开的 Tc 区间、完整能隙／Z、温度分类、初值和来源交付；协调 F5 消费，使用有界版本兼容，不能继续以一个 tc 标量代表 both。对应 FR-005/010/011、SC-005/008，细化 T024，依赖 T040/T042。
- [ ] T044 [US5] 在 F3 通过及预算允许后，先验证目标原生单温度两侧／正常侧收敛与输出，再执行固定温度范围的两模式搜索、至少三个预先固定有效能隙温度及独立临界附近参考；记录有效区间≤0.5 K、完整 Δ／Z 与本征值对照，不能用温度刻度冒充精度。对应 FR-012/013、SC-006/007，细化 T026/T034/T035，依赖 T043。
- [ ] T045 [US5] 用公开 CLI 验证仅变温度复用 F3、失败／中断／部分成功和跨目录恢复，将两模式实际结果交付 0014、三来源证据交付 0015，并更新验收与 Overview；没有真实证据的任务保持未完成。对应 FR-010/013、SC-004/008，细化 T028/T036/T037，依赖 T044。

## 2026-10-19：原生代码首批实施状态

用户决定暂不加密网格，先补原生执行代码。本批覆盖 T039～T043 的代码基础：原生请求／进程、退出与收敛检查、全 Δ／Z、egnv 映射及带偏移、双初值、整温度区间、计数持久化及版本 2 本地交付；旧 Allen–Dynes／BCS 正式路径及其正向数学测试已移除。

对应回归在 superconductivity.rs 使用独立进程和目标版本格式，明确属于契约 fixture。T039～T043 暂不整项勾选：实际 EPW 探测、统一入口最终阶段验证、显式失败重试和 F5 自包含消费尚缺；T044／T045 完全没有新的科学验收证据。依赖和原验收标准不变。

## 2026-10-19：RMS 20 门与原生代码收尾子任务

- [x] T046 [US3/US4] 实现显式失败重试和中断接续：保留旧目录、次数、阈值与身份，未知原分配不重提，终态证据后才创建新尝试；验证跨进程恢复、活动拒绝和上限拒绝。对应 FR-008～010，细化 T042。
- [x] T047 [US4] 最终重验原生日志、全 Δ／Z、分类／区间、固定配置、所选模式和必需能隙；拒绝篡改区间和部分求解冒充完成。对应 FR-010/011，细化 T043。

证据见 [代码收尾验收](../0015-epwflow-mgb2-real-validation/native-closeout-validation.md)。以上仅勾选可隔离核验的代码子任务；T039～T045 整项仍等待真实 EPW、完整生产链和科学参考证据。
