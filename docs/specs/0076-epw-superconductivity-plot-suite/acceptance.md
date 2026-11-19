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
- [ ] 全量回归通过：当前阶段进程组仍有 109 项失败，保留未通过。
- [ ] 真实 MgB₂ 的完整五类图：还缺 Padé、qdos 和第二组参数数据。
- [ ] 真实 10 个以上温度点的数据集性能；当前只验证真实 3 点及合成 12 点。
- [ ] Mori 全部五类图的真实数值／形状对照和覆盖率报告。
- [ ] Issue／Spec 双向远程链接及线上 Documentation Impact 回写：本轮没有推送或修改线上 Issue。

这些缺口保留未完成，本轮不新增科学计算来补齐、不关闭 #76。Plan 中的解析缓存及按时间戳增量出图尚未实现，不计入当前功能。

### 扩大回归的失败边界

`three_stage_execution` 的失败包括：旧场景没有显式准备第 2 步 NSCF XML 即进入下游；旧调度辅助函数期望活动子任务，但当前入口已经提交阶段分配；部分旧交付目录预期与当前原生路径不同。失败发生在前置检查或测试夹具取值位置，不应通过放松 NSCF 核验、伪造 XML 或跳过用例来让绘图需求通过。

第一轮 debug 运行在到达该测试组前已完成多组回归；受本助手并发重编译干扰的 3 项失败已单独重跑排除。固定制品的 release 进程测试完整输出见 `final-process.log`。任务开始前的 HEAD 已在构建缓存中的只读源码快照完成同组对照，未切换分支；109 项失败与当前逐个相同，见 `baseline-process.log`、`baseline-head.txt` 和 `baseline-comparison.json`。对照结束后重新构建并覆盖 #76 的固定公开入口。保留全部失败，不把未执行、暂挂或存在失败的测试组计为通过。

### 提交边界

#76 专项验证通过不等于仓库全量验证通过。当前完整阶段进程回归仍有已对照确认的 109 项基线失败，且全目标 Clippy／全仓格式检查有既有阻断。依照 `AGENTS.md` 的“若验证失败……不得提交”要求，本轮保留修改及完整证据，不自动提交、不推送，也不关闭 #76。
