# 实施任务

输入：[spec.md](spec.md)、[plan.md](plan.md)、[research.md](research.md)、[data-model.md](data-model.md)。

- [x] T001 核验两作业及 QE 本地源码，保存 docs/local/0021-dos-pencil-resume 原始日志和源数据哈希。
- [x] T002 [US1/US2] epwflow/tests/three_stage_execution.rs 新增 32 MPI DOS 失败后公开 -s 恢复、旧脚本拒绝及参数隔离测试；先复现失败。
- [x] T003 [US1/US2] workspace_science.rs 修复 DOS 参数；workspace.rs 改善旧 job 冲突提示，保留所有保护。
- [x] T004 串行运行 EPW／QE 相关回归，覆盖旧保护与 shell 失败传播。
- [x] T005 更新 Overview、模块与算例 README，规范构建安装三个 EPW 程序、核对真实文件哈希，完成 acceptance.md 并按明确路径提交。

顺序：T001→T002→T003→T004→T005。
FR／SC 对应设计及任务详见 plan.md；全部用户故事为本次 P1 修复，其他科学问题不扩大范围。
