# 实施任务：QE 三模式数据准备

**输入**：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据模型](data-model.md)、[契约](contracts/interface.md)。

## 阶段 1：准备

- [ ] T001 核对本项原文及前置契约，记录到 docs/specs/0010-epwflow-qe-data/acceptance.md。

## 阶段 2：基础能力

- [ ] T002 建立或扩展 epwflow/src/qe_data/mod.rs 的模块边界与版本检查；依赖前置已验收接口。

## 阶段 3：US1 — 公开命令与阶段边界

独立验收：执行 FR-001 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T003 [US1] 在 epwflow/tests/qe_data.rs 添加 FR-001 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T004 [US1] 在 epwflow/src/qe_data/cli.rs 实现 FR-001 全部行为，使用统一契约并执行前项测试。

## 阶段 4：US1 — 科学配置与单位

独立验收：执行 FR-002 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T005 [US1] 在 epwflow/tests/qe_data.rs 添加 FR-002 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T006 [US1] 在 epwflow/src/qe_data/qe_config.rs 实现 FR-002 全部行为，使用统一契约并执行前项测试。

## 阶段 5：US1 — 一次 SCF 与完整快照

独立验收：执行 FR-003 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T007 [US1] 在 epwflow/tests/qe_data.rs 添加 FR-003 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T008 [US1] 在 epwflow/src/qe_data/scf.rs 实现 FR-003 全部行为，使用统一契约并执行前项测试。

## 阶段 6：US2 — 完整均匀 NSCF 电子态

独立验收：执行 FR-004 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T009 [US2] 在 epwflow/tests/qe_data.rs 添加 FR-004 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T010 [US2] 在 epwflow/src/qe_data/nscf.rs 实现 FR-004 全部行为，使用统一契约并执行前项测试。

## 阶段 7：US2 — 三种 PH 行为

独立验收：执行 FR-005 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T011 [US2] 在 epwflow/tests/qe_data.rs 添加 FR-005 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T012 [US2] 在 epwflow/src/qe_data/phonon.rs 实现 FR-005 全部行为，使用统一契约并执行前项测试。

## 阶段 8：US3 — q 点映射

独立验收：执行 FR-006 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T013 [US3] 在 epwflow/tests/qe_data.rs 添加 FR-006 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T014 [US3] 在 epwflow/src/qe_data/qmap.rs 实现 FR-006 全部行为，使用统一契约并执行前项测试。

## 阶段 9：US3 — 响应数据完整归集

独立验收：执行 FR-007 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T015 [US3] 在 epwflow/tests/qe_data.rs 添加 FR-007 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T016 [US3] 在 epwflow/src/qe_data/collect_ph.rs 实现 FR-007 全部行为，使用统一契约并执行前项测试。

## 阶段 10：US3 — 可定位的输入输出与交付清单

独立验收：执行 FR-008 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T017 [US3] 在 epwflow/tests/qe_data.rs 添加 FR-008 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T018 [US3] 在 epwflow/src/qe_data/manifest.rs 实现 FR-008 全部行为，使用统一契约并执行前项测试。

## 阶段 11：US4 — 导入、复用与旧目录

独立验收：执行 FR-009 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T019 [US4] 在 epwflow/tests/qe_data.rs 添加 FR-009 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T020 [US4] 在 epwflow/src/qe_data/reuse.rs 实现 FR-009 全部行为，使用统一契约并执行前项测试。

## 阶段 12：US4 — 依赖、预算与失败

独立验收：执行 FR-010 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T021 [US4] 在 epwflow/tests/qe_data.rs 添加 FR-010 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T022 [US4] 在 epwflow/src/qe_data/scheduler.rs 实现 FR-010 全部行为，使用统一契约并执行前项测试。

## 阶段 13：US5 — 预先固定的数据与数值对照标准

独立验收：执行 FR-011 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T023 [US5] 在 epwflow/tests/qe_data.rs 添加 FR-011 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T024 [US5] 在 epwflow/src/qe_data/compare.rs 实现 FR-011 全部行为，使用统一契约并执行前项测试。

## 阶段 14：US5 — 独立真实接口验收与说明

独立验收：执行 FR-012 的全部成功与拒绝场景；对应完整故事见 spec.md。

- [ ] T025 [US5] 在 epwflow/tests/qe_data.rs 添加 FR-012 的行为测试，包含独立进程或数据故障边界，先复现拒绝场景。
- [ ] T026 [US5] 在 epwflow/src/qe_data/qe_acceptance.rs 实现 FR-012 全部行为，使用统一契约并执行前项测试。

## 最终阶段：成功标准与交付

- [ ] T027 [US5] 核验 SC-001 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T028 [US5] 核验 SC-002 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T029 [US5] 核验 SC-003 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T030 [US5] 核验 SC-004 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T031 [US5] 核验 SC-005 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T032 [US5] 核验 SC-006 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T033 [US5] 核验 SC-007 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T034 [US5] 核验 SC-008 的全部标准，将命令、结果、缺项和证据写入 docs/specs/0010-epwflow-qe-data/acceptance.md；真实项目无证据不得勾选。
- [ ] T035 完成 epwflow/README.md、docs/overview/epwflow/ 的已实现行为回写与 quickstart 验证，并在本项 acceptance.md 收敛差距。

## 依赖与执行顺序

T001→T002 后按故事和前置任务顺序执行；同一共享文件串行，不设置无依赖依据的 [P] 标记。先测试、再实现、再真实验收。F1→F2→F3→F4→F5，前置未满足的真实接口不得放行。MVP 为本项 P1 故事的可验证闭环，不能以 MVP 代替所有任务。

## 需求覆盖

| 来源 | 任务 | 设计/文件 | 验证 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/qe_data/cli.rs | epwflow/tests/qe_data.rs |
| FR-002 | T005/T006 | epwflow/src/qe_data/qe_config.rs | epwflow/tests/qe_data.rs |
| FR-003 | T007/T008 | epwflow/src/qe_data/scf.rs | epwflow/tests/qe_data.rs |
| FR-004 | T009/T010 | epwflow/src/qe_data/nscf.rs | epwflow/tests/qe_data.rs |
| FR-005 | T011/T012 | epwflow/src/qe_data/phonon.rs | epwflow/tests/qe_data.rs |
| FR-006 | T013/T014 | epwflow/src/qe_data/qmap.rs | epwflow/tests/qe_data.rs |
| FR-007 | T015/T016 | epwflow/src/qe_data/collect_ph.rs | epwflow/tests/qe_data.rs |
| FR-008 | T017/T018 | epwflow/src/qe_data/manifest.rs | epwflow/tests/qe_data.rs |
| FR-009 | T019/T020 | epwflow/src/qe_data/reuse.rs | epwflow/tests/qe_data.rs |
| FR-010 | T021/T022 | epwflow/src/qe_data/scheduler.rs | epwflow/tests/qe_data.rs |
| FR-011 | T023/T024 | epwflow/src/qe_data/compare.rs | epwflow/tests/qe_data.rs |
| FR-012 | T025/T026 | epwflow/src/qe_data/qe_acceptance.rs | epwflow/tests/qe_data.rs |
| SC-001 | T027 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-002 | T028 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-003 | T029 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-004 | T030 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-005 | T031 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-006 | T032 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-007 | T033 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-008 | T034 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |

## 2026-10-16 实现收敛

- [ ] T036 依据 0015 B01～B09 修复本 Feature 的真实生产路径，取消已通过回归的 ignore；核对其余 FR/SC、真实程序输出和来源，更新验收与当前功能文档。原已勾选项不能替代本轮复核。实现与测试均由本助手承担。

- [ ] T037 按复用方案移除重复 QE 结构/输入实现，提取 0006 当前声子链公共能力；验证两入口、三模式、快照隔离及 EPW 专用数据校验。关联 0010 FR-002～007、SC-001～004 与 0015 阻断回归；部分输入层完成不等于整条链路完成。

- [x] T038 完成共享结构/PW/PH/均匀网格及日志检查的第一批复用；7 项原阻断回归转通过，qeflow 27 项回归与共享/epwflow 79 项通过。证据见 0015 acceptance.md 的 2026-10-16 追加记录；不包含尚未接通的快照/调度/EPW 科学链路。

## 保存树与 PH 主路径收敛

- [x] T039 [US1/US2] 修复实际 prefix 保存树、完整 SCF 快照及 NSCF／PH 独立复制，核验真实 XML、均匀网格、波函数和执行日志。对应 FR-003/004/008、SC-001/003。
- [x] T040 [US2] 从实际 dyn0 和 SCF 倒格矢恢复 q 映射；按 Γ／非 Γ 与 coords 局部 q 读取原生 dyn XML、patterns、control、dvscf，核对身份、尺寸和 EPW save 交接。对应 FR-005～008、SC-002/004。
- [x] T041 [US1/US2] 使用真实格式独立进程覆盖三模式、非默认 prefix、不同可写副本及失败拒绝；只读解析已有 LDA QE 输出，回归统一入口和 qeflow。分开记录代码行为和未执行的真实科学验证。

以上三项证据见 [原生 QE 交接验收](../0015-epwflow-mgb2-real-validation/native-qe-handoff-validation.md)，不代替原有真实科学 SC-007/008。

- [ ] T042 [US3/US4] 收敛 F1 对导入 F2 阶段的自动复用、F2 阶段内部分失败续算及 outdir 基址布局；保留旧尝试且不得重复已验证 SCF。覆盖资源与未知现场边界，对应 FR-003/008～010、SC-003/005/006。

## 三程序收敛

- [x] T100 验证 qeflow nosplit 完整本地链，增加可选 `retain_epw_data` 和原生 PH 归档，记录拒绝与兼容证据。
- [x] T101 将 epw_phonon 接入同一 qeflow 库，准备完整 NSCF 和严格 EPW 交付清单；覆盖后处理失败恢复、固定质量／网格位移，在指定 local_run 验证配置与脚本生成。真实科学全链另行验收，不由替身或生成结果替代。
