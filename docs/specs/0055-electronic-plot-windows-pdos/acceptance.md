# 验收记录

> 旧章节记录历史版本。当前生成端改为共享纵轴组合图，最新验收见文末“2026-11-16：统一组合图”；旧八图及显示范围不再是当前规则。

日期：2026-10-26。本地 0055；线上 Gitee 仍由用户管理。依据用户“开始落地。”及随后红蓝保留色补充实施，不重新请求已有授权。

## 已交付行为

- 第 3 步 4 张 QE 图，第 5 步 4 张 QE／EPW 图，全部提供近费米／完整价带范围；DOS 与能带同窗。
- 全部电子图含红色费米线；蓝色只表示 EPW／Wannier 插值。QE 深灰，窗口和元素用其他颜色，普通文字黑色。比较总览电子面板及历史包重绘同样适用。
- PNG 从数值矢量重新生成 3000×2100，密度 11812 px/m，即 300.0248 DPI。内置有许可的 DejaVu Sans，计算节点不依赖宿主字体。
- 第 3 步补真实 QE 元素 PDOS；第 5 步仍只比较 TDOS。不修改选窗、已有科学比较窗或自动评级规则。
- 共用主题／渲染器，沿用原阶段收尾和根 results 发布；只补必要原子投影，不建立第二套科学执行链。完整 CSV 与 plot.json 支持移动重绘。

## 真实 PBE MgB₂

来源为 `test_auto/MgB2/local_run/012-stage-results/`，只读复制到 `014-electronic-plots/validation-v2/`；脚本在 `test_auto/MgB2/epwflow/`，测试制品在仓库 `bin/.build/`。源 NSCF 共 69 个文件、另 7 个直接参考产物的文件集合／SHA-256 前后相同。原 dos.x 表保留。

| 项目 | 实际值 |
| --- | --- |
| 共同 E_F | 9.16990224089968 eV |
| 近费米范围 | −5～+5 eV，相对 E_F |
| 完整价带范围 | −75.29716629237811～+5 eV，相对 E_F |
| PNG | 8 张，3000×2100，300.0248 DPI |
| 元素 | Mg、B；逐 atom/wfc LDOS 各计一次 |
| 投影网格 | 9315 点；TDOS 来自同次 projwfc 的 DOS 列 |
| 最大 TDOS−投影和绝对残差 | 3.8668414 states/eV/cell；原样保存，未归一化 |
| 新增成功科学后处理 | projwfc 1 次；另有一次失败 projwfc 尝试 |
| 新增 PW／PH／Wannier／EPW／postw90 | 全部 0 次 |

投影和不等于 TDOS 并非自动认定实现错误，也不能靠缩放抹平；保留逐点残差供人工判断。不以八图交付证明选窗或粗网格收敛。

### 作业与恢复

| Slurm 作业 | 节点秒 | 结果及处理 |
| --- | ---: | --- |
| 1370547 | 6 | 验收夹具缺编号布局；未启动科学程序，补初始化 |
| 1370559 | 6 | 验收夹具缺当前配置快照；未启动科学程序，补快照 |
| 1370598 | 6 | projwfc 小 FFT 的 MPI 分布失败；按原生提示使用 `-pd .true.` |
| 1370704 | 15 | 原生 projwfc 成功并保存检查点；随后解析发现与旧 dos.x 网格不同，拒绝混用 |
| 1370760 | 17 | 成功复用原生检查点；发现三位有效数字的舍入被过严核验误判，修正按打印精度核验 |
| 1370782 | 9 | 完成 1 项真实验收；日志明确复用原生检查点及完整投影，没有新增原生程序调用 |

本功能合计 59 节点秒，原共享账本累计 5482／28800 节点秒，预留和活动分配为零。开始时结算的旧作业 1370507／1370508 共 31 节点秒另属历史，不计入这 59 秒。未重置预算。

`projwfc` 的必要性、来源、程序及输入先记录在 `pdos/decision.json`；成功原生检查点先于解析完成收据保存。`verification.json` 的 projwfc=1 表示该现场累计成功调用，不表示最后恢复作业新调用了一次；失败尝试单独按本表记录。旧科学数据未因后处理问题重跑。

### 图像与移动验证

`verify_electronic_0055.py` 实际检查八图、相同双窗、DPI、红／蓝像素和 Mg／B 投影。人工查看全部八图及代表性原尺寸图，核对文字、费米线、连续 DOS、元素曲线、实际窗口及插值覆盖缺失。很短的人工路径间隙会造成 A／L 等标签碰撞，绘图层合并端点文字，保留真实曲线横坐标、断段及科学表。

完整 `results` 复制到 `014-electronic-plots/portable-results-v8/`，用公开 `epw_wannier --plot --results ...` 在 `PATH=/nonexistent` 下重绘，再 `--results-check`。13 份 CSV／plot.json 哈希不变；不读取原波函数或执行科学程序。包内完整性通过不冒充重新核验不可访问的原始源。

## 自动化回归

构建使用 `CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh test --release --locked ... --no-run`，制品构建完后冻结，执行期没有覆盖程序。覆盖：

- 两万点窄峰逐像素连续性、历史红蓝窗口修正、300 DPI、嵌入字体。
- LDOS 汇总、防重复、缺原子、能轴错位、打印舍入及不相容投影拒绝。
- PDOS 失败保留 TDOS／能带；删除派生收据后复用原生检查点；不重复成功 PW／DOS／PH／模型。
- 第 3／5 步自动发布、损坏拒绝、缺图修复、相同公开入口幂等、声子链及第 6 步复用。
- 独立比较图、电子总览费米线与窗口颜色、Slurm 日志和恢复。

中间版本如实保留：v6 入口回归 77 通过、1 失败、1 忽略（旧逐输入目录脚本断言不适用于父电子作业内的投影）；v7 24 通过、54 失败、1 忽略（测试替身没有遵守 filpdos，文件名不同于真实 QE）。均已修正对应夹具／断言，没有放宽科学来源核验。最终通过计数和制品更新见本记录末尾。

## 标准对照与边界

SC-001／006：八图、两阶段、真实窗口和数据；SC-002：像素与 PNG 密度；SC-003：尖峰连续性与真实图实看；SC-004：真实 Mg／B 投影、同网格及不归一残差；SC-005：零新增主要科学计算、原生检查点恢复；SC-007：移动包重绘；SC-008：总览与手册同步；SC-009：费米线及红蓝专用。

这次是真实已有数据＋最小投影后处理的验收，不是再次从 SCF 到超导重跑；0046～0051 的历史全链证据仍在原验收记录。未搜索 Tc、扫描网格或重新评判选窗。当前元素投影只支持非磁标量；没有增加 EPW 元素／轨道 PDOS。

原始用户截图仅聊天预览，未取得原始文件／哈希；线上 Issue 标签／状态未操作。这两项外部核验保持未完成，不能用本地实现冒充完成。

补充：在 bubblewrap 空文件系统中仅映射系统运行库、程序和结果副本，原 `/data` 完全不可见，空 PATH 重绘与 `--results-check` 仍通过，满足 SC-007 的来源不可访问条件。13 份数值／图参哈希仍不变。原目录没有改权限、删除或重命名。图例移至绘图区右侧独立留白，避免白色图例框截断 DOS 峰或费米线。

更早一次比较图测试曾出现总线错误；后续固定内嵌字体、避免宿主字体 mmap 后，v6／v7／v9 串行比较图均 9 项通过。原总线错误根因未被证明，不把该中断运行计为通过。

默认保留的 3 项外部只读忽略项是：`real_bound_native_decay_files_render_without_scientific_calls`（需真实 coarse 来源）、`real_workspace_wdata_readonly`（需指定独立工作区）及 `numbered_real_legacy_pw_is_read_only`（需指定旧工作区）。它们均不是失败被改成忽略；新增真实八图用例单独显式运行通过。完整 epwflow 历史全部测试及再次从 SCF 开始的科学链不属于本次复跑证据。

## 最终验证结果

| 验证 | 通过 | 忽略 | 证据 |
| --- | ---: | ---: | --- |
| EPW 单元测试，含双窗／配色／尖峰／投影 | 39 | 2 | `docs/local/0055-unit-v12.log` |
| 完整公开入口及作业内阶段链回归 | 78 | 1 | `docs/local/0055-three-stage-v8.log` |
| 独立比较图和总览 | 9 | 0 | `docs/local/0055-comparison-v12.log` |
| 共享绘图／内置字体 | 4 | 0 | `docs/local/0055-shared-v7.log` |
| Slurm 日志 | 6 | 0 | `docs/local/0055-slurm-v7.log` |
| 显式真实 MgB₂ 八图后处理 | 1 | 0 | 1370782；`verification.json` |

隔离回归合计 136 项通过、3 项忽略；真实验收另 1 项通过。Wannier90flow 库无单元测试，已参与编译，不能把 0 项写成科学验收。完整入口回归通过后仅调整标签／图例布局，随后重跑全部 EPW 单元测试、9 项比较图回归，并用真实结果重绘核验；没有重新计入或重复运行成功科学后处理。

最终实际图片记录为 `docs/local/0055-real-images-v11.json`；隔离重绘日志为 `0055-isolated-redraw-v11.log`，最终包核验为 `0055-final-portable-check.log`。全部八图逐一核验范围／像素／密度／颜色，代表性原图和标签裁剪已实看。13 份完整数值及图参文件在最后图例调整后仍保持哈希不变。


交付构建执行 `CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh build --release --locked -p epwflow -p wannier90flow --bins` 成功，并按 0017 自动发布到仓库 `bin/epwflow/` 三程序和 `bin/wannierflow/wannier90`。构建及兼容打包记录为 `docs/local/0055-delivery-build.log`。测试／真实执行全部结束后才更新公开程序，没有边测试边替换制品。加载 `scripts/env.sh` 后解析到上述仓库路径；公开三入口帮助和独立 Wannier90 入口检查通过。

总览、模块手册、MgB₂ README／RESULTS 与测试说明均已同步。根 README 的 Overview 导航已覆盖本次变化，无需增加重复入口。保留原有 AGENTS.md 未提交修改，不混入本功能提交；没有推送或操作线上 Issue，也没有再次清理用户当前 examples 运行数据。


最后删除了已被外置图例替代的重复图例回调，统一由共用函数绘制图例。v12 的 39 项单元测试和 9 项比较测试再次通过；重绘后的 8 张 PNG **逐字节哈希不变**，13 份数值及图参哈希也不变。最终程序重新按相同构建命令发布，记录见 `docs/local/0055-final-delivery-build.log`。公开制品的空 PATH 重绘／核验及八图检查记录为 `0055-delivered-*`；布局调整没有改变数据或科学调用。

## 重合曲线可见性补充验收

用户后续明确要求长短虚线及下层约加粗 30%，覆盖先前的 QE 实线约定。共享样式采用下层 18／6、宽 4，上层 6／6、宽 3（实际粗 33.3%）；独立电子图、既有四面板总览和独立 Wannier 对比同步。第 3 步 TDOS／元素图使用加粗长虚线总量和不同短虚线元素曲线；单侧 QE 能带仍为实线。图例同步，红色费米线和蓝色插值保留。

SC-010 的生产渲染回归输入完全相同的能带、DOS，以及三条 TDOS／B／Mg 曲线，检查绘图区同一行的全部颜色像素，排除图例和不重合部分。密集 20001 点窄峰仍通过单曲线连续性检查，不把设计内的虚线间隔当成渲染碎片。

最终独立源码快照基于 `f91419b` 加本次五份源码／测试改动，包含已提交的 0056，不携带工作区未提交的 0057 等改动。EPW 单元 45 通过、2 个既有真实来源用例忽略，共享绘图 4 通过，公开比较入口／总览 9 通过；合计 58 通过、2 忽略，不将忽略或历史全链当作此次通过。日志：`docs/local/0055-overlap-clean-unit-final.log`、`docs/local/0055-overlap-clean-comparison-final.log`，源码边界见 `docs/local/0055-overlap-source.json`。

真实已有 PBE MgB₂ 包在空科学程序 PATH 下重绘到 `test_auto/MgB2/local_run/014-electronic-plots/overlap-style-optegcsq/results`，八张均为 3000×2100／300.0248 DPI。31 份原结果文件及 13 份 CSV／plot.json 的集合与 SHA-256 不变；六张多曲线图片换线型，两张 QE 单侧能带不变，结果包完整性检查通过。查看三图预览确认长短虚线、元素颜色及红色费米线；证据为同目录 `verification.json`、`figures.json`、`check.json`、`preview.png`。新科学调用／Slurm 作业均为零，预算不增加。

四个产品 `epw_phonon`、`epw_wannier`、`epw_superconductivity`、`wannier90` 已由独立快照重新构建，按明确产品名单安装至仓库 `bin/`；默认 PATH 的 `epw_wannier` 已实际重绘并通过包核验。安装后逐产品与已验证制品 SHA-256 一致，来源和哈希见 `docs/local/0055-overlap-deployment.json`；构建／安装日志为 `0055-overlap-build.log`／`0055-overlap-install.log`。独立 `wannier90 plot` 另用完全相同的两份能带输入核验曲线本体同时可见深灰和蓝色（`standalone-wannier.json`）。未改动其他任务的源码或提交其工作树变更。

## 电子能带 QE 实线纠正验收

用户指出能带 QE 参考必须为实线，此次纠正此前扩大范围的实现。第 5 步两张能带对比图、四面板总览的电子能带面板和独立 Wannier 绘图均为 QE 深灰连续实线先绘、EPW／Wannier 蓝色短虚线后绘；保持 4∶3 参考／插值线宽和红色费米线。DOS 的长短虚线规则不变。

EPW 单元 45 通过、2 个既有外部来源用例忽略，共享绘图 4 通过，公开比较／总览回归 9 通过。完全相同的能带输入在绘图区同一行保留两色且横向连续，SVG 中 QE 能带为完整多点折线，图例为实线。日志为 `docs/local/0055-band-solid-unit.log` 和 `0055-band-solid-comparison.log`。

真实已有 PBE MgB₂ 结果重绘至 `test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results`，八张保持 3000×2100／300.0248 DPI；对照上版包，只有第 5 步两张能带 PNG 改变，其余六张图片字节不变。31 份源结果及 13 份 CSV／plot.json 集合与 SHA-256 不变；空科学程序 PATH 重绘及结果包核验通过，没有科学调用或新 Slurm 作业。证据在同目录 `verification.json`、`figures.json`、`check.json` 和 `preview.png`。上述纠正覆盖前一节记录的电子能带长虚线行为，前一节作为历史证据保留。

四个受影响程序已重新构建并安装，默认 PATH 的 `epw_wannier --plot` 实际重绘及 `--results-check` 通过；独立 `wannier90 plot` 用相同的两份能带确认两色可见且 QE 全区间连续。构建和安装日志为 `docs/local/0055-band-solid-build.log`，默认入口日志为 `0055-band-solid-installed-plot.log`／`0055-band-solid-installed-check.json`；制品与源码 SHA-256 记录在 `0055-band-solid-deployment.json`。此次不重建或替换 `epw_clear`，不提交已有 AGENTS.md 修改。

## QE 参考实线及第 5 步实际外窗纠正验收

用户指出前次只修正能带、遗漏 DOS，并要求第 5 步默认能带与 DOS 使用实际 dis_win 外窗。本节覆盖前两节中仍保留 QE DOS 长虚线及第 5 步默认 ±5 eV 的行为；原记录保留为历史证据。

共享样式删除 QE 虚线定义，所有 QE 参考及其图例统一实线，涵盖能带、DOS、声子比较和总览；插值保留蓝色短虚线，电子费米线为红色。第 3 步元素 PDOS 保留不同长度的短虚线，TDOS 为实线。多曲线图参考／插值线宽为 4∶3。

第 5 步默认 `electronic-bands.png`、`electronic-dos.png` 统一使用实际 dis_win_min−E_F～dis_win_max−E_F，正常发布与旧结果包独立重绘均适用。绘图参数保存相同边界，已有显示元数据补充 `default_ev`／`default_preset`；缺失、重复、非有限或反序的外窗拒绝出图，不以 frozen 窗或显示预设代替。`-valence` 范围、第 3 步默认 ±5 eV 及通用比较总览预设保持原约定，不修改实际拟合窗口或科学比较指标。

执行 `CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh test --offline --release --locked -p epwflow --lib --test comparison_plots`：EPW 单元 46 通过、2 项既有外部真实来源用例忽略，比较／总览 9 通过，共 55 通过、2 忽略。新增回归检查完全重合的能带、DOS、TDOS／元素曲线本体连续无 QE 虚线空隙，检查所有比较面板的 SVG 实线及旧包外窗重绘不改变数值、不二次扣除费米能。忽略项为显式真实粗网格衰减与 wdata 来源检查，不计通过。日志为 `docs/local/0055-qe-solid-outer-tests.log`。

四个受影响程序经离线发布构建重新更新到仓库 `bin/epwflow/{epw_phonon,epw_wannier,epw_superconductivity}` 及 `bin/wannierflow/wannier90`。构建日志为 `docs/local/0055-qe-solid-outer-build.log`；加载 `scripts/env.sh` 后解析到上述入口，四个产品的构建源与发布文件 SHA-256 均匹配发布元数据。源码边界为 `2c811c3` 加本次改动，未混入 AGENTS.md；来源与制品哈希见 `docs/local/0055-qe-solid-outer-deployment.json`。未替换 `epw_clear`。

使用已发布 `epw_wannier` 在 `PATH=/nonexistent` 下直接原位重绘用户指定的 `test_auto/MgB2/local_run/014-electronic-plots/band-solid-2r9ljckd/results`，结果包核验及八图校验通过。对应实际 E_F 为 9.16990224089968 eV，绝对外窗为 −3.40453～26.42576 eV，因此两张默认对比图显示 −12.574432240899679～17.255857759100323 eV（相对费米能）。完整价带仍为 −75.29716629237811～5 eV。八张 PNG 保持 3000×2100、300.0248 DPI；实际查看默认能带、默认 DOS 和完整价带 DOS，确认实线、虚线、红色费米线和范围。

更新前完整结果备份及本轮证据位于 `test_auto/MgB2/local_run/014-electronic-plots/qe-solid-outer-ifh9bhhb/`，包括 `before-results/`、`verification.json`、`figures.json`、`check.json`、`preview.png`。5 份科学 CSV 字节不变，8 份绘图数据中的科学数组与全部来源信息不变，仅两张默认对比图的边界更新；已有显示元数据同步。改变的图片为第 3 步两张 DOS、第 5 步默认能带及两张 DOS，其余三张能带字节不变。无科学程序调用、无新 Slurm 作业，预算不增加。不将结果包核验写成完整工作流或科学拟合验收。

模块手册、Overview、示例 README 与需求产物已同步当前行为；线上 Issue 仍由用户管理，不推送，不改写旧科学数据或冻结脚本。

## 2026-11-16：统一组合图

本节替代旧章节中关于八张分离图、±5 eV、最低占据态至 E_F＋5 eV，以及第 5 步默认实际外窗的当前输出要求；历史原话及旧验收数据不改写。

### 实现与覆盖

| 需求／标准 | 实际交付与证据 |
|---|---|
| FR-001～004、SC-001～002 | 第 3、5 步各两张组合图，左能带右 DOS；共享 [plot] 两个独立上下限，默认 −3／＋3；全范围只取 QE 路径全部本征值极值。公开配置修改过程核对科学身份与调用数不变，真实 XML 独立复算极值一致。 |
| FR-005～007、SC-001／003 | 第 4、6 步各一张声子组合图，0 至路径最高频率；复用既有声子投影聚合／展宽，原始负频不改。共享渲染器使左右纵轴像素对齐，DOS 转置；QE 实线、EPW 蓝色短虚线，元素区分线型及四窗口保留，不恢复紫色标记。 |
| FR-008～010、SC-004 | DOS 失败保留科学数据和逐带范围诊断；缺图、配置修改、移动结果包及真实写图失败恢复不增加科学调用。阶段图放入 plot/，结果包按阶段 plot/ 发布，旧管理图片定向清理。 |
| SC-005 | Issue／Spec、契约、任务、相关 0047／0050、Overview、根／模块及示例说明同步；release 编译覆盖固定命令，按明确路径单独提交，不推送。 |

### 验证结果

- release 单元：75 通过、4 项既有外部真实来源检查忽略，0 失败；忽略项不计通过。
- release 公开进程：31 通过、0 失败，覆盖 qe_stage_plots、stage_results、phonon_import、phonon_chain，以及原生 DOS 归一化与失败恢复。
- 最后补充的第 3 步费米能／展宽来源元数据保留，经 QE 绘图 7 项再次复测通过，不与前面的 31 项重复计数。
- 显式真实只读出图：1 通过，使用已有 MgB₂ 电子与声子数据生成 6 张组合图；来源哈希检查通过，科学程序调用和 Slurm 提交均为 0。
- 真实 PNG 均为 3600×2400，密度不低于 300 DPI；实际查看近费米、全范围电子组合图及声子组合图，检查窗口、符号、末端标签与 DOS 刻度间距。
- 绘图缓存曾因 plot/ 与 plot 字符串不一致触发重复提交；已统一目录表示并兼容旧清单尾斜杠，不重复提交回归通过。声子比较方法名大小写适配已修正，真实数据与过程回归均通过。
- 构建命令为 bash scripts/build-native.sh build --release -p epwflow --locked，已覆盖固定 epw_phonon、epw_wannier、epw_superconductivity、epw_clear，无版本目录或旧命令备份。

### 真实图与原始证据

只读来源为 examples/MgB2/ubuntu/run。新证据目录为 docs/local/0055-combined-final-20261116，verification.json 登记六张图片、来源身份与零科学调用，logs/ 保存单元、公开进程、真实出图和最后构建日志。未覆盖来源工作区的旧图片或科学结果；新生成端和恢复入口按当前组合图契约交付。

构建保留用户此前未提交的共享线宽设置；该样式差异不纳入本次提交。示例 input.toml 只提交新增 [plot]，原有注释排版修改保留在工作区。其他任务未提交的 VASP 测试修改不纳入。

线上 Issue、远程双向链接与历史上传图片原文件仍由既有外部约定管理，本轮不宣称完成。真实重绘验证布局和数据来源，不新增物理收敛结论。

## 第 3、5 步现场纯重绘入口补齐

用户要求立即调整 input.toml 并试画第 3、5 步。现有 Ubuntu 配置保留 −3／＋3 eV，只补充独立上下限及重绘命令注释。补齐 epw_phonon --stage electronic --plot，且 epw_wannier --plot 支持旧现场保存的分离图参迁移；均只使用已有数值，不进入科学阶段推进或修改独立 results 包。

公开回归 2 项通过：旧分离图参迁移到两张组合图，非对称区间生效，第 3 步不提前改第 5 步，科学／后处理调用及提交数不变；既有组合图缺图重绘仍通过。release 编译覆盖固定入口成功。

在 examples/MgB2/ubuntu 实际运行上述两个命令成功，两个阶段 plot/ 各有 electronic.png 和 electronic-from-lowest-to-highest.png。指定 XML、原生 DOS、模型及来源表哈希不变；strace -f -e trace=execve 只记录两个绘图入口自身，无 QE、EPW、postw90、MPI 或 Slurm 子程序。证据在 docs/local/0055-current-replot-35/verification.json 及同目录调用记录。核验不枚举或读取原始电荷密度／波函数。
