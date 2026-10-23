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

## 后续现场重跑验证

- [x] T007 新增公开 CLI 回归，验证保留旧版 SCF 目录时失败恢复会归档原脚本与两份日志，在原路径生成版本 3 合并输出脚本，并执行和归集成功。
- [x] T008 重跑 EPW 全套与共享 QE 回归，核验并更新三个 EPW 运行制品；保留旧现场后执行一次新格式 SCF，记录作业、日志身份与预算。EPW 全量 250 通过／1 项路径异常，失败用例最终复验通过，覆盖 251 项；共享 63 通过。真实作业 1344407 成功，旧 18 文件及新回执 30 产物核验通过，预留归零。
- [x] T009 按用户明确确认修改 `native_workspace` 的显式 SCF 恢复提交语义；验证成功后连续两次重跑、旧回执／快照归档、活动拒绝和普通归集／复用（FR-007、SC-005）。最终源码的三项 SCF 定向回归通过，含提交及待办归集提示。

T007 对应 FR-003～005、SC-003／004；T008 对应 FR-001／002／005／007、SC-001～005。依赖 T007 → T009 → T008；仅真实 SCF 现场验证新增一项科学执行，其他阶段由隔离公开进程覆盖。
