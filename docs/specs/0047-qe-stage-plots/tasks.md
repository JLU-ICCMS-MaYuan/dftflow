# 实施任务：QE 分阶段即时出图

输入：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据契约](data-model.md)、[接口契约](contracts/plots.md)。

## 准备

- [x] T001 追加用户实施原话，完成本目录设计文档及 FR／SC 覆盖核对，保存重叠文件任务前基线。

## US1：电子子任务即时出图

- [x] T002 [US1] 在 `epwflow/tests/support/qe_stage_plots.rs` 增加公开进程用例，覆盖无声子／拟合、子任务失败与独立出图。
- [x] T003 [US1] 在 `workspace_plot.rs`、`workspace_qe_plot.rs`、`workspace_electronic.rs` 及复用入口接入单侧电子图与仅补图。

## US2：QE 声子与元素投影

- [x] T004 [US2] 在 `epwflow/tests/support/qe_stage_plots.rs` 覆盖元素贡献、投影列缺失拒绝、删除图片和绘图失败恢复时的科学调用计数。
- [x] T005 [US2] 在 `workspace_science.rs`、`workspace_qe_plot.rs` 接通 qf-phonon 绘图、完整投影检查与数值后处理检查点。

## US3：插值叠图边界

- [x] T006 [US3] 修改 `delivery/comparison_plot.rs` 与 `interpolation/comparison.rs`，保留电子叠图，声子 DOS 仅 TDOS；在 `epwflow/tests/comparison_plots.rs` 验证没有元素曲线／占位。

## 收尾

- [x] T007 完成公开 -r／-s、现有电子叠图与比较回归；使用真实已有数据只读出图、核对来源哈希并实看，记录 `acceptance.md`。
- [x] T008 按已验证行为更新 `epwflow/README.md`、`docs/overview/epwflow/README.md` 和本目录，检查链接／差异，仅提交本任务增量。

## 依赖与覆盖

T001 → T002／T003 → T004／T005 → T006 → T007 → T008。同一文件串行，测试仅证明其覆盖的边界；真实科学计算和安装另需单独任务。

| 来源 | 任务 |
|---|---|
| FR-001～003／SC-001 | T002、T003 |
| FR-004～005／SC-002 | T004、T005 |
| FR-006～009／SC-003～004 | T006、T007 |
| FR-010／SC-005 | T003、T004、T005 |
| FR-011／SC-006 | T007、T008 |

MVP 为 US1；完成后扩展 US2 和 US3，最终共同验证，不把单元通过当成科学验收。

验证证据见 [验收记录](acceptance.md)：常规回归分批复测后去重 211 项通过、8 项按条件跳过；真实数据只读绘图 1 项显式通过。文档链接与任务补丁校验通过，T008 随本次自动本地提交保存；线上登记与远程双向链接仍待用户管理。
