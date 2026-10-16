# 测试与 debug 任务

输入：[Spec](spec.md)、[Plan](plan.md)、[研究](research.md)、[数据](data-model.md)、[契约](contracts/verification.md)。

## 准备与阻断核查

- [x] T001 [US1] 同步远端，运行现有 41 项测试及公开 CLI，记录 docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md。
- [x] T002 [US2] 在 docs/local/0015-epwflow-smoke/ 盘点三套来源与哈希、只读查询队列；完成初查，不宣称完整 EPW 可读性或预算已核准。

## US1：真实接口失败回归（P1）

- [ ] T003 [US1] 在 epwflow/tests/real_contracts.rs 复现 B01～B08，覆盖公开 CLI、原生 stdout、空清单、实际 XML、未知 Slurm 状态和物理结果来源；对应修复触点见 acceptance.md，职责确认后执行，不用源码字符串断言替代行为测试。

## US2：三模式续算（P1）

- [ ] T004 [US2] 核对并修复 epwflow/src/qe_data/reuse.rs、collect_ph.rs 的树复制、来源和 q/扰动对应，验证 tmp、真实 XML 与原始哈希。
- [ ] T005 [US2] 核对并修复 foundation/{cli,process,slurm,budget}.rs 与各 scheduler 的真实入口/日志/依赖；在 docs/local/0015-epwflow-smoke/ 固定输入、独立参考与预算清单，预算未知时不提交。
- [ ] T006 [US2] 在 examples/MgB2/sugon-amd/epw/0015/ 隔离目录依次验证一套及三模式真实 NSCF/Wannier/质量比较/细网格/α²F，记录源码制品、作业、阶段次数及数值报告；不纳入 Git。

## US3：科学正确性及完整起点（P2）

- [ ] T007 [US3] 按 0010/0012/0013 完整固定标准核对独立参考、原生两模式超导与从 SCF 开始的真实 CLI/恢复；缺失实现按分工交接，证据写 acceptance.md，不用经验公式补结果。

## 收尾

- [ ] T008 更新 docs/overview/epwflow/README.md、epwflow/README.md、上游验收与本项 acceptance.md，逐项标记通过/失败/未完成；验证后提交，不自动安装或推送。

## 顺序与覆盖

T001→T002→草案/职责确认→T003→T004/T005→T006→T007→T008。共享文件串行。MVP 是一个来源的真实 α²F/λ，不代替三模式和完整科学验收。

| 需求/标准 | 任务 |
| --- | --- |
| FR-001 / SC-001 | T001、T003 |
| FR-002、FR-003 / SC-002 | T002、T004 |
| FR-004、FR-007 / SC-002 | T005、T006 |
| FR-005、FR-006 / SC-003 | T003、T006、T007 |
| FR-008 / SC-004 | T003、T007、T008 |
