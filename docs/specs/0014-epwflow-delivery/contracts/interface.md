# CLI 与数据契约：图表、真实对照与安装交付

**版本**：1；**需求**：[Spec](../spec.md)；**模型**：[数据模型](../data-model.md)。

## 公开接口

完整命令、参数、默认值、单位与行为以 Spec 的 FR-001 为规范；本接口不另改命名。所有 --help 均无科学执行副作用。参数使用 argv，显式路径按所属配置或命令规则解析，不 eval。

共同执行/错误/持久协议由 [0008 基础契约](../../0008-epwflow-foundation/contracts/interface.md) 唯一拥有；科学字段由各自 Spec 定义，全部进入同一 TOML schema_version=1。check/status 的 --json 返回 schema_version、operation_status、workflow_status、diagnostics；status 查询成功不代表科学完成。

## 输入输出与错误

读取经过绑定的 input-init.toml 及前置 manifest；每次读取确认版本和指纹。缺字段/类型错误/单位不符返回非零并报告字段路径；状态未知、预算拒绝和数据无效用独立错误类别，不自动重试。可预期的未定位或部分结果以结构化状态表示，不填伪造数值。

F1 返回配置/运行绑定与阶段协议，F2 返回 qe-data/manifest.json，F3 返回 interpolation/manifest.json，F4 返回 superconductivity/manifest.json 与 tc.json，F5 返回 results/manifest.json。每个 manifest 列出完整角色清单、尺寸、哈希、生产阶段、上游摘要与验证状态。文件记录采用相对 artifact_root 的路径；绝对机器路径只存在实际运行的忽略目录。拒绝路径穿越和指向来源的可写链接。

## 本项数据字段

- `ResultManifest`：schema_version,run_id,source_digests,tables,figures,status,source_verification；移动可重绘；原始不可访问不伪造重新验证。
- `ComparisonCell`：ph_mode,solver_mode,reference,input_hashes,metrics,tolerances,status,evidence；六个唯一组合，缺任一项不得整体成功。
- `InstallManifest`：source_commit,diff_digest,platform,source_hashes,packaged_hashes,entries,node_checks；逐文件状态；无默认历史二进制。
- `Figure`：kind,mode,temp,smearing,units,data_hash,format,path,status；只画有效量；状态图不冒充科学曲线。

## 权限、兼容与副作用

仅使用当前用户已有权限。不得自动修改系统库、清除预算、覆盖来源、取消无关作业或推送仓库。默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。

文件接口升级须新版本并提供显式兼容检查，不能静默读取未知格式。失败输出可保留用于诊断，但不能充作下游成功数据。查询和错误输出不得泄露无关配置或认证内容。

## 必需产物角色和交接

summary、a2f、coupling、gap_summary、gap_states、tc、report、figure_png、figure_svg、figure_pdf；各图由对应规范化表产生，figure_manifest 哈希不反馈修改原始科学文件。

Artifact={role,relative_path,byte_size,sha256,producer_stage,input_digest,format_version}；validation={status,checks,units,thresholds,evidence}。只有 status=valid 的必需角色完整才发布该阶段 manifest。format_version 未知或文件集合与已验证适配器不符即拒绝；不靠 glob 命中一个文件放行。

## 2026-10-19：比较结果包与图表契约（完整目标；当前子集见后文）

沿用已规划的 `epwflow collect -w W [--output R]`、`epwflow plot --results R [--output D] [--formats png,svg,pdf]` 和 `epwflow results check --results R [--json]`，不新增平行命令。当前落档不表示这些公开入口已经接通；实际实现／验收以 acceptance.md 为准。

collect 允许没有 F4 清单的有效 F3 诊断输入；输出 R/comparison，保留 [0012 契约](../../0012-epwflow-interpolation/contracts/interface.md) 的五类 CSV、metadata.json、metrics.json 和独立 manifest.json。上游原始数据只读，图表所需数据全部随包保存。原结果主清单兼容性保持原约定，新比较包用独立版本；旧包缺此数据不能通过新增叠图验收。

| 图种基名 | 内容 | 图层顺序 |
| --- | --- | --- |
| electronic_bands_comparison | 共同有效窗内 QE／Wannier 能带 | QE 实线→EPW 虚线 |
| electronic_dos_comparison | 总量、Mg/B 元素及严格轨道电子 DOS 分面／图例 | 同投影 QE 实线→EPW 虚线 |
| phonon_dispersion_comparison | QE q2r/matdyn 与 EPW 连续曲线，独立 ph.x 点 | QE 实线→EPW 虚线；直接点独立标识 |
| phonon_dos_comparison | 总量与 Mg/B 元素声子 DOS | 同元素 QE 实线→EPW 虚线 |
| comparison_overview | 上述四类的 2×2 总览 | 复用相同图层语义 |

图种各输出 PNG／SVG／PDF，登记在绘图产物清单中；默认 D=R/figures。metadata 的能窗、投影标签、E_F、单位、核／宽度与归一化必须同时进入图例或可定位的说明；不画跨路径断点连线。

缺可靠投影或参考时，plot 只能生成标明缺项的部分面板；结果检查明确 comparison_complete=false。篡改、未知版本、非法单位／非有限值或未知文件覆盖冲突返回非零。完整但质量失败的结果允许操作成功并输出诊断图，同时 quality_status=failed、workflow_complete=false；操作成功绝不等同科学验收通过。

collect／plot／results check 不运行 QE、EPW 或 sbatch，不创建科学任务；缺参考计算应由受预算控制的 F3 明确执行。重绘仅更新本工具拥有的图像和绘图清单，comparison 表格和科学源文件内容必须保持不变。

## 2026-10-19：已接通的比较包命令

公开 collect／plot／results check 已消费上述独立 comparison 包；没有超导清单也能导出有效诊断。默认输出 results/comparison、results/figures，后者的 comparison-figures.json 绑定源清单与全部图像哈希；results check 验证默认图目录的该清单（若存在），自定义 --output 的图目录不由默认检查自动定位。重复收集仅接受相同来源；重绘只覆盖清单拥有且哈希仍相符的图像，未知冲突拒绝。

当前四张独立图与总览均支持 PNG、SVG、PDF；总览的 DOS 面板展示总量，元素／轨道在独立 DOS 图分面。缺投影面板显示缺失，标题标记 PARTIAL RESULTS；quality=failed 与 workflow_complete=false 保留。电子能带按真实线段与能窗求交裁剪，不能把窗外线段压到边界形成假平带。只有所有新增投影、覆盖及科学标准满足后才能通过完整叠图验收。

本轮 collect／plot／results check 仅交付比较包，不表示 Spec FR-002 的完整 α²F／能隙／Tc 结果包已经接通；旧 delivery 子命令兼容保留。

### 拟合外窗展示

F3 metadata.disentanglement_window_ev 存在时，电子能带纵轴和电子 DOS 横轴严格采用 window_ev，标题同时注明实际 dis_win_min/max 的绝对 eV 和 E−E_F。独立图及总览复用同一语义；不存在该字段的旧比较包继续按其已有 window_ev 绘制。plot 不读取外部 wout，不重新运行科学程序。


## 高对称路径的绘图轴约定

电子能带与声子色散只在底部横轴下方显示高对称点符号，按真实路径距离定位；不显示路径数值刻度或 k path／q path 标题。此约定同时作用于独立图和总览子面板，DOS 物理横轴仍显示数值及单位。比较表保留原始路径距离；绘图不改写数据。


### 外窗与冻结窗边界

metadata.frozen_window_ev 是可选的绝对 eV 二元数组，来自同一已核验 wannier_wout 的唯一 Inner 记录；缺失代表未记录。存在时必须有限、递增且包含于 disentanglement_window_ev。四个边界均由绝对值减同一 fermi_ev 后定位，图例使用 dis_win_min/max、dis_froz_min/max 并注明 E−E_F；曲线数据与 display window 不因标注而改变。重合边界不偏移，交错显示不同颜色的虚线。旧包不能根据显示范围推断物理窗口；未知、无效或篡改数据依现有清单规则拒绝。

### 费米能级标识

电子能带／DOS 的共同参考为 E−E_F=0 eV。plot 使用绿色虚线及 E_F 文字／图例，能带水平、DOS 竖直，独立图及总览一致；该参考线与 dis_win／dis_froz 的物理边界分别标识。复用已有 fermi_ev 和相对能量，不增加元数据或再次平移能量；缺失电子数据的面板及声子图不伪造此参考。

## 2026-10-19：50／20 门与原生代码收尾

对应 FR-001～005、SC-001/003/004。F4 版本 2 通过独立原生结果包交付，不压缩成旧单 Tc 标量。包内保存分模式搜索、最低正频率逐态 Δ／Z／k 坐标／物理权重、带统计、原生谱与累计 λ；完整 Matsubara 数组保留 F4 哈希来源。六类图均由 Rust 消费包内表，支持 PNG／SVG／PDF 和移动后空 PATH 重绘。显式 collect／plot 通过发布日志恢复半写文件；只替换已登记且哈希匹配的旧文件，未知文件拒绝覆盖。

科学求解与真实验收共用原预算；9³ PH 暂缓，不新增需求或另建线上 Issue。阈值调整不表述为插值改进，真实 Tc、独立手动参考与完整自动链仍须各自提供证据。

## 自动阶段的图表完整性与恢复

第二阶段要求四类比较图及总览的 PNG／SVG／PDF 全部登记。单格式重绘保留同源、已登记且哈希有效的其他格式归属；若只有单格式，不能通过自动阶段的三格式门。比较来源改变须使用新输出目录。比较图与原生结果图复用发布日志：检查拒绝半写状态，显式绘图先验证并恢复已有日志，不覆盖未知内容。F1 显式恢复只在原作业已确认终态且锁归属相同时回收比较导出、收集和绘图锁。

## 原生图部分格式重绘

原生图与比较图都由各自清单继续拥有同源、哈希匹配但未请求重绘的格式，不能把它们遗留成后续无法覆盖的未知文件。同源只重绘 SVG 后，原三格式交付仍可验证。数据来源摘要改变时，必须重绘全部已有格式或选择新输出目录；不能给旧 PNG／PDF 标记成新数据。重绘仍不执行科学程序，篡改文件仍拒绝。对应 0008 T046、0014 FR-004/005；进程与文件故障验证见[连续执行设计](../../0008-epwflow-foundation/public-chain-validation.md)。
