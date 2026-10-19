# 本地需求 0016：完善 Wannier 外窗与冻结窗上下限的选择

**类型**：`type:feature`。**状态**：已按用户最新要求撤回本轮新版源码／编译内容；组合投影选窗方向已放弃，新纯能量窗口为调研建议，尚未实现。线上自管约定不变。

0016 沿用本仓库本地需求编号约定，不是已经创建的 Gitee Issue number。远端为 Gitee，线上 Issue 仍由用户管理；本次未创建线上记录、未推送。正式线上编号确定后再核对目录映射，不宣称远程双向链接有效。

## 原始需求记录

> 调研：如何完善 dis_win和 frozen_win这两个窗口的上下限的选择，先给我你当前的选择窗口上下限的方案，然后再调研。你要明白：你需要先调研清楚已有的开源项目，或者文献，然后再给我答案。使用 issue 和 spec 新建一个需求，然后开始做这件事。


> 我想让你把[https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows)作为 external 里面的子仓库，然后完全采用这个[https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows)里面的设置窗口的方案来构建。而且你可以再 看看[https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows)有没有其他可以优化我的epwflow 和 wannierflow 代码的地方，都可以借鉴一下。

> 我希望你使用这个仓库的默认方案是“E_F/导带底以上约 2 eV 的能量冻结＋投影解纠缠”。你说的：新投影解纠缠参数是什么参数？？？？？

> 我希望你完成采用[https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows)里面设置窗口的方式。之前的方式🔛

> 我希望你完成采用[https\://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows]\(https\://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows)里面设置窗口的方式。之前的方式弃用。

> 我希望你能直接在/data/home/mayuan/code/dftflow/external/wannier90 这里操作把 4.0.3 的版本拉取过来，然后在/data/home/mayuan/code/dftflow/external/q-e里面获得新的 qe 的代码，然后编译一个新的 epw

> 我建议还是放弃之前的解纠缠窗口的设计。改用其他方案，明显调研之后发现，所有的 epw 都不支持 wannier90 4.0以上版本。你来帮我看看怎么设计新的解纠缠窗口和 frozen 窗口。而且你帮我把所有的 qe 和wannier 和 epw 的编译内容也都修改回去。编译出来的东西也都删掉。

## 最新取舍与本轮目标

以最后一条原话为准，停止并撤回新 QE／W90／EPW 构建，删除新增源码、接口适配和编译产物；基于一手源码及文献研究现有 EPW 可用的纯能量四边界。新建议和真实逐点计数见 [兼容设计调研](energy-window-redesign.md)。以下旧目标、范围、验收与 Documentation Impact 是上一阶段的历史记录，不表示组合方案仍获实施授权；本次不创建或关闭线上 Issue，不启动新科学试算。

## 上一阶段目标与价值

先解释现状并完成外部调研，再按用户最新指令采用指定仓库默认的能量冻结＋投影解纠缠，统一 epwflow 与 wannierflow 的选择与原生输入，弃用原带号启发式，不静默回退。

## 范围

- 引入固定提交的 `external/aiida-wannier90-workflows` Git 子模块，保留许可与来源。
- 使用共享 Rust 模块复现默认约 2 eV、投影阈值 0.01／0.95、能量平移与冻结上界限制；未设置外窗／下界保留原生默认。
- 两套 flow 使用真实本征值，记录来源指纹、参数与逐点能量态数；实际投影由原生程序读取。
- 修改检查命令及两类原生输入，删除旧自动算法，设置执行前必要能力门。
- 审计其他可借鉴项；本次同时修复 wannierflow 的空参数写入和生成脚本失败传播，其他发现不冒充已修复。

## 范围外

本次不启动新 QE／PH／EPW 作业或阈值扫描，不升级／覆盖科学程序，不自动扩大物理基底，不更换赝势／泛函，不恢复暂缓的 DOS 投影、不放宽质量门或预算，不切换默认安装、不推送。实际科学程序不支持时明确阻断。

## 关系与依赖

本需求属于 [Epic 0007](../0007-epwflow-superconductivity/issue-draft.md) 领域，拟作为其补充 Feature；反向登记待草案确认，不改写已有协作状态。依赖 [0012](../0012-epwflow-interpolation/spec.md) 的原生数据、能量基准与质量检查，复用 [0014](../0014-epwflow-delivery/spec.md) 的图表，真实 MgB₂ 对照衔接 [0015](../0015-epwflow-mgb2-real-validation/spec.md)。独立建需求是用户本轮明确要求，不替代或关闭既有 Feature。

## 验收与关闭条件

1. 当前窗口调查与指定上游行为均可按固定源码、日志和实际能谱复核。
2. 共享策略进入两个 flow 的最终原生输入，阈值与平移／上限行为和固定上游一致；缺少本征值或必要能力时不回退。
3. 旧自动算法已删除，旧 CLI 参数明确报弃用；已有科学结果读取和只读带号／能量分析仍可用。
4. 相关数值、错误边界、CLI、输入生成与差分检查通过，记录真实 MgB₂ 只读结果及本机能力限制。
5. 源码／输入验证不等于新窗口原生质量通过；电子最大／RMS 0.05／0.02 eV，声子最大／RMS 50／2 cm⁻¹及其他科学验收标准不变。
6. 线上关系与完整科学验证不在本轮伪造完成；本地实现验证完成不自动关闭线上需求。

## Documentation Impact

- [x] Issue／Spec 保存完整原始需求和最新取舍；固定来源、计划、任务、审计与采用依据分别落档。
- [x] 当前功能总览及 CLI 使用说明已按实际接入与限制回写，验证见 acceptance.md。
- [ ] 线上 Issue 编号、Epic 反向关系和远程 Spec 链接：沿用用户自管约定，不为补链接自动推送。

## 本次产物

[Spec](spec.md)、[当前选择](current-selection.md)、[前期调研](research.md)、[指定上游行为](upstream-adoption.md)、[复用审计](reuse-audit.md)、[计划](plan.md)、[任务](tasks.md)、[快速验证](quickstart.md)、[验收](acceptance.md)。


## 最新需求结论：撤回构建并重新设计

用户最新原话取代此前组合投影与新版构建要求。已完成本轮新增编译内容清理；旧组合选窗源码尚未替换，不能宣称现有两套 flow 已切为纯能量默认。新的四界生成、容量检查和有界候选建议见 [兼容设计调研](energy-window-redesign.md)，属于 AI 根据实证提出的待确认设计。

本轮验收为清理范围准确、原有安装未改、原生构建进程结束、现有真实能谱计数可复核，以及新建议明确区分版本兼容、必要可行性与科学拟合质量。当前不批准新的候选扫描、不扩大轨道或预算，原电子／声子质量门保持不变。下一阶段若实施，须先改写受影响的 Spec、Plan、Tasks 和接口要求，不把历史复选框当成新方案完成证据。


## 最新用户确认与文档影响

> 增加纯能量窗口模式，使用现有 EPW

沿用本地 0016，关联 0012 第二阶段；先交付显式 energy_only、四界记录与行为回归，不升级科学程序或自动扫描。Documentation Impact：更新本 Spec／Plan／Tasks／验收、epwflow README 与 Overview；真实质量及两套 flow 默认迁移仍保持未完成。

实际 PBE 原生执行确认 EPW 5.9 会清空 namelist 中的外窗下界。本次兼容修复将纯能量的下界仅通过受控 wdata 传给内嵌 W90，避免重复关键字，保持选择规则和科学参数；完整定向回归 90 通过、2 项真实环境暂挂。原生实际四界仍由 0012 T062 续验，独立的双 FFT 缺陷及 NaN 不算本项修复成果。Documentation Impact 已落实到本项计划／任务／验收、README、Overview 和 0015 真实记录，不关闭整体科学需求。
