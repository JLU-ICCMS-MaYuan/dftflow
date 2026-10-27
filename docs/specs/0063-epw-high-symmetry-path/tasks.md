# 实施任务：高对称端点路径

输入：[Spec](spec.md)、[计划](plan.md)、[调查](research.md)。

- [x] T001 核对原生取点及旧行为，更新 `docs/specs/0063-epw-high-symmetry-path/research.md` 并澄清比较统计口径。
- [x] T002 [US1] 在 `epwflow/src/interpolation/electronic_sampling.rs` 校验真实端点、断段、零长度、非法坐标和标签。
- [x] T003 [US1] 在 `epwflow/src/interpolation/epw_config.rs` 接入短路径，增加 `electronic_sampling.rs` 的密度独立与 MgB₂ 九段回归。
- [x] T004 [US2] 在 `epwflow/src/interpolation/electronic_sampling.rs` 核验原生实际采样并按已澄清口径解耦比较；仅同坐标统计且不显示覆盖率。
- [x] T005 [US2] 在 `epwflow/src/foundation/workspace_science.rs`、`workspace_plot.rs` 及既有 comparison 模块保持能带、DOS 和后续总览发布。
- [x] T006 [US3] 在 `epwflow/tests/support/stage_results.rs` 和协议 fixture 验证真实生成入口、失败拒绝及历史保护，运行相关回归并写入 `acceptance.md`。
- [x] T007 同步 `docs/overview/epwflow/workflow.md` 及受影响示例，检查原话、链接和需求映射，按本任务范围提交。

顺序：T001～T003 后实施 T004～T005，再执行 T006～T007。比较口径已获用户明确答复。修改同一文件的任务串行，不启动多代理。US1 为路径声明的最小交付，全部验收还须 US2／US3。
