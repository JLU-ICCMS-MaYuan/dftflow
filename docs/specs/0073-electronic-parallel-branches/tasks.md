# 实施任务

## 阶段一：基线

- [x] T001 核对 #73 原文、现有草稿与调度边界，补齐 docs/specs/0073-electronic-parallel-branches/ 设计产物。

## 阶段二：独立提交与恢复

- [x] T002 [US1] 在 epwflow/tests/support/electronic_parallel_0073.rs 添加分支提交、依赖、活动和未知状态的进程回归。
- [x] T003 [US1] [US2] 在 epwflow/src/foundation/electronic_parallel.rs 实现独立分支提交、执行、回执和选择性恢复，隔离成功数据。

## 阶段三：科学与绘图边界

- [x] T004 [US3] [US4] 在 epwflow/src/foundation/workspace_electronic.rs 与 workspace_science.rs 分开投影执行和组合绘图，保留即时范围诊断并验证失败隔离。

## 阶段四：交付

- [x] T005 在 docs/specs/0073-electronic-parallel-branches/acceptance.md 记录回归、真实 MgB₂ 运行及恢复证据。
- [x] T006 更新 docs/overview/epwflow/README.md、epwflow/README.md 和根 README.md 的现有操作说明，构建覆盖固定入口并提交本任务文件。
- [ ] T007 完成 GitHub #73 Documentation Impact、父子关系与双向远程链接核验后关闭。

依赖：T001 → T002 → T003 → T004 → T005 → T006 → T007；不安排其他功能实施。
