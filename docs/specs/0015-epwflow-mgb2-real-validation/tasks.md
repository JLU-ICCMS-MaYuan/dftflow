# 测试与 debug 任务

输入：[Spec](spec.md)、[Plan](plan.md)、[研究](research.md)、[数据](data-model.md)、[契约](contracts/verification.md)。

## 准备与阻断核查

- [x] T001 [US1] 同步远端，运行现有 41 项测试及公开 CLI，记录 docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md。
- [x] T002 [US2] 在 docs/local/0015-epwflow-smoke/ 盘点三套来源与哈希、只读查询队列；完成初查，不宣称完整 EPW 可读性或预算已核准。

## US1：真实接口失败回归（P1）

- [ ] T003 [US1] 在 epwflow/tests/real_contracts.rs 复现 B01～B09，覆盖公开 CLI、原生 stdout、空清单、实际 XML、未知 Slurm 状态和物理结果来源；对应修复触点见 acceptance.md，由本助手负责失败复现和复测，生产补齐由实现模型负责，不用源码字符串断言替代行为测试。

## US2：三模式续算（P1）

- [ ] T004 [US2] 由实现模型修复 epwflow/src/qe_data/reuse.rs、collect_ph.rs，本助手核对树复制、来源和 q/扰动对应，验证 tmp、真实 XML 与原始哈希。
- [ ] T005 [US2] 由实现模型修复 foundation/{cli,process,slurm,budget}.rs 与各 scheduler 的真实入口/日志/依赖，本助手复测；在 docs/local/0015-epwflow-smoke/ 固定输入、独立参考与预算清单，预算未知时不提交。
- [ ] T006 [US2] 在 examples/MgB2/sugon-amd/epw/0015/ 隔离目录依次验证一套及三模式真实 NSCF/Wannier/质量比较/细网格/α²F，记录源码制品、作业、阶段次数及数值报告；不纳入 Git。

## US3：科学正确性及完整起点（P2）

- [ ] T007 [US3] 按 0010/0012/0013 完整固定标准核对独立参考、原生两模式超导与从 SCF 开始的真实 CLI/恢复；缺失实现按分工交接，证据写 acceptance.md，不用经验公式补结果。

## 收尾

- [ ] T008 更新 docs/overview/epwflow/README.md、epwflow/README.md、上游验收与本项 acceptance.md，逐项标记通过/失败/未完成；验证后提交，不自动安装或推送。

## 顺序与覆盖

T001→T002→T009 交接→实现模型分批修复与本助手 T003 复测→T004/T005→T006→T007→T008。完整草案确认和真实提交前置条件仍独立核对。共享文件串行。MVP 是一个来源的真实 α²F/λ，不代替三模式和完整科学验收。

| 需求/标准 | 任务 |
| --- | --- |
| FR-001 / SC-001 | T001、T003 |
| FR-002、FR-003 / SC-002 | T002、T004 |
| FR-004、FR-007 / SC-002 | T005、T006 |
| FR-005、FR-006 / SC-003 | T003、T006、T007 |
| FR-008 / SC-004 | T003、T007、T008 |

## 分工交接

- [x] T009 记录用户已确认的职责，建立 [实现交接清单](handoff.md)，关联 B01～B09、修复顺序、复现与逐批复测标准。仅完成文档交接，T003～T008 保持未完成。

### T003 首批进度（2026-10-16）

已新增 `epwflow/tests/real_contracts.rs`：1 项 stdout 对照通过，8 项已知失败回归显式执行并复现 B01/B04/B05/B06/B07/B08/B09。已知失败用例标注 ignore 与缺陷号，默认测试不会把其计为通过；修复后须逐项显式复测、取消 ignore。B02/B03、树复制与保存指纹、缺质量参考、完整状态恢复等回归仍待补齐，T003 保持未完成。仅新增测试与记录，没有补齐生产功能。
