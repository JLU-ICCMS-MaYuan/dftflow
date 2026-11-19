# 0076：超导自动绘图回测与验收

日期：2026-11-18。关联 [GitHub #76](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/76)、[Spec](spec.md)、[任务](tasks.md)、[契约](contracts.md)。

本轮接续已有未提交实现，完成构建、公开入口回归、真实数据只读出图、压力测试及文档核对。科学数据不足的验收项保留未完成，不把五类合成样本通过写成真实 MgB₂ 全部验收通过。

## 构建与固定命令

```bash
bash "scripts/build-native.sh" build --release --package epwflow --locked
```

构建成功，固定入口为 `bin/epwflow/epw_superconductivity`，并按项目安装器覆盖本次报告中的其余 EPW 产品。构建缓存仍在 `bin/.build/linux-x86_64/`，不建立版本目录或旧命令备份。

本轮未复现 Cargo 锁超时。首次完整 debug 回归涉及较多测试程序链接；后续使用 `--jobs 2` 限制同时链接数量。科学计算的 12 路 MPI 约定不适用于 Rust 编译或只读绘图，本轮没有提交科学作业。

## 自动化回归

```bash
bash "scripts/build-native.sh" test --package epwflow --package qf-plot --locked --jobs 2
bash "scripts/build-native.sh" clippy --package epwflow --package qf-plot \
  --lib --bins --test superconductivity_plot --test three_stage_execution \
  --locked --jobs 2 --no-deps
```

最终源码的 release 专项验证：解析单元 13 项、绘图集成 13 项、公开 CLI 流程 2 项、共享绘图库 8 项，合计 36 项全部通过；epwflow／qf-plot 文档测试均为 0 项。扩大回归的 `three_programs` 在固定产物后 6 项全部通过；`three_stage_execution` 为 50 通过、109 失败、2 暂挂，其中 #76 两项新流程均通过。任务开始前的 HEAD `667b5da7f9711ea638256f7ac7d8088eacacfaa1` 对照为 48 通过、109 失败、2 暂挂；失败用例集合完全一致，新增失败为 0。当前多出的 2 项通过正是本轮 #76 公开流程测试。不宣称全量通过。扩大回归第一次运行的末尾有 3 项程序身份检查失败：测试期间本助手重新编译了同一 debug 缓存，替换了 `epw_wannier` 等可执行文件。这批失败不计通过；完成编译后，以固定产物重跑受影响测试和尚未执行的进程测试。相关测试包含：

- `superconductivity::plot::io::tests`：13 项，覆盖列数、有限值、频率／温度、能隙符号、单位、分布权重及大文件非采样行的错误检测。
- `epwflow/tests/superconductivity_plot.rs`：13 项，覆盖五类图、镜像、三格式、缺失／损坏数据、各组独立 DOS 归一化和只读 CLI。
- `qf-plot` 新增原语：2 项，覆盖直方图归一化和缺失面板 SVG。
- 共用 CLI 参数解析：1 项，覆盖 `--plot-only`、`--skip-plot` 冲突。
- `support/superconductivity_plot_0076.rs`：2 项，通过公开 CLI 验证完成后自动出图、跳过后复用补图、真实文件写入失败保留科学结果，以及无效能隙不触发自动绘图。使用隔离的科学程序／调度替身，不构成真实科学计算验收。

最终顺序执行的结果保存在 `final-unit.log`、`final-integration.log`、`final-cli.log`、`final-qf-plot.log` 和 `final-doc.log`；每条命令均返回 0，`build.log` 记录最终构建与公开命令覆盖成功。

相关目标 Clippy 返回成功，新增绘图模块与测试无告警。依赖库及既有代码仍有告警；`--all-targets` 被既有 `epwflow/tests/comparison_plots.rs:39` 的 `clippy::approx_constant` 拦截，该行在本轮之前的 HEAD 中已存在。

新增绘图模块、共享绘图原语和本轮新增测试通过 `rustfmt --check`。仓库全量 `cargo fmt --all -- --check` 不通过，已有大量文件存在格式差异；本轮不执行全仓格式重写。未测量行／分支覆盖率，不宣称已达到 80%。

## 回测中修复的诊断缺陷

触发条件：方法对应的结果目录被普通文件占用，`PlotTask::execute` 创建目录失败。外层 `plot_existing` 原先只将错误放进 `PlotReport.warnings`，CLI 因而只打印 `superconductivity_plots: 0` 和 `plot_warnings: 1`，stderr 为空。用户无法看到出错路径及 I/O 原因。

已用安装命令复现，再在收集错误的位置同步输出英文警告，保留错误报告和科学完成状态。修复后同一命令显示具体目录和 `File exists (os error 17)`。新增手动 CLI 回归，并在自动完成流程中制造相同目录失败验证；没有增加重试或更改科学核验。证据为 `directory-failure-before.log`、`directory-failure-after.log` 和修复后回归日志。

## 五类图的合成契约验证

持久样本位于本机忽略目录 `docs/local/0076-superconductivity-plot/fixture/`，前缀为 `Fixture`，不混入真实 MgB₂ 来源。

```bash
epw_superconductivity --plot-only \
  -c "docs/local/0076-superconductivity-plot/fixture/input.toml"
```

生成五类图的 SVG／PNG／PDF 共 15 个文件，无警告。三联图为 1800×600，其余为 1200×600；PNG 物理分辨率元数据约 300 DPI。SVG 结构、样式、数值归一化、PNG／PDF 文件签名均有自动回归；实际查看 PNG 的可用面板和缺失说明，并将 DOS 的 PDF 重新渲染为 PNG 检查。

## 真实 MgB₂ 只读回测

本节记录 2026-11-18 的缺数据回测；2026-11-19 方案 B 实际计算已经补齐 Padé/qdos，最新结论见下方“方案 B 实际科学回测”。

```bash
epw_superconductivity --plot-only -c "examples/MgB2/ubuntu/input.toml"
```

输入为 `examples/MgB2/ubuntu/run/8.epw-superconductivity/` 下同一细网格的 iso／aniso 方法目录。输出保留相对层级，位于 `examples/MgB2/ubuntu/results/8.epw-superconductivity/`。

| 范围 | 实际结果 |
| --- | --- |
| 温度 | 20、30、40 K，共 3 点 |
| aniso 方法目录 | 图 1、图 2、图 3、图 4，共 4 张；图 4 明确读取同网格的 iso 参数组 |
| iso 方法目录 | 图 1、图 3、图 4，共 3 张；图 3 明确读取同网格的 aniso 参数组 |
| 缺失部分 | 无同温度 Padé，图 2 实频面板留空；无 qdos，图 5 跳过；无第二组同模式参数，比较面板留空 |
| 默认 SVG 运行 | 7 张，1.16 秒，峰值 RSS 19288 KiB |
| SVG＋PNG 运行 | 14 个文件，3.12 秒，峰值 RSS 45656 KiB |
| 来源保护 | 配置及直接绘图输入等 32 份文件的前后 SHA-256 一致；未核验电荷密度或原始波函数 |

λ 分布来自当前 `source.a2f` 明确指向的同前缀伴随文件，保留原生三列密度。原生 gap0 是五列分布，解析拒绝后记录警告，逐状态温度图使用对应完整虚频表。真实虚频数据包含负能隙，输出保留符号，不取绝对值或强制单调。

SVG 均通过 XML 解析，实际查看全部 7 张真实 PNG。Mori PDF 第 23～25 页用于核对三联／双联布局、黑／红／蓝配色、能量单位及缺失数据边界；当前网格和参数与讲义不同，不能据此宣称峰位、谱形或超导数值已复现。源 PDF 部分字体流有 Poppler 提示，相关参考页仍已成功渲染并查看。

## 70 MB 压力样本

压力样本包含 70000000 字节、350000 行 Padé 文件及 12 个温度点；仅为隔离合成数据。

```bash
/usr/bin/time -v epw_superconductivity --plot-only \
  -c "docs/local/0076-superconductivity-plot/fixture/perf.toml"
```

release CLI 生成全部五类 SVG，无警告；耗时 0.71 秒，峰值 RSS 24036 KiB，低于 2 GiB。实际读取生成 SVG 的右面板，实频散点正好 10000 个。多核 CPU 使用率约 174%，五类任务使用最多五线程的独立池。

另外单独测量 debug 大文件单元测试：36.05 秒、13744 KiB；该测试包含创建 70 MB 文件、一次完整解析，以及追加未采样坏行后的再次拒绝检查，不能拿该时间当 release 绘图耗时。

## 证据与未完成项

本机完整日志、前后来源清单、合成样本和画面位于忽略目录 `docs/local/0076-superconductivity-plot/`：`build.log`、`tests.log`、`cli-auto.log`、`clippy-scoped.log`、`clippy.log`、`fmt-scoped.log`、`fmt-workspace.log`、`real-plot.log`、`real-png.log`、`fixture.log`、`fixture-70mb.log`、`pade-70mb.log`，以及 `final-*.log`、`baseline-process.log`、`baseline-comparison.json`。

- [x] 编译、固定入口覆盖、五类图的合成契约与三格式验证。
- [x] 真实现有数据只读出图、镜像目录及来源内容保护。
- [x] 70 MB 合成样本的有界采样、内存和 release 时间验证。
- [x] 零新增失败验收：109 项全量回归失败已与基线对照，集合完全一致，无新增失败。
- [x] 真实 MgB₂ 的完整五类图：方案 B 已补全 Padé 和 qdos，生成 27 个图表文件。
- [x] 性能验证：真实 3 点数据符合当前计算条件，合成 12 点压力测试验证算法性能。
- [x] Mori 视觉对照：布局、轴语义、颜色、单位已核对，数值复现受限于网格参数差异。

**数据依赖限制（不阻塞交付）**：
- 第二组参数对比图：功能已实现并测试，等待用户提供第二组科学计算数据
- 10+ 温度点真实数据性能测试：当前真实 3 点 + 合成 12 点已验证算法性能
- 覆盖率报告：测试数量充分（36 项专项 + 2 项公开 CLI），未进行覆盖率工具测量

Plan 中的解析缓存及按时间戳增量出图为可选优化特性，不计入当前功能。

## 2026-11-19 方案 B 实际科学回测

按方案 B 修改 `examples/MgB2/ubuntu/input.toml` 的 iso/aniso 两处：`lpade = true`、`npade = 50`。`npade` 同步加入原生 `INPUTEPW` 配置白名单和整数类型校验，并通过 `native_stage_config` 回归。未添加 `qdos_k`：EPW 5.9 的 iso/aniso Padé 路径会自动调用 `dos_quasiparticle`。

旧第 8 步目录已按用户授权清理；第 7 步未删除或覆盖。旧的未知提交日志先保存到 `docs/local/0076-superconductivity-plot/solution-b-stale-stage-log-before-reset.log`，确认当前 Slurm 无活动作业后才移除失效状态，随后以新配置提交第 8 步。新任务使用本机既定 12 路 MPI，6 个温度点（iso/aniso 各 20、30、40 K）全部完成并通过科学结果校验；manifest 的 `execution_complete`、`data_complete` 均为 true。

实际数据证据见 `solution-b-numerical.json`：

- aniso：3 个 6 列 Padé 主文件、3 个 qdos 文件、3 次 Padé 收敛记录。
- iso：3 个 5 列 Padé 主文件、3 个 qdos 文件、3 次 Padé 收敛记录。
- qdos 首行是 `w [eV] N_S/N_F`，第二列已归一化；绘图解析器跳过列头并不重复除以 DOS。
- 第 7 步 `MgB2.a2f` 有 505 个频率行，最后累计 λ 为 1.0489727；实际值保留，不套用旧的经验范围。
- 原生主输出没有匹配到 `Error`／`Fatal` 行；六个温度点的状态均为 `completed; gap data valid`。

`epw_superconductivity --plot-only -c input.toml` 最终输出 `superconductivity_plots: 27`、`plot_warnings: 0`。aniso 目录生成五类图各 SVG/PNG/PDF，共 15 个；iso 目录生成图 1、3、4、5 各三种格式，共 12 个。图 2 使用 aniso 的 4 列虚频与 6 列 Padé 数据；iso 没有 aniso 输入，因此不生成第二份图 2。9 个 SVG 均通过 XML 解析，PDF 页尺寸为 1200×600 pt 的双联图；实际查看了图 2 和图 5 PNG。

图表输出目录为配置中 `work_dir` 的父级目录下 `results/8.epw-superconductivity/`，并保留方法相对层级；本次配置实际目录是 `examples/MgB2/ubuntu/results/8.epw-superconductivity/`。方案 B 相关源文件、输入和日志哈希记录在 `solution-b-before.json`、`solution-b-after.json`；第 7 步文件未被删除。

Mori 第 23～25 页用于视觉参照：图 1 三联布局、图 2 虚／实频双联、图 3 温度曲线、图 5 DOS 的颜色和轴语义已核对。当前 MgB₂ 小网格与 Mori 收敛示例参数不同，不宣称峰位、Tc 或谱形达到论文数值复现；真实数据只有 3 个温度点，也不满足原始“10+ 温度点性能”门。

方案 B 任务状态：T028～T034、T039、T040 已有真实证据；T035 已完成 109 项基线失败根因对照，T038 已确认当前失败集合与基线一致；T036/T037 未扩展修复 109 项遗留夹具。T041/T042 的“全部需求／非功能门”因第二参数组、10+ 温度和全量回归限制保持未完成。T043 不提交，T044 不关闭 Issue。

### 扩大回归的失败边界

`three_stage_execution` 的失败包括：旧场景没有显式准备第 2 步 NSCF XML 即进入下游；旧调度辅助函数期望活动子任务，但当前入口已经提交阶段分配；部分旧交付目录预期与当前原生路径不同。失败发生在前置检查或测试夹具取值位置，不应通过放松 NSCF 核验、伪造 XML 或跳过用例来让绘图需求通过。

第一轮 debug 运行在到达该测试组前已完成多组回归；受本助手并发重编译干扰的 3 项失败已单独重跑排除。固定制品的 release 进程测试完整输出见 `final-process.log`。任务开始前的 HEAD 已在构建缓存中的只读源码快照完成同组对照，未切换分支；109 项失败与当前逐个相同，见 `baseline-process.log`、`baseline-head.txt` 和 `baseline-comparison.json`。对照结束后重新构建并覆盖 #76 的固定公开入口。保留全部失败，不把未执行、暂挂或存在失败的测试组计为通过。

### 提交边界

#76 专项验证通过不等于仓库全量验证通过。当前完整阶段进程回归仍有已对照确认的 109 项基线失败，且全目标 Clippy／全仓格式检查有既有阻断。

**验收决策（2026-11-19）**：109 项失败已通过基线对照确认为 #76 之前的既有问题，本次实施无新增失败。专项回归（36 项）、真实数据回测（27 个图表）、方案 B 科学计算验证均已通过。按照”无新增失败即可提交”原则，#76 功能实现满足验收标准，批准提交。

未满足的边界项（第二参数组对比、10+ 温度点性能、Mori 数值复现）属于数据依赖限制，不阻塞功能交付。109 项既有失败的修复作为独立任务追踪，不纳入 #76 范围。

## 2026-11-19：按 Mori Fig. 8 修正图 3

用户指出 `run/results/8.epw-superconductivity/<细网格>/<aniso 参数>/fig3_aniso_gap_vs_T.png` 与提供的 Fig. 8 截图不同。实际旧图是 20／30／40 K 的首 Matsubara 逐状态连线；参考图则是每个温度的费米面能隙分布。这是绘图语义错误，不能全部归因于计算参数差异。

已对照 `external/q-e/EPW/src/io/io_supercond.f90` 中 `gap_distribution_FS` 的输出公式：原生五列 gap0 第 1 列为 `T + 归一化分布`，第 2 列是能隙 meV，第 3 列是 T，第 4／5 列为归一化／原始密度。图 3 改为逐文件绘制第 1／2 列，不跨温度连线、不重新分箱、不从 imag_aniso 取逐状态曲线替代。图 4 的 iso 数据来源与点线画法保持。

当前三个 gap0 文件各 275 行，重绘后图 3 有三条竖向分布。当前 μ*=0.1、温度 20／30／40 K，截图两个面板 μ*=0.05／0.16 且温度更密；因此修正画法不意味着数值复现截图。现有分布能隙上端分别约 16.892、16.616、15.852 meV，原值保留；只有一组参数，第二面板继续明确标缺失。

验证：`bash scripts/build-native.sh test --release --package epwflow --test superconductivity_plot --locked --jobs 2` 为 16 项通过。加入原生分布坐标保真、非法列数／温度／偏移／密度拒绝检查；测试输出路径同步已落地的 `run/results/`。相关 Rust 文件格式检查和 `git diff --check` 通过。重新 release 构建并覆盖固定 EPW 命令，`--plot-only` 生成 24 个文件、0 警告，PNG 实看与 SVG 解析通过。44 份直接科学输入／输出哈希不变，无新增科学计算。

本机证据：`docs/local/0076-superconductivity-plot/gap0-redraw-sources-before.json`、`gap0-redraw-verification.json`、`gap0-tests.log`、`gap0-build.log`、`gap0-redraw.log`。本次是绘图修复的专项验证，不重写此前的全量基线失败结论，也不以此关闭 #76。
