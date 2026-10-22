# 技术评估与设计依据

日期：2026-10-22。性质：只读源码核对与待审阅设计，不代表新输入、原生计算或功能验收已通过。需求基线见 [Spec](spec.md)。

## 独立评估：不重复计算合理，数据来源需分清

用户要求避免重复计算是合理的：完整相容的 QE 参考、力常数、频率、插值样本应复用。第 1～4 步只能提供 QE 侧参考；第 5 步产生的电子模型、第 6 步产生的粗网格电子声子数据，不可能在尚未执行它们的第 1～4 步内预先存在。首次用模型求指定位置的数值属于必要计算，不能承诺全部对比数据只读取第 1～4 步。

固定决策顺序：

1. 核验来源身份与比较口径，直接读已有目标值。
2. 能通过拷贝、真实坐标表示转换、单位换算和表格整理得到时，仅做派生处理。
3. 确实缺必需采样时，事前保存缺项、不能复用的具体原因、最少原生程序及目标输出，再执行后处理或插值求值。

不能把来源损坏解释成“正常缺项”后无说明地重算；不能通过新增 SCF／PH 绕过不同晶体、模式、ASR 或网格身份冲突。第 5 步电子比较不允许因比较而补做 PW；有效 QE 参考缺失时先报告上游缺项。新增工作区原有第 2／3 步计算职责不因此取消。

| 现有数据情况 | 处理设计 | 不允许的替代 |
|---|---|---|
| 第 3 步路径能量及第 2 步完整均匀网格能量有效 | 直接读取电子 QE 参考 | 第 6 步另做同一份 PW 参考 |
| 已有相容力常数 | 直接读取／拷贝 | 再运行 q2r |
| 已有同点、同 ASR 的声子频率 | 直接读取／变换单位 | 再运行 matdyn |
| 力常数有效，但缺目标 q 的频率 | 事前说明点位缺项，仅最少 matdyn 求值 | 重做 PH |
| 规则不同但原始响应有效且存在获授权的修正方案 | 明确规则差异，生成隔离派生副本及必要后处理 | 将不相容结果标作复用成功 |
| 已有电子／声子插值样本相容 | 直接读，不再调用插值程序 | 每次重画都运行 EPW |
| 模型有效而缺目标位置插值样本 | 记录原因，最少插值求值；不重新拟合 | 重跑整个 Wannier／粗网格链 |

## 讲义明确支持什么

[归档讲义](assets/Tue.6.Lafuente.pdf)原件：`test_auto/MgB2/epwflow/Tue.6.Lafuente.pdf`。25 页，670142 字节，SHA-256：`519a13414a6b27486c0f10bfc9acf6ce6aebdda6c85c31f1b0c83f7a262c0fc5`。原字节保留；本文是解释，不替代 PDF。

- 第 6～7 页：在 `epw.in` 用 `wdata` 写 `bands_plot = true`、`begin kpoint_path`、路径段、`end kpoint_path` 和 `bands_num_points`；内嵌 Wannier90 输出 `*_band.dat/.gnu/.kpt`。
- 第 10～11 页：介绍读取已有粗网格后的 EPW `band_plot`、`filkf/filqf` 路径插值。这是后续粗网格插值路线，不能与第 5 步 `wdata` 内嵌 Wannier90 输出混为一谈。
- 没有找到由 `epw.in` 独立输出电子 DOS 曲线的教程步骤。不能仅凭未找到就断言所有 EPW 版本都无此能力；本设计的限制进一步核对了下面的本地源码。

示意配置只说明参数关系，不能直接作为待运行的完整输入：

```fortran
wdata(1) = 'bands_plot = true'
wdata(2) = 'begin kpoint_path'
wdata(3) = 'G 0.0 0.0 0.0 M 0.5 0.0 0.0'
wdata(4) = 'end kpoint_path'
wdata(5) = 'bands_num_points = 10'
```

实际输入需同时保留拟合窗口、收敛参数及完整晶体路径；字母标签不代替坐标核验。

## 本地科学源码证据

核对树为 `/data/home/mayuan/soft/qe-7.4.1-amd-oneAPI2024`；这是本地源码能力证据，不证明当前 PATH 中的可执行文件就是该构建，也不代表生成的输入已经原生测试。

| 文件与位置 | 观察 | 对设计的约束 |
|---|---|---|
| `EPW/src/epw.f90:182` | 调用 `wann_run`，电子声子后续受 elph 分支控制 | 第 5 步可以只拟合电子模型，不依赖 PH／粗网格 |
| `EPW/src/wannierization.f90` 的 `write_winfil`／`pw2wan90epw` | 写入 wdata 后调用内嵌 Wannier90 | 能带参数进入该路线，取消外部 W90 绘图 |
| `EPW/src/input.f90:34` | `CHARACTER(LEN=256) :: wdata(200)` | 至少对本版本检查总条数和单行长度；包含已有窗口参数，不能溢出或截断 |
| `external/wannier90/src/wannier_lib.F90:391` | 调用 `plot_main`，未调用 postw90 DOS 路径 | 不能假设向 wdata 写 dos=true 就能获得电子 DOS |
| `external/wannier90/src/plot.F90:210–216` | 首段取 bands_num_points，后续段按相对长度 nint 取整 | 不是所有段固定同样采样数；不能直接套用当前 QE 展开规则 |
| `external/wannier90/src/postw90/postw90.F90` | 读取 eig、chk 等后调用 DOS 模块 | 允许用已有模型及已有 NSCF 能量补足输入，仅执行 DOS 后处理 |
| `EPW/src/use_wannier.f90:1343–1344` | write_dos 位于非 band_plot 且 eliashberg 的后粗网格分支 | 不能为第 5 步电子 DOS 提前执行粗网格或超导计算 |

采用仅 DOS 允许 postw90 的已确认方案：复用第 5 步 `.chk` 等模型，按现有适配从第 2 步已有 XML 派生必要 `.eig`；完整输入取决于实际版本及模式，未来需真实验证。不得以准备输入为由再独立运行 Wannier90 拟合或 restart=plot。

## 当前仓库实现与需要改变的职责

| 当前证据 | 现状 | 待设计变更 |
|---|---|---|
| [layout.rs](../../../epwflow/src/foundation/layout.rs) | results 指向 6.epw-coarse/results，comparison-native 统一落第 6 步 | 分阶段职责与工作区根 results；保留旧布局版本化读取 |
| [workspace_science.rs](../../../epwflow/src/foundation/workspace_science.rs) 的 run_wannier／electronic_comparison | 第 5 步已有拟合与电子图路径，但 native-post 另执行 Wannier90 绘图，再执行 postw90 | 拟合时配置 wdata；现有模型、来源与图形适配尽量复用 |
| [workspace_plot.rs](../../../epwflow/src/foundation/workspace_plot.rs) | 保留真实能窗、两侧图及 DOS 适配；能带当前存在按 xmax 比例缩放横轴 | 改为坐标／分段核验后的真实路径配对，不能保留全局比例对齐 |
| [automatic_comparison.rs](../../../epwflow/src/interpolation/automatic_comparison.rs) | 自动比较构造路径＋NSCF 网格，另生成电子 QE 参考，并用 EPW 生成电子和声子样本 | 消除重复电子参考计算，第 6 步只保留声子求值，电子图／数据来自第 5 步 |
| [epw_config.rs](../../../epwflow/src/interpolation/epw_config.rs) | WannierOnly 的 wdata 目前主要设置窗口与收敛 | 在同次 EPW 拟合中加入路径及绘图参数 |
| [native_workspace.rs](../../../epwflow/src/foundation/native_workspace.rs) | 逐步执行并登记，整体循环后 refresh_post | 对应子任务和阶段收尾即发布，不能等更高目标循环结束 |
| [workspace.rs](../../../epwflow/src/foundation/workspace.rs) 与 [native_jobs.rs](../../../epwflow/src/foundation/native_jobs.rs) | 0048 已有分配内执行、父尝试与检查点 | 复用作业内收尾机制，避免额外 sbatch 或重复预算预留 |
| [workspace_qe_plot.rs](../../../epwflow/src/foundation/workspace_qe_plot.rs) | 0047 已有单侧图、派生缓存及来源核验 | 将成功图及其数据、图参纳入及时发布，不改科学检查点 |
| [delivery](../../../epwflow/src/delivery/mod.rs) | 已有原生／比较包、独立重绘、格式保留及发布恢复 | 为分阶段包调整接口，根索引仅导航，沿用已验证机制 |

## 路径和 DOS 的比较口径

当前 QE 路径按每段固定数量展开，而所核对的 Wannier90 按段长比例取点。仅指定同名高对称路径不证明逐点一致。设计先用第 3 步实际 k 点作为电子参考位置，使用 wdata 中的路径段和采样参数使内嵌输出落在相同位置，生成前预检、输出后以 `.kpt` 等真实坐标复核。

若内嵌路径接口无法精确表达已有采样，不允许缩放横轴、把画线用的插值值当原生采样误差，也不为此重跑 PW。必须明确报告不相容点位，并在已允许的 wdata 路线内设计可验证的路径拆分或列出受限覆盖；未解决时不得宣称完整对比完成。路径拆分、端点去重、断段和容量检查是实施验证重点，当前只读核对不保证任意长路径已可表达。

DOS 必须基于双方真实一致的均匀网格坐标、权重和偏移，使用一致能量零点、展宽和自旋计数。第 2 步完整 NSCF 网格为电子比较的参考网格。已有第 3 步 DOS 若积分网格或展宽不一致，可保留为单侧图，但比较参考应从已有 NSCF 能量重累计，不额外计算 PW。postw90 的积分／展宽设置须和该比较表一致；不能只把默认网格设成同样大小就宣称一致。

声子参考应优先读当前第 4 步的力常数／频率。当前自动比较使用 Γ onsite、Hermitian 等修正，再 q2r 的 zasr=no 和 matdyn 的 asr=no；第 4 步若采用不同规则不能直接共用频率。核对参数与已有数据后再选复用或隔离派生后处理，不能偷偷更改一侧 ASR 使图看起来重叠。

均匀网格 DOS 比较的目的，是从整个倒空间的态分布补充检查插值吻合程度；路径对比与 DOS 对比互补。既不将路径点混进 DOS 统计，也不把图像一致自动解释为粗网格收敛。历史示例的 183＋32 不是通用固定参数。

## 发布与恢复的设计取舍

复用现有交付格式、校验与 journal，不建立另一套工作流状态。科学完成证据可先持久化以保护检查点，但“阶段最终交付成功”必须晚于该阶段应有图表及发布核验。图片失败不能通过使科学来源身份失效而迫使重算。

每个成功电子子任务先发布自己的数据；根索引逐项增量引用已完整的交付，不能要求同阶段另一子任务先完成。每个阶段条目携带来源身份，不混用上次请求、另一模式或另一拟合窗口的结果。

目录迁移是派生数据职责调整。旧科学文件、冻结脚本、回执和历史结果原样保留；旧版本读取只用于导出经核验的派生结果。不能仅改 layout 然后因找不到旧数据就发新计算。

## 备选方案与未采用原因

- 全部用 EPW 后粗网格 DOS：会延迟第 5 步交付，且偏离已确认的仅 DOS 使用 postw90。
- 外部 Wannier90 restart=plot：技术上已有路径，但用户已明确取消独立绘图执行。
- 所有比较结果都限定来自第 1～4 步：无法提供尚未产生的 EPW 模型与采样。
- 始终重新执行比较计算：浪费已有结果，违反复用优先要求。
- 先只改目录、最后统一 collect：无法保证子任务即时可见和作业内收尾。

## 验证边界

上述判断来自已读取 PDF 和源码，不是性能、科学收敛或原生输入验收。未来必须核验实际 executable 身份、内嵌 Wannier90 版本、wdata 路径输出及 postw90 DOS 输入；本轮无真实计算授权，不执行这些程序。任务和证据规划见 [tasks.md](tasks.md)、[acceptance.md](acceptance.md)。
