# 验收说明与证据边界

日期：2026-10-22。关联：[Issue](issue-draft.md)、[Spec](spec.md)、[任务](tasks.md)。

## 最新联合验收入口

初次实现的替身与只读证据保留在下文。用户随后授权真实从头验收、安装和 examples 清理，并将 0051 纳入联合交付；后续事实以 [0046～0051 联合验收](real-chain-validation.md)为准。下文“未执行／未部署”为初次源码交付当时的边界，不代表最新联合验收状态。

## 初次授权和交付

前轮 `0919b5e` 保存完整需求与设计文档；用户随后明确“开始落地”，本轮实施生产代码、隔离回归和文档同步。沿用 0050 本地暂定编号，0049 已被其他需求占用。线上 Gitee Issue、远程链接与状态仍由用户管理。

本轮交付第 5 步独立电子对比、第 6 步声子对比和四面板复用、根 results 按阶段发布、作业内自动收尾及有界旧采样读取。第 3／4 步继续输出 QE 单侧图；电子不等待声子；第 4 步保留真实逐原子数据汇总的元素 DOS，第 6 步仅声子 TDOS。第 7 步没有既有独立最终图，不额外增加任务。

未更新默认安装，未改动真实 MgB₂ 科学数据或冻结脚本，未提交真实 Slurm、运行真实 QE／EPW／postw90 或推送。工作树原有 `AGENTS.md`、MgB₂ README 的其他修改保留。本需求使用独立 RESULTS.md 补充 MgB₂ 操作说明。

## 功能与适用证据

| 标准 | 已实现路径与适用证据 | 原生科学验收边界 |
|---|---|---|
| SC-001 | `stage_results::wannier_allocation_publishes_without_phonons_and_repairs_only_derivatives`：没有第 4／6 步文件，真实公开入口与生成 shell 执行后可查看两图及数据 | 科学程序为隔离替身；真实 EPW 输出待 T013 |
| SC-002 | 同一测试在脚本结束、再次调用公开命令之前核对 results；`direct_submit::stage_allocations_complete_upstream_once_then_both_native_solvers` 覆盖 3／6／7／Tc；gap 第二方法作业内发布另有断言 | 未提交真实作业 |
| SC-003 | 调用账目仅 EPW 内嵌能带与 postw90 DOS；`wannier_post_failure_keeps_model_and_recovery_never_calls_wannier90` 验证失败恢复不重拟合 | 已核对内嵌库源码；真实程序版本与 DOS 数值未验收 |
| SC-004 | `coarse_publishes_phonons_using_stage_five_electrons_without_reference_pw` 与 `matdyn_recovery_reuses_successful_force_constants_and_native_coarse` 核对 PW／PH、postw90、粗网格、q2r 调用不增加 | 使用替身调用计数；不冒充真实程序性能或科学精度结论 |
| SC-005 | 损坏内嵌坐标拒绝；非正交／非等长路径纯函数核对；现有 DOS 自旋、网格、ASR、来源回归；q2r／matdyn／插值执行前写具体原因 | 真实输入只读检查另列；未放行不相容来源 |
| SC-006 | 第 5 步两图及 CSV／参数／采样／窗口／差值；第 6 步声子两图和总览读取第 5 步 `band.eig`；元素聚合回归及图片检查 | 图片来自进程替身数据，不能用于判断材料或选窗科学合理性 |
| SC-007 | DOS／后处理／图像失败保护科学检查点；缺图修复不再 postw90；索引更新在文件发布前后均可接续；上游缺失重建前撤下旧下游回执 | 恢复边界由真实文件 I/O、shell 和独立进程验证，调度器为替身 |
| SC-008 | 完整 3～6 步包移动后在空 PATH 重绘，调用计数不变；既有旧工作区导入和来源哈希保护回归保留；旧电子采样仅从已知 schema 2 来源读取 | 未迁移真实旧工作区；无可核验旧采样时保留模型并报缺项 |

## 回归和可复查位置

使用仓库 native 构建脚本，不用直接 Cargo 绕过本机运行库配置：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow -p qeflow --no-fail-fast -- --test-threads=4
```

完整回归日志：`/tmp/dftflow-0050-final-tests.log`。首轮为 273 通过、2 失败、9 显式暂挂；其中日志测试仍沿用旧提交方式，另一失败暴露了科学输入变化后旧结果包未归档的问题。修正后按受影响范围定向复测，不把首轮失败算作通过。共享库已有编译警告不属于本需求新增。

`/tmp/dftflow-0050-targeted-tests.log`：最新源码的库测试 32 通过、1 暂挂；原生插值契约 13 通过、1 暂挂；Slurm 日志 6 通过。`/tmp/dftflow-0050-stage-verified-tests.log`：新增 6 项阶段契约和长链修复复测全部通过，合计 7 通过、0 失败。该组还验证更换电子窗口后相容 q2r／matdyn 的调用数不增加，旧电子采样迁移不更改科学回执。

全量检查与最终定向复测按测试名去重合计覆盖 **276 项通过**，默认 9 项暂挂另列；不是宣称修改后的源码曾一次全量命令返回零。中间阶段日志 `/tmp/dftflow-0050-stage-final-tests.log` 保存了新增回归中旧字段断言、测试输入未提供 guiding_centres 采用证据、已拷贝后处理不应要求新提交脚本这三项失败；对应测试已修正并在最终 7 项中通过。没有遗留已执行而未通过的用例。

真实现有 MgB₂ 配置只读检查：`/tmp/dftflow-0050-real-readonly.log`，1 通过。实际清单为 183 个路径点和 32 个网格点，采样 wdata 为 141 条；完整 WannierOnly 输入通过总容量 200 条、单条 256 字节检查。仅解析已有配置／XML 并构造输入字符串，没有启动任何科学程序或改写原工作区；这不能证明真实 EPW 已执行这些输入。

默认回归的 9 项显式暂挂如下；第一项本轮已单独执行通过，其余 8 项未执行。既有历史原生测试不是本需求新增的关闭条件，也不把它们的暂挂计作科学验收通过。

| 测试 | 默认不执行的原因／本轮处理 |
|---|---|
| `real_workspace_wdata_readonly` | 需要显式配置；本轮已提供现有 MgB₂ 配置并通过只读检查 |
| `actual_lda_coarse_restart_and_wannier` | 需要指定本机真实 LDA 粗网格证据 |
| `actual_lda_native_layout` | 需要指定已有 LDA 工作区 |
| `existing_real_qe_data_plot_without_scientific_execution` | 需要指定真实工作区及新的输出目录 |
| `actual_lda_fine_matrices` | 会执行真实细网格 EPC，需要独立预算和调度授权 |
| `actual_lda_fine_readback` | 需要指定已经结束的真实 EPC 证据 |
| `actual_lda_native_nonlinear_diagnostic` | 会执行固定非线性诊断，需要独立授权和预算 |
| `actual_lda_native_tc` | 会执行真实两模式 Tc，需要独立授权和预算 |
| `numbered_real_legacy_pw_is_read_only` | 需要显式指定本机旧 PW 来源配置 |

独立保留的公开进程证据：`/tmp/dftflow-0050-evidence/public-chain-18e0d99d47b7d9e3-2afe45-0/`，对应日志 `/tmp/dftflow-0050-visual-tests.log`。该测试在成功后保留 3／4／5／6 步图与数据，并完成完整包移动、空 PATH 重绘、科学调用不增加检查。已实际查看第 5 步能带及 DOS PNG、第 6 步四面板 PNG、第 4 步元素 DOS PNG；这些是替身数据的图形证据，不是 MgB₂ 科学结果。

最终阶段测试的全部工作区保留于 `/tmp/dftflow-0050-verified-evidence/`。窗口数值与图例更新后的第 5 步两图另在 `/tmp/dftflow-0050-final-evidence/public-chain-18e0db4e27530872-2f280b-7/run/results/5.epw-wannier/` 实际查看；该处对应已通过的无声子独立交付用例。

## 技术原因和故障修正

- 第 4 步 simple ASR 的力常数／频率不等于比较采用的 Γ onsite 加 Hermitian 规则。必要 q2r 与 matdyn 分成两个尝试；后者失败复用前者，不重做 PH。
- 内嵌 W90 按段长比例决定间隔数。wdata 以最短所需间隔校准并拆段，输出后按真实 `.kpt` 提取；六位小数无法分辨目标点时拒绝。原模型不覆盖变更后的新目标点时明确报缺项，内嵌库没有不拟合的独立 `restart=plot` 路线，不能偷偷重拟合或执行外部 W90。
- gap 原生提前结束即便退出零，只要请求温度不完整，最终作业仍非零；成功点保留。后续改坏已冻结原生日志会拒绝复用，不能追认其为新科学结果。
- 上游目录删除后重建，即便输出字节恰好相同，也不能让旧下游回执复活；阶段提交前先归档失效回执。
- 更换科学输入后，第 6 步旧结果包随阶段归档，避免新表与旧包冲突。已核验 q2r／matdyn 的独立派生副本按声子口径缓存，电子模型变化不会使相容后处理重算；缓存坏数据不会被静默补算。
- 单元文件发布与根导航更新之间增加事务记录；两处中断均能接续。根索引缺失时只从完整阶段包重建，不去科学目录重新采样。

## 文档和关闭边界

Issue、Spec、计划、研究、数据模型、CLI 契约、任务、需求检查表及快速验证同步本轮实施范围；用户原文围栏不改。按总览维护 skill 同步 `docs/overview/epwflow/` 的目录、流程图、比较目的与恢复说明，根 README、模块手册和 MgB₂ RESULTS.md 对应当前源码。总览入口现有导航仍有效，无需重复建立入口。

本地相对链接、围栏、原文一致性、编号、附件哈希和 Git whitespace 在提交前核验。只有被实际验证的任务才勾选。T013 的真实 EPW／postw90 输出、版本、调用账目和科学图片验收未执行，默认安装和真实工作区迁移未执行，线上 Issue 与远程双向链接未核验。以上限制不以源码回归代替，也不宣称线上 Issue 已关闭。


## 后续合并真实验收

用户授权 0046～0051 联合交付后，复用从 SCF 走通八阶段的独立 MgB₂ 现场；公开 coarse 作业 1334753 自动补交付全部 15 张实空间衰减图，没有重跑成功科学任务。18 份科学回执／原始衰减数据哈希不变，图表已按 PNG／SVG／PDF 实看；合并回归、移动重绘、最终安装和 examples 清理的完成状态统一见 [联合验收记录](../0050-epw-stage-results/real-chain-validation.md)。首轮未部署边界为历史记录，不代表后续状态。


联合收尾已完成：仓库及默认三程序入口与真实受测制品哈希一致；examples 旧现场 1394 项移出归档校验一致，输入保护不变，新目录公开检查通过；手动 README／RESULTS 与总览已同步。最终回归为 EPW 248＋QE 34＝282 项通过，默认暂挂 10 项中两项真实只读另行通过，其余历史来源／Tc 测试不计通过。全过程科学调用与阶段回执核验、预算及路径证据见联合验收；线上状态仍由用户管理。
