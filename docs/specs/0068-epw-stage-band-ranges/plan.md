# 实施计划：三阶段能带范围诊断

日期：2026-11-13。需求：[spec.md](spec.md)，Issue：[本地正文](issue-draft.md)。

## 技术上下文与设计

Rust 2021；复用 qf-wannier 窗口推导、现有 QE XML 解析、qf-plot SVG 栅格化和 300 DPI PNG 编码、八阶段日志及 stage_results。无数据库或新依赖；Linux／macOS 共享渲染路径，不硬编码本机 MPI。

新增 foundation/band_ranges.rs 单独负责读取、统计、渲染与派生缓存。纯能量四边界从 qf-wannier 提取无数量检查的共享解析函数；严格 select_energy_only 继续原检查。组合模式沿用 select_default，只将可解析边界显示，失败时保留显式边界与原因，不伪造原生默认。

SCF 读取已核验 snapshot XML，NSCF 读取自身 out XML，electronic 读取路径 PW XML并绑定 NSCF E_F。SCF 无 NSCF 依赖。自动窗基于当前数据仅为预览，最终 Wannier 仍读正式 NSCF。

## 触点与责任

- crates/qf-wannier/src/lib.rs：抽取纯能量边界转换，生产严格选择共用。
- epwflow/src/foundation/band_ranges.rs：逐带报告、缓存与 SVG／PNG／CSV／JSON。
- workspace_qe_plot.rs：将 SCF／NSCF 缺图与即时刷新纳入已有后处理。
- native_workspace.rs：SCF／NSCF 完成后同其他阶段一样刷新派生交付。
- workspace_science.rs：后处理遍历包含前两步。
- workspace_electronic.rs：路径 PW 完成后、其他电子程序之前生成范围图。
- tests/support/stage_results.rs：公开提交／分配执行契约。

输出位置和缓存见 [契约](contracts/band-ranges.md)，数据实体见 [数据模型](data-model.md)。

## 质量门

| 来源 | 要求 | 方案与结论 |
| --- | --- | --- |
| 用户已确认总结 | 三步独立即时绘制 | SCF／NSCF 阶段后处理及路径 PW 子任务完成点接入 |
| AGENTS 0067 | 不检查密度和波函数 | 只解析 XML／配置，图像独立派生，不读取 raw payload |
| KISS／DRY | 不复制科学选窗逻辑 | 提取纯能量解析，共用既有编码与发布 |
| 既有科学边界 | 选窗失败不能放行 | 诊断计数与严格选择分离，测试两者并存 |
| 既有线上约定 | 不操作错误远端 | 0068 本地正文待用户发布 |

## FR／SC → 设计 → 任务 → 验证

| 来源 | 设计 | 任务 | 验证 |
| --- | --- | --- | --- |
| FR-001／SC-001 | 阶段完成与叶子完成触点 | T002、T003 | 提交脚本执行后、下一阶段前已有图 |
| FR-002～005／SC-002 | XML 报告、共享解析、SVG | T001、T002 | 独立极值／参考／排除带测试及三阶段真实图 |
| FR-006／SC-003 | 诊断不抛数量超限，正式选择保留检查 | T001、T004 | 超限出图但 Wannier 拒绝 |
| FR-007～008／SC-003～004 | 派生缓存与发布 | T002～T005 | 丢图补齐、配置变化、重复执行、300 DPI 与来源不变 |

顺序：共享边界 → 共用诊断 → 完成点接入 → 公开回归与真实补图 → 覆盖构建、文档与提交。无需分布式调度或额外常驻进程。
