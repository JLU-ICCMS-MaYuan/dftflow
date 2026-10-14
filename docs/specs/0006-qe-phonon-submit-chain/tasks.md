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

## 补充收敛任务：US3

- [x] T008 [US3] 记录确认后的原始需求、接口与测试设计，冻结迁移前工具 MgB₂ 输出。
- [x] T009 [US3] 独立 Rust 工具、主项目算法解耦及测试迁移。
- [x] T010 [US3] 共享 QE 初始化实现与 qe_ph_nosplit --dyn0 准备/执行/提交。
- [x] T011 [US3] 模拟初始化失败边界、workspace 97 项回归、独立构建及 4 项测试；证据见补充 acceptance。
- [ ] T013 [US3] 真实隔离初始化：作业 1250893 已提交，PENDING，待结果验证。
- [x] T012 [US3] 手册、总览、AGENTS、51 入口隔离安装与旧链接迁移、制品验证及提交。
- [ ] T014 [US3] 真实初始化通过后切换本机默认版本，核对入口与制品。

依赖：T008→T009/T010→T011→T012；以上代码改动串行实施。T013 依赖 T011，T014 依赖 T012/T013。FR/SC 映射见 plan.md。
