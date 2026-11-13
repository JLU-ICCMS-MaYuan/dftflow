# 实施任务

输入：[Spec](spec.md)、[Plan](plan.md)、[Research](research.md)、[数据模型](data-model.md)、[契约](contracts/band-ranges.md)。

- [x] T001 [US2] 在 crates/qf-wannier/src/lib.rs 提取纯能量边界推导；保持严格选择不变，测试数量超限仍可解析边界。
- [x] T002 [US1/US2] 在 epwflow/src/foundation/band_ranges.rs 实现共用报告、布局、缓存和阶段派生发布。
- [x] T003 [US1] 在 native_workspace.rs、workspace_science.rs、workspace_qe_plot.rs、workspace_electronic.rs 接入阶段／路径 PW 完成点与复用补图。
- [x] T004 [US1/US2] 在 epwflow/tests/support/stage_results.rs 验证公开阶段即时出图、独立数据、补图无重算、非法窗口仍出图；在 band_ranges 单元测试独立验证极值／计数。
- [x] T005 在 examples/MgB2/ubuntu 复用真实结果生成三图、实看、核对来源未改、合理回归、重编译固定入口；更新 Overview、根／模块 README、验收及独立提交。Ubuntu 输入与手册由并行任务修改，不混入本次提交。

顺序 T001→T002→T003→T004→T005；同一文件顺序编辑，不委派并行代理。FR／SC 覆盖见 Plan。技术勾选仅表示证据支持的实现完成，不替代线上 Issue 状态。

任务完成证据见 [验收记录](acceptance.md) 的 FR／SC 对照、回归、真实结果和构建核对；线上发布不作为技术任务完成的替代状态。
