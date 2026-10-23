# 0053：实施任务

输入：[规格](spec.md)、[计划](plan.md)、[研究](research.md)、[数据契约](data-model.md)、[接口](contracts/logs.md)。

- [x] T001 核对现用与历史脚本绑定并保存前置哈希，记录于 `docs/specs/0053-epw-merged-program-output/acceptance.md`（FR-005）。
- [x] T002 [US1/US2] 更新 `epwflow/tests/support/direct_submit.rs`，覆盖实时双路输出、版本 1／2 兼容及缺失旧日志拒绝（FR-001／002／004／006）。
- [x] T003 [US1/US2] 修改 `epwflow/src/foundation/native_jobs.rs`、`process.rs`、`workspace_electronic.rs`、`workspace_science.rs` 和 `epwflow/src/superconductivity/{attempt,batch}.rs`，统一生成、日志归集与产物登记（FR-001～004／006）。
- [x] T004 [US1/US2] 在 `epwflow/tests/support/` 验证声子链、全阶段和 iso／aniso 方法实际执行与恢复；运行相关回归（SC-001～004）。
- [x] T005 核对并迁移可修改现用脚本，证明冻结脚本不变，更新 `epwflow/README.md`、MgB₂ 示例说明及 `docs/overview/epwflow/README.md`（FR-005）。
- [x] T006 验证文档、完成验收记录、重新编译 EPW release 三程序并核验 `--help`，按明确路径提交；保留默认安装（SC-001～004）。

依赖顺序 T001 → T002 → T003 → T004 → T005 → T006。同一文件串行修改；不委派子代理，不提交科学作业。实现证据写入验收后再勾选。

完成依据：[验收记录](acceptance.md) 的需求覆盖、全套回归、现场清点、release 构建及明确范围提交。
