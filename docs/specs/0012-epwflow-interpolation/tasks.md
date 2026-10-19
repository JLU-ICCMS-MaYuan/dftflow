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

- [x] T046 [US2/US5] 解析带哈希 Wannier 实际外窗并换算共同 E_F，重新导出完整显示窗 DOS；验证缺省、Fortran 指数、错误窗口、偏移关系和真实数据哈希。对应 FR-014、SC-009。

本轮 T046 已通过窗口回归、真实重绘和迁移核验；完整证据见 [0015 外窗与目录验收](../0015-epwflow-mgb2-real-validation/window-directory-validation.md)。其余整体科学验收状态不变。

- [x] T047 复用实际 Wannier 窗口来源，补充冻结窗元数据及边界校验；验证实际 Outer／Inner、Fortran 指数、缺失／错误记录、冻结窗包含关系与旧包兼容，向 0014 交付相对共同 E_F 的四边界。对应 FR-014；实测与科学源哈希证据见 [验收记录](acceptance.md) 的窗口边界追加段。

## 2026-10-19：RMS 20 门与原生代码收尾子任务

- [x] T048 将共用声子 RMS 门改为 20 cm⁻¹，保持最大值 50、电子／虚频／覆盖规则，验证 20 边界与超限拒绝；用已有五点执行只读 CLI，保存新报告和来源哈希，不覆盖历史失败。对应 FR-009/012；证据见 [代码收尾验收](../0015-epwflow-mgb2-real-validation/native-closeout-validation.md)。

## 原生粗／细主路径收敛

- [x] T049 [US1/US2/US3] 修复 F2 版本 2 数据复制、内置 Wannier、原生粗网格重启、先参考后输出的质量配对、分展宽目录及 F1/F4 同源复核。对应 FR-003～009/012、D11；验证原生格式进程链和错误阻断。
- [x] T050 [US4/US5] 显式核验现有 LDA 的原生 Wannier 与粗网格重启布局，保存只读来源哈希和限制；完整执行 epwflow/qeflow 回归，回写实际证据。对应 SC-001/002/003 的子集，不替代真实 EPC／Tc。
- [ ] T051 [US4/US5] 收敛粗／细阶段内恢复、历史粗网格有界导入和既有 LDA 规范的重读质量检查；解决实际选窗兼容决策后，完成真实细网格、独立参考与三来源六组合。对应 FR-010、SC-003/004；不擅自实施 0016 候选扫描。

T049/T050 验证范围、133 项回归及真实文件只读证据见 [原生执行验收](../0015-epwflow-mgb2-real-validation/native-interpolation-validation.md)；不勾选 T051 或完整 SC 科学验收。


## 真实粗网格重读与细网格归集收敛

- [x] T052 [US2/US4] 用原生重启角色重算固定六 k／五 q，补齐 EPW loadbm 必需的同源 .ukk 及完整记录校验；保留原 Wannier／来源哈希，不重新选窗。对应 FR-005/010 的执行子集；真实质量重读通过。
- [x] T053 [US3/US5] 显式细网格温度，修复所选 q 与完整网格计数区分；生产函数完成真实 40³/20³ EPC，验证 24 个分片、500 点谱及两种 λ，保留失败日志后只读恢复归集。对应 FR-007/008/012 的执行子集；证据见 [细网格验收](../0015-epwflow-mgb2-real-validation/native-fine-validation.md)。

T052/T053 不替代 T051 的历史来源自动导入／阶段恢复、独立数值对照及六组合验收，也不表示已经得到 Tc。

## 三程序阶段边界更新

按已确认的[三程序契约](../0008-epwflow-foundation/three-programs.md)，Wannier／粗网格与质量独立发布 wannier/manifest.json；细网格 EPC／α²F 在第三个程序执行，并另行发布 interpolation/manifest.json。阶段分离和来源不变进程回归已通过，见[0008 验收](../0008-epwflow-foundation/acceptance.md)；自动参考已通过进程回归；四类比较图的自动来源及真实完整科学验收仍未完成。

- [x] T054 收敛原生粗／细阶段显式恢复：持久请求、物化输入与成功指纹、失败归档、发布中断重验归集及逐展宽复用；覆盖未知现场、来源／输出篡改、质量失败不重算和不重复 SCF／PH。对应 FR-010，T051 的代码恢复子集；不替代真实科学与六组合验收。


- [x] T055 [US2] 缺省独立质量参考自动生成：复用已绑定 SCF 副本执行固定 k 的 QE 电子态、固定 q 的直接 PH；用原 Γ onsite ASR 与 QE dynmat 对角化，保存全部原始／派生来源、失败归档和恢复证据。覆盖坐标／模式／质量／虚频、程序和来源变化、零新增 SCF 及质量失败阻断。对应 FR-006/009/010、SC-002/005，0008 FR-016 的参考子集。
- [ ] T056 [US2/US5] 自动生成比较路径／DOS 网格的 QE 与 EPW 数据，接入已有比较导出和 0014 四类图／总览；保持独立质量点和粗细边界，覆盖失败诊断及移动重绘。对应 FR-014/015、SC-009/010，投影暂缓条款继续适用。

T055 与显式纯能量的完整串行回归为 145 通过、0 失败、6 项真实环境暂挂；详见[专项验收](../0015-epwflow-mgb2-real-validation/native-reference-validation.md)。该勾选只表示参考生成与恢复代码验收，不关闭独立覆盖或真实科学验证。
