# 技术任务：README 命令行展示与取消表格

需求入口见 [GitHub Issue #71](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/71)，实现方式见 [计划](plan.md)。本文件只记录技术拆解，不替代线上状态。

- [x] T001 追加完整补充原话及实施授权，将命令行形式写入 FR-007／SC-005，核对 US1／US2、范围与原有要求一致。证据：Issue／Spec 的两条完整原话及新增需求。
- [x] T002 按 US1／US2 将目标 README 的八阶段表格改为独立小节，保留命令、作用、产物、参数与其他操作说明；完整 shell 命令只放独立代码块。证据：[目标 README](../../../examples/MgB2/sugon-amd/run/epwflow/README.md) 八个步骤与 11 个代码块。
- [x] T003 按 SC-001～005 核对排版、命令、内容和链接，执行代码块语法检查并写入 [验收记录](acceptance.md)；同步需求检查表和文档影响，本任务按明确路径单独提交。
