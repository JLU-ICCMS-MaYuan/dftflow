# 0051 验收记录

日期：2026-10-22。范围：用户“开始落地”授权的 FR-001～008、SC-001～005。本地代码、受控阶段脚本及既有数据只读图表分别记录；不将出图通过解释为科学收敛。

## 实现与隔离范围

新增 `delivery/decay.rs` 及其测试，复用 Plotters／svg2pdf、Identity／Artifact 和发布 journal；在粗网格科学回执就绪后的 `refresh_post` 首先交付，并接入现有阶段 results 导航、核验和重绘。没有新公开命令、依赖、配置、科学门或网格扫描。科学身份排除 `decay/` 派生目录。

Completion 仅对粗网格回执中已绑定的四类 decay 文件缺失保留科学复用资格，衰减收集列出缺项而不重算；已存在但内容／链接改变仍拒绝。其他科学输出沿用原检查，回执与输出键结构不变。重绘逐点核对包内原生副本与 JSON，不把更新哈希后的错误数据视为有效。

当前共享工作树有 0050 等并发修改。本次另外归档 `9da0891db90ac2877243cdaabe5f98327d777cde`，只加入本任务十个源码／测试路径，在 `docs/local/0051-real-space-decay/source-validation/` 验证；候选路径与主工作树逐字节核对。`workspace.rs` 从基线仅加入衰减锁恢复调用；该副本不包含此文件及 `electronic_sampling.rs`、`automatic_comparison.rs`、旧 `support/stage_results.rs` 的并发修改。构建环境仅复用本机忽略的 build env 和编译缓存。

## 回归命令与结果

以下 Cargo 命令均通过 `CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh` 执行，隔离副本另外设置原仓库的绝对 `CARGO_TARGET_DIR`。

| 检查 | 结果与边界 |
| --- | --- |
| 主工作树 `check -p epwflow --lib` | 通过；只有既有 qf-scflow 未使用变量等警告 |
| 主工作树 `test -p epwflow --lib` | 37 通过、2 默认暂挂；包括并发任务新增的一个库用例，不作为隔离候选计数 |
| 主工作树 `test -p epwflow --test three_stage_execution decay:: -- --nocapture` | 1 通过；实际公开程序与生成的阶段脚本，科学 IO／调度为替身 |
| 隔离候选 `test -p epwflow --lib` | 36 通过、2 默认暂挂；未包含并发新增库用例 |
| 隔离候选 `test -p epwflow --test three_stage_execution stage_results:: -- --nocapture` | 6 通过；既有阶段出图、恢复、来源拒绝和历史导入回归 |
| 隔离候选新增衰减公开进程回归 | 最终版 1 通过（137.62 秒），包括错误锁所有者拒绝、终态原作业恢复、缺图补绘和科学调用不增加 |
| 显式真实衰减测试 | 1 通过；空 PATH 下只读既有原生文件，不启动科学程序 |

默认暂挂两项分别为：新增真实衰减出图测试（要求显式原生回执及全新输出目录，本轮已单独执行通过），以及已有 `real_workspace_wdata_readonly`（需专用真实配置，本轮未执行）。暂挂不计通过。

## 真实 MgB₂ 图表

输入为 `examples/MgB2/sugon-amd/run/epwflow/6.epw-coarse/completion.json` 绑定的四份文件。绘图前核验粗网格回执，绘图后逐项重新核验其全部 106 份输出身份，并比较回执原字节，全部保持不变。

最终图表位于忽略的 `test_auto/MgB2/local_run/013-real-space-decay-final/`：

- [PNG 总览](../../../test_auto/MgB2/local_run/013-real-space-decay-final/decay-overview.png)
- [SVG 总览](../../../test_auto/MgB2/local_run/013-real-space-decay-final/decay-overview.svg)
- [PDF 总览](../../../test_auto/MgB2/local_run/013-real-space-decay-final/decay-overview.pdf)

显式运行环境为 `DFTFLOW_DECAY_REAL_COMPLETION=<上述绝对回执路径>`、`DFTFLOW_DECAY_REAL_OUTPUT=<全新最终输出路径>`、`PATH=""`，执行库测试二进制的 `--ignored --nocapture --exact delivery::decay::tests::real_bound_native_decay_files_render_without_scientific_calls`。本测试不写原工作区，不调用外部命令。初版图表另保留在 `013-real-space-decay/`，最终版减少纵轴标签密度；未覆盖初版。

| 原生文件 | 数据行 | 不同距离数量 | 逐行核对 |
| --- | --- | --- | --- |
| `decay.H` | 57 | 14 | 与原生前两列和来源哈希一致 |
| `decay.dynmat` | 7 | 3 | 同上 |
| `decay.epmate` | 57 | 10 | 同上 |
| `decay.epmatp` | 7 | 2 | 同上 |

包内清单绑定 20 个文件：四份原生副本、绘图数据 JSON、15 图；清单自身为第 21 个文件。阶段发布时另用既有发布单元清单绑定。科学键为 `15733d4cc4fca67ad24d07daa994a9c1b4a0ea6dd98b458490e16be154dccbde`，来源回执 SHA-256 为 `f0f2993ab24e0caddc89249876f9ac196caa6c896f61fcce744d8cf4bdb4833a`。

实看 PNG 独立 H 图和总览；通过 rsvg-convert 栅格化 SVG、pdftoppm 栅格化 PDF 后实看总览。三格式均有正确的面板顺序、紫色散点、距离／Ry 标签和对数纵轴，无连线、拟合或平均。MgB₂ 数值范围来自本次文件，不套用讲义 Pb 的范围。同距点可能视觉重合，数据与 SVG 点数仍完整保留。

## SC 覆盖

| 标准 | 验证证据 |
| --- | --- |
| SC-001 | 15 图格式签名／标签检查及上述真实三格式实看；真实四组点数均保留 |
| SC-002 | 解析用例覆盖 E／D、重复距离、零值、全零、单点及常数幅值；坏行、负数、NaN／Inf／溢出拒绝，错误含文件和行；JSON 与原始副本逐点一致检查 |
| SC-003 | 新公开进程测试先注入第 6 步 matdyn 比较失败，粗网格科学回执完成后仍产生完整 decay 发布单元；重绘调用记录和空 PATH 验证不增加科学调用／作业 |
| SC-004 | journal 部分发布可恢复，未知目标文件拒绝覆盖；终态作业遗留锁只在所有者匹配且调度器确认结束后恢复，错误所有者拒绝；缺图仅重绘，重复收集可复用；缺两个原生文件列出全部缺项，科学调用数和回执不变；已存在但篡改拒绝 |
| SC-005 | 单元测试移动包并移走原科学目录后成功重绘；公开进程移动完整 results 后空 PATH 重绘并通过 results 校验；无第 7 步且声子比较失败时仍完整交付 |

## 文档与交付边界

已同步本地 Issue、Spec、需求检查表、计划、研究、任务、快速说明，以及 EPW Overview、模块手册和 MgB₂ `RESULTS.md`；不覆盖并发修改的算例 README。根 README 已有 EPW 导航，本项不新增公开入口，保持原导航。原始需求三个围栏在 Issue／Spec 中与建档提交逐字一致。文档链接、需求编号、Python fixture 语法及精确范围 diff 在提交前核验。

本轮无新增真实科学作业、科学输入／数据修改、安装、冻结脚本迁移、线上 Issue 操作或推送。线上标签／关系／双向链接由用户管理，尚未核验；聊天截图仍为仅预览，未声称原图归档。功能本地验收与远程 Issue 关闭分开处理。

最终源码复核：锁恢复补充后库回归再次 36 通过、2 默认暂挂；公开衰减进程回归再次 1 通过。该测试现场为 `/tmp/public-chain-18e0e078971845b9-38c85f-0`，只含受控替身计算；临时证据目录可能随系统清理，测试源码与上述复现命令随仓库保存。

锁恢复补充后的既有阶段发布六项回归再次全部通过（106.25 秒），最终隔离候选合计 36＋6＋1＝43 项通过，2 项默认暂挂单列。真实衰减只读测试另外显式通过。最终 146 条本地链接、原始用户围栏、FR／SC 编号与 fixture 语法检查通过；任务使用精确路径和共享文件单独补丁提交。


## 后续合并真实验收

用户授权 0046～0051 联合交付后，复用从 SCF 走通八阶段的独立 MgB₂ 现场；公开 coarse 作业 1334753 自动补交付全部 15 张实空间衰减图，没有重跑成功科学任务。18 份科学回执／原始衰减数据哈希不变，图表已按 PNG／SVG／PDF 实看；合并回归、移动重绘、最终安装和 examples 清理的完成状态统一见 [联合验收记录](../0050-epw-stage-results/real-chain-validation.md)。首轮未部署边界为历史记录，不代表后续状态。


联合收尾已完成：仓库及默认三程序入口与真实受测制品哈希一致；examples 旧现场 1394 项移出归档校验一致，输入保护不变，新目录公开检查通过；手动 README／RESULTS 与总览已同步。最终回归为 EPW 248＋QE 34＝282 项通过，默认暂挂 10 项中两项真实只读另行通过，其余历史来源／Tc 测试不计通过。全过程科学调用与阶段回执核验、预算及路径证据见联合验收；线上状态仍由用户管理。
