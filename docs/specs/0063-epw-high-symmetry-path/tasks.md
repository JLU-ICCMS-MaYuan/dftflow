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

## 自定义文件头回归修复

- [x] T008 [US1/US3] 复现 SiP₂ 自定义标题拒绝，按 FR-011 修复完整四行文件头识别，补任意标题、裸端点及正文拒绝测试。证据：验收记录的原样复现与受控验证。
- [x] T009 [US2/US3] 使用 SiP₂ 原样路径验证公开 CLI 输入生成与显式恢复，证明路径哈希和已有电子证据保持不变、无额外前置科学调用。证据：`sip2-cli-resume.log`、`sip2-readonly.log`。
- [x] T010 更新本 Issue／Spec 的原因、证据和操作说明；验证后从隔离源码编译发布正式 CLI，核验命令路径及制品哈希，按任务范围自动提交，不提交科学作业。证据：验收记录的发布与安装后跨目录恢复。

## 失败任务归档交接续修

- [x] T011 [US3] 复现失败模型归档后输入生成再次失败的链，修复旧活动记录归档交接与严格遗留接续，补缺失、损坏、歧义、同代及未知任务拒绝回归。证据：续修验收的 67 项隔离回归及真实旧 CLI 复现。
- [x] T012 [US3] 使用真实 SiP₂ 原有失败状态执行生产恢复准备，不提交作业；核验 `s,p`／500 次输入、脚本、旧记录保留和前置科学身份不变。证据：`real-resume-prepare.log`，1 通过；`real-prepared-before-formal.json`。
- [x] T013 更新 0063 的验收、操作说明与当前行为，从隔离源码发布正式 CLI，完成真实目录 CLI 准备核验并自动提交任务差异。证据：`published.json`、`real-installed-cli-prepare.log`、`real-state-after.json` 及本次 Git 提交；无真实作业提交。
