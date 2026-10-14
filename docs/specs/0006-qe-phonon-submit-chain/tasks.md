# 实施任务

输入：[spec.md](spec.md)、[plan.md](plan.md)、[research.md](research.md)、[data-model.md](data-model.md)、[contracts](contracts/cli.md)。

## 准备

- [x] T001 完整文档与门检查，docs/specs/0006-qe-phonon-submit-chain/。

## 基础与测试

- [x] T002 [US1] 编写模拟调度集成测试，qeflow/tests/phonon_chain.rs；实现原生提交与查询，crates/qf-runner/src/lib.rs。
- [x] T003 [US2] 坐标、快照隔离、outdir 合并与旧目录测试，crates/qf-phonon/tests/phonon.rs；相应数据实现与 crates/qf-state/src/lib.rs。

## 完整链路

- [x] T004 [US1] 实现全部阶段、失败恢复和输出验证，qeflow/src/phonon_chain.rs、qeflow/src/bin/qeflow.rs。
- [x] T005 [US1] 三模式准备入口、手册、模拟验收与 workspace 回归，examples/MgB2/prepare.sh、examples/MgB2/*/README.md。

## 验收与收尾

- [x] T006 [US2] 在 sugon-amd 实际跑 index/coords/nosplit 并对照，证据 docs/local/、本目录 acceptance.md。
- [x] T007 安装制品与来源校验，更新 README、docs/overview/dftflow-rust.md、AGENTS.md 并提交。

## 覆盖与顺序

T001→T002/T003→T004→T005→T006→T007；共享文件串行。FR-001/004 对应 T002/T004，FR-002/003 对应 T003/T004，FR-005 对应 T003/T004/T006，FR-006 对应 T005/T007；SC-001～004 由 T002～T007 覆盖。
