# 实施任务：粗网格实空间衰减图

输入：[Spec](spec.md)、[计划](plan.md)、[研究](research.md)。按 T001→T006 串行完成；共享文件修改不得覆盖并发工作。

- [x] T001 复核需求与授权，补齐本目录 plan.md、research.md、tasks.md，建立 FR／SC→职责→任务→验证映射。
- [x] T002 [US1] 在 `epwflow/src/delivery/decay.rs` 实现绑定来源解析、自包含数据和语义检查；验证重复距离、E／D、零值、坏数和缺项列表。
- [x] T003 [US1] 在同模块复用 Plotters／svg2pdf 实现四张散点图及总览、15 图、对数刻度、退化范围与全零说明，复用 publication 发布。
- [x] T004 [US2] 接入 `foundation/workspace_science.rs`、`foundation/stages.rs` 与 `foundation/stage_results.rs`：科学完成后独立发布、缺诊断数据不重跑科学、修改数据拒绝、派生结果恢复及现有重绘入口。
- [x] T005 [US1] [US2] [US3] 新增解析／包恢复测试及 `epwflow/tests/support/decay.rs` 公开进程测试；补原生 IO fixture 的 decay 输出，验证实际脚本自动交付、对比失败仍发布、恢复不增加科学调用和移动后空 PATH 重绘。
- [x] T006 [US3] 对既有 MgB₂ 原生数据只读出图并实看 PNG／SVG／PDF，运行相关回归，完成 quickstart.md、acceptance.md，更新 Overview／手册并精确范围提交；不安装或运行科学作业。

覆盖关系见计划中的矩阵。只有对应生产路径及边界验证通过后，才勾选 T002～T006；真实数据绘图不代替新增科学计算验收，线上关联与图片原件缺口继续单列。
