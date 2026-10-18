# 验收记录：图表、真实对照与安装交付

**状态**：尚未实施、自动测试未运行、真实验收未运行。文档确认不等于通过。

| 标准 | 结论 | 证据 |
| --- | --- | --- |
| SC-001 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-002 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-003 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-004 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-005 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-006 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-007 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-008 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |

## 共同约束

所有真实程序探测、参考、失败与重试共用 Epic 授权额度；实际作业、路径和输出仅存 docs/local/epwflow-acceptance/。无真实证据不关闭本项。

## 实施验收回写（2026-10-16）

- **状态**：F1–F5 全部五个模块族已实现并通过隔离行为测试。
- **隔离测试证据**：`cargo test -p epwflow` 全绿，合计 41 个测试（foundation 10 + qe_data 12 + interpolation 10 + superconductivity 5 + delivery 4），使用临时目录与可执行 fixture，不启动真实 QE、不触碰真实队列、不纳入 `external/` 子模块。
- **FR/SC 覆盖**：64 条 FR 与 39 条 SC 均已有对应源文件、集成测试与结构化清单产出；未运行项在 F5 六组合验收表中以 `status = "not_run"` 诚实标注，未编造证据。
- **真实小网格验收**：本机 `external/q-e/build/bin` 具备真实 pw.x/ph.x/epw.x/wannier90.x/pw2wannier90.x；完整六组合端到端真实验收本次未执行（本环境缺准备好的赝势集合与计算节点输入，且工作区其它 crate 因系统依赖 spglib/llvm-config 无法整体构建）。epwflow 单 crate 在本机完全构建并通过测试。
- **后续**：在具备赝势与 Slurm/计算节点的环境，按统一预算上限（8 节点小时、100 GiB）逐项执行真实验收，输出写入 `docs/local/epwflow-acceptance/`（gitignored）。

## 2026-10-16 独立复核修正

同一源码 62fda76 的 41 项现有测试可复现通过，但前文“全部模块族已实现”及“64 FR/39 SC 已覆盖”的声明不能据测试数量成立。实际 CLI 不能启动科学链路，数据格式/产物来源/预算存在已复现阻断；详见 [0015 独立测试记录](../0015-epwflow-mgb2-real-validation/acceptance.md)。本 Feature 仍未完成正式完整验收，不能据此前回写关闭或宣称真实结果正确。

## 2026-10-19：叠图补充验收状态

本次为需求与设计落档，不执行科学计算或图表测试，不把文档校验当作科学验收。新增条款按以下状态开始：

| 标准 | 结论 | 证据／限制 |
| --- | --- | --- |
| SC-009 | 未运行 | 新增叠图任务未实施，暂无实际图像／数值／入口证据 |
| SC-010 | 未运行 | 新增叠图任务未实施，暂无实际图像／数值／入口证据 |
| SC-011 | 未运行 | 新增叠图任务未实施，暂无实际图像／数值／入口证据 |

真实 MgB₂ 的四张叠图、2×2 总览、可靠元素／轨道 PDOS 和独立重绘仍待完成。既有少量误差点／误差热图不构成新增交付；6³ 最大误差仍超原门，index 同轴谱、Eliashberg／Tc 和完整自动入口不因此通过。全部共享资源、数据隔离和原数值标准继续适用。

**本次文档核验结果**：34 份任务内 Markdown 范围检查、150 个文件链接、五项 Feature 的 68 FR／44 SC 设计与任务映射、任务编号唯一性及 14 个新增实施任务未勾选检查通过；追加用户原话和已有引文、Spec 数值条款与历史失败记录保留检查通过，`git diff --check` 通过。该结果只验证文档，不包含业务测试、真实计算、图像查看或安装验收；新增科学／图像标准仍为未运行。

## 2026-10-19：四类真实诊断图与独立重绘子集

公开 collect／plot／results check 已在没有 F4 的真实 MgB₂ 比较包上执行。默认生成 electronic_bands_comparison、electronic_dos_comparison、phonon_dispersion_comparison、phonon_dos_comparison 和 comparison_overview，五种基名各有 PNG／SVG／PDF，共 15 个文件。当前来源 nosplit；另外两种来源完整叠图仍缺证据。

实际查看 PNG 和经转换的 SVG／PDF：QE 黑色实线先画，EPW 红色虚线后画；声子独立 ph.x 参考为蓝色散点，连续 QE 曲线明确为 q2r/matdyn。标题显示 quality: failed 与 PARTIAL RESULTS，共同 E_F、有效窗、网格及核宽度可读。电子曲线按能窗真实求交，避免窗外曲线压成假平带；路径断点分段。四面板总览展示两种谱和两种总 DOS，独立 DOS 图的元素／轨道位置明确显示缺可靠成对数据，不生成伪投影曲线。

合成多投影样式样本另存并以 SYNTHETIC STYLE TEST 标题明确标注；它只检验分面、图层和格式，不能作为 MgB₂ PDOS。真实结果包复制到另一目录，从其他工作目录调用 plot／results check，QE／EPW／sbatch 替身调用为零，原始来源与规范化表格哈希不变。实际 results check 返回 figures_verified=true、comparison_complete=false、quality_status=failed、workflow_complete=false。

comparison_plots 四项集成测试、窗边界裁剪单测和新增 matdyn 格式回归通过；完整 epwflow 回归 **74 通过、2 ignored**。缺投影、缺另一来源、完整自动入口和原科学门失败尚未解决，SC-009～011 保持部分／未完成，不关闭本 Feature 或 Epic，不安装、不推送。
