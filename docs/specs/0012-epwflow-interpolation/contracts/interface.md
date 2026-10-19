# CLI 与数据契约：Wannier 化与 EPW 插值

**版本**：1；**需求**：[Spec](../spec.md)；**模型**：[数据模型](../data-model.md)。

## 公开接口

完整命令、参数、默认值、单位与行为以 Spec 的 FR-001 为规范；本接口不另改命名。所有 --help 均无科学执行副作用。参数使用 argv，显式路径按所属配置或命令规则解析，不 eval。

共同执行/错误/持久协议由 [0008 基础契约](../../0008-epwflow-foundation/contracts/interface.md) 唯一拥有；科学字段由各自 Spec 定义，全部进入同一 TOML schema_version=1。check/status 的 --json 返回 schema_version、operation_status、workflow_status、diagnostics；status 查询成功不代表科学完成。

## 输入输出与错误

读取经过绑定的 input-init.toml 及前置 manifest；每次读取确认版本和指纹。缺字段/类型错误/单位不符返回非零并报告字段路径；状态未知、预算拒绝和数据无效用独立错误类别，不自动重试。可预期的未定位或部分结果以结构化状态表示，不填伪造数值。

F1 返回配置/运行绑定与阶段协议，F2 返回 qe-data/manifest.json，F3 返回 interpolation/manifest.json，F4 返回 superconductivity/manifest.json 与 tc.json，F5 返回 results/manifest.json。每个 manifest 列出完整角色清单、尺寸、哈希、生产阶段、上游摘要与验证状态。文件记录采用相对 artifact_root 的路径；绝对机器路径只存在实际运行的忽略目录。拒绝路径穿越和指向来源的可写链接。

## 本项数据字段

- `InterpolationManifest`：f2_digest,wannier_digest,coarse,fine,spectra,quality,mode_readiness；粗细谱各自依赖图，不混用基底。
- `QualityReport`：points,energy_reference,matching,max_error,rms_error,coverage,thresholds,status；缺参考/覆盖不足即无效。
- `FineData`：kpoints,qpoints,energies,frequencies,weights,kqmap,ephmat,layout；矩阵维度和掩码与索引一致。
- `Spectrum`：smearing_id,energy_mev,a2f,lambda_integral,lambda_sum,cutoff；各展宽分列；零频规则明确。

## 权限、兼容与副作用

仅使用当前用户已有权限。不得自动修改系统库、清除预算、覆盖来源、取消无关作业或推送仓库。默认 outdir 是按配置目录解析的基址；实际保存树为 <outdir>/<run_id>/<stage_id>。同一绑定内路径一经准备不变，用户自定义优先，工作目录与保存目录分别显示。

文件接口升级须新版本并提供显式兼容检查，不能静默读取未知格式。失败输出可保留用于诊断，但不能充作下游成功数据。查询和错误输出不得泄露无关配置或认证内容。

## 必需产物角色和交接

wannier_input、wannier_log、rotation、disentanglement（适用时）、epw_coarse_data、coarse_ephmat、quality_reference、quality_report、fine_frequencies、fine_eigenvalues、fine_kqmap、fine_ephmat_shard、spectral_table；从目标 EPW 读取例程确定文件记录集合并生成 schema，其中缺任一角色即不发布。F4 消费其谱函数或 fine 集合。

Artifact={role,relative_path,byte_size,sha256,producer_stage,input_digest,format_version}；validation={status,checks,units,thresholds,evidence}。只有 status=valid 的必需角色完整才发布该阶段 manifest。format_version 未知或文件集合与已验证适配器不符即拒绝；不靠 glob 命中一个文件放行。

## 2026-10-18 原生读取与讲义参数适配

以下是已实现的局部接口，不等于完整 F3 自动执行：

- `wannier.frozen_max_ev` 可只给冻结窗上界，与 `frozen_window_ev` 互斥；没有外窗时沿用目标 Wannier90 的默认语义，不虚构下界。F2 Fermi 相对能窗转换成实际 eV 数值，不写未知 `energy_reference` namelist。
- 顶层可选 `atomic_masses`（元素→amu）由共享 QE 输入层和 EPW `amass(i)` 使用。EPW 序号须来自结构元素顺序；未绑定顺序时拒绝渲染显式质量。
- 原生 coarse 使用 `qe.nscf.k_grid`、`phonon.q_grid`；fine 使用 `interpolation.k_grid/q_grid` 的 `nkf*/nqf*`，不自行乘二。`degaussq` 单位为 meV，`degaussw/fsthick` 为 eV。5 个 Wannier 函数不等于 5 条投影字符串，轨道展开另行核验。
- 原生只读目录有 `epw.in/epw.out/epw.exitcode`、相应 wout 和唯一 a2f 谱时，`epwflow interpolation check -w W --json` 返回 `native_epc_only`、`workflow_complete=false`；这不认证质量、能隙或自动链。
- 目录有 `quality-reference.json` 时，同一命令检查真实 QE XML、EPW `band.eig/phband.freq` 与同 ASR 的参考频率，输出电子与声子最大/RMS及逐模式误差。超差保留 JSON 报告并非零退出；不会启动 QE/EPW/sbatch。

`quality-reference.json` 使用版本 1，字段为 `schema_version/asr/window_ev/fermi_reference/electron_reference/electron_interpolated/phonon_reference_cm1/phonon_interpolated_mev/provenance`。文件项为 `{path,sha256}`，相对路径从清单目录解析；读取前重新核对。ASR 明确为 `epw_simple_gamma_onsite`，派生频率须登记原始 PH、转换程序和实际 dynmat 输出。缺来源、指纹改变、缺态或错误单位均拒绝。点的独立性、冻结计划和完整物理来源仍属于验收检查，不能由一份自述清单自动取得认证。

数值阈值仍为 Spec 的当前 0.05/0.02 eV、50/2 cm⁻¹（2026-10-19 用户只调整声子最大误差）。质量检查先于细网格；通过的历史报告不能绑定到另一套粗网格目录。本轮尚未替换旧全部 coarse/fine 占位格式，不能把渲染和只读检查当成从头运行接口。

## 2026-10-19：叠图数据交接契约（完整目标；当前子集见后文）

FR-014/015 使用同一配置绑定和现有阶段入口，不新增一套 CLI 或自由 namelist 透传。路径、DOS 网格、投影与展宽进入统一类型化配置和快照；`quality` 已固定的共同能窗／参考 E_F 继续复用，新增比较路径不能改变原独立质量点。check/prepare 只校验并显示实际数据计划；必要参考只在受控 run/submit/resume 中计算。

生产者为 F3 的 `interpolation/comparison/`，消费者为 F5 和 0015 验证：

| 文件角色 | 最低数据 |
| --- | --- |
| electronic_bands.csv | method、segment_id、point_id、distance、band、energy_ev；坐标与能量零点见元数据 |
| phonon_bands.csv | method、reference_kind、segment_id、point_id、distance、mode、frequency_cm1 |
| phonon_reference_points.csv | 真实 ph.x 点、坐标、mode、frequency_cm1 与来源；不能冒充连续曲线 |
| electronic_dos.csv | method、energy_ev、projection_kind、element、orbital、density；总量行标签明确 |
| phonon_dos.csv | method、frequency_cm1、projection_kind、element、density |
| metadata.json | 路径与 DOS 网格、原始／统一单位、E_F／有效窗、质量／ASR、投影算符／规范、权重、核／σ、范围／尾部及归一化 |
| metrics.json | 对应曲线逐点差、最大／RMS、DOS 绝对／归一化 L1 差、积分、覆盖与缺项；参考积分为零时不用除零公式 |
| manifest.json | 独立 schema_version=1、run_id、PH 模式、上游指纹、相对文件角色／大小／哈希、数据完整性和科学质量状态 |

上表是规范化交接，不改名或覆盖原始科学文件。电子轴为相同 E−E_F（eV），DOS 为 states/(eV·cell)；声子轴为 cm⁻¹，DOS 为 states/((cm⁻¹)·cell)。若原生能量／频率坐标换算 x′=c x，密度相应为 D′=D/c，积分与符号保持；必须记录原单位和转换依据。对比在相同网格上生成，不能事后自由平移、平滑或重采样以降低误差。

缺投影、规范不明、网格／ASR／展宽不一致、窗内缺态、空文件、篡改或非有限值均拒绝对应数据就绪；其余有效面板可显式部分交付。投影完整性与物理质量分开保存，quality_failed 的完整数据仅作诊断，workflow_complete=false、下游不就绪。

独立清单相对 artifact_root 定位，绑定原 F3 来源摘要；旧主清单保持原读取契约。未带 comparison 清单的旧结果可以按旧功能读取，但不能通过新增叠图验收；未知新清单版本明确拒绝，不静默把旧输出当成新投影。

## 2026-10-19：已实现的比较诊断子集

新增 `epwflow interpolation export-comparison --source S --output D`，只读解析已有原生文件，不运行科学程序或提交作业；D 必须不存在。该入口不代替计划中的受控参考计算阶段。数据模型位于 interpolation/comparison_data.rs，F5 直接消费该模型，避免 F3 反向依赖绘图模块。

来源 JSON 严格使用 schema_version=1，包含 run_id、ph_mode、quality_reference、fermi_reference、qe_bands_xml、qe_mesh_xml、epw_bands、qe_phonon_bands、epw_phonon_bands、epw_log、provenance、path、ticks、mesh、mesh_shape、reciprocal、window_ev、electronic_sigma_ev、phonon_sigma_cm1；文件项沿用 `{path,sha256}`。相对路径按 S 所在目录解析；path 的项为 segment_id、point_id、distance、crystal，ticks 为 distance、label；reciprocal 将晶体坐标右乘转换成 QE 的 2π/alat 坐标。

当前原生输入要求电子／声子插值文件依次含比较路径和完整 Γ 中心 DOS 网格；QE 能带 XML 只含路径，独立 NSCF XML 含完整均匀网格。检查实际坐标、点数、权重、原子顺序与九声子模式。适配器只接受 Mg、B、B；不宣称通用材料适配已完成。

共同单一参考 E_F；电子因非磁性自旋简并使用权重 2/N，声子使用 1/N。两侧在同轴上用 `exp[-((x−e)/σ)²]/(σ√π)` 累积，不按曲线重新归一化。声子 meV→cm⁻¹ 在累积之前转换，保留虚频；频率轴包含八倍 σ 的核尾。电子表仅覆盖所选共同有效窗，报告窗内积分，不能当全电子态计数；缺窗内态显式记录为不完整。

规范化 CSV 的表头与前表相同，当前数值子格式不支持带引号／逗号的标签；复杂标题和备注放在 JSON 中。独立 manifest 严格核对五个 CSV、metadata.json、metrics.json 的完整集合、相对路径、大小和哈希。元数据单位必须为 eV_relative_fermi、cm-1、states/(eV*cell)、states/(cm-1*cell)。未知版本、非法单位、非有限值、错误网格或篡改均拒绝。

当前导出仅有总 DOS；metadata 的 electronic_projection、phonon_projection 明确 unavailable，并在 manifest.missing 说明缺项。消费其他数据中的真实元素／轨道列时，必须分别声明 atomic_projector_operator、mass_weighted_eigenvectors；该声明只是格式前提，不代替物理来源验收。质量失败保留诊断，不发布细网格 EPC 或超导就绪。


2026-10-19 当前范围：元素／轨道投影暂缓，严格投影契约保留供后续恢复，不以总量填充投影列。原生质量报告新增 `phonon.max_error_limit_cm1=50`、`rms_error_limit_cm1=2`，实测误差仍原样记录；`workflow_complete=false` 不变。旧报告作为当时标准的证据，不覆盖重写。

### 紫色误差标注的已实现数据字段

metadata.json 的可选 `phonon_error_markers` 默认为空，兼容旧比较包。每项字段为 `scope=sampled_maximum|path_maximum`、`q_crystal[3]`、1 起始 `mode_sorted`、`qe_cm1`、`epw_cm1`、`error_cm1`、可空 `path_distance/path_label`。来源为 quality-reference.json 的直接 PH 与 EPW 频率，逐点排序匹配；晶体坐标由既有 reciprocal 逆变换，路径身份按实际采样点核对，重复路径端点取第一次出现。字段有限性、模式范围、差值一致性、scope 唯一性和路径位置均校验，元数据包含于原清单哈希。路径外最大不写伪距离或高对称点名。两个最大值只覆盖核验样本，不代表全布里渊区。

### 实际外窗的诊断导出

comparison Sources 新增可选 wannier_wout 文件项（path、sha256），必须为实际 Wannier `.wout`，解析唯一 `Outer: min to max (eV)` 行，支持 Fortran 指数并拒绝非有限、逆序或多条冲突窗口。给出时覆盖原 window_ev 为绝对窗减共同 E_F；不修改 quality-reference 的验证窗。metadata.json 可选 disentanglement_window_ev 保存绝对边界，缺省为空以兼容旧包；存在时校验与 window_ev、fermi_ev 的数值关系。科学带值、DOS 权重、质量门不因此变更，缺态继续报告。


### 外窗与冻结窗边界

metadata.frozen_window_ev 是可选的绝对 eV 二元数组，来自同一已核验 wannier_wout 的唯一 Inner 记录；缺失代表未记录。存在时必须有限、递增且包含于 disentanglement_window_ev。四个边界均由绝对值减同一 fermi_ev 后定位，图例使用 dis_win_min/max、dis_froz_min/max 并注明 E−E_F；曲线数据与 display window 不因标注而改变。重合边界不偏移，交错显示不同颜色的虚线。旧包不能根据显示范围推断物理窗口；未知、无效或篡改数据依现有清单规则拒绝。

## 2026-10-19：50／20 门与原生代码收尾

对应 FR-009/012、SC-003/007。本次只调整共享 RMS 常数与边界回归。五点、45 模及相同 ASR、单位、电子窗保持不变；旧失败报告不改写。新的只读报告留在编号目录 029。

科学求解与真实验收共用原预算；9³ PH 暂缓，不新增需求或另建线上 Issue。阈值调整不表述为插值改进，真实 Tc、独立手动参考与完整自动链仍须各自提供证据。

## 原生执行交接细化

quality.energy_window_ev 为相对已绑定 F2 Fermi 的 eV；QualitySources.window_ev 仍为绝对 eV。首次执行只核验原质量清单的独立参考／来源，不要求其中旧插值输出存在。当前粗网格完成后生成工作目录内 quality-reference.json、quality-report.json 和 quality-binding.json；绑定含原清单和新配对清单的路径、规范目标、字节及 SHA-256，F1／F4 必须重验。

粗网格格式标识 epw5.9-coarse-le-c16；必需重启角色是 coarse_epmatwp、coarse_epwdata、coarse_crystal、coarse_wigner、coarse_vmedata、coarse_dmedata，另记录输入、日志、Wannier 日志和独立验证输出。epmatwp 位于 coarse/out，文本重启文件位于 coarse 根。只支持已核验的小端 REAL(8) 复数布局，未知布局拒绝。

每个展宽在 fine/smearing-NNNN 独立运行；fine 主数据来自 spectrum_index 所选目录，其余展宽的产物角色以 smearing_NNNN_ 前缀保存于同一 artifacts 列表，所有文件均须验源。F4 仅使用未加前缀的所选谱／矩阵角色，不再要求所选谱恰为最后一个展宽。版本 1 的清单外壳不变；本细化不允许旧简化文本进入原生生产路径。


## 粗／细尝试恢复契约

沿用三个程序的 --resume 选项，不增加可执行程序或阶段命令。native-attempts/<角色>.json 使用版本 1，包含唯一尝试编号、固定请求摘要、受控目录、preparing／running／complete／archiving 状态、物化输入身份、成功输出身份、归档目标与原目录是否存在的证据。请求摘要覆盖完整配置、EPW 程序身份、启动参数、F2／原粗清单与独立参考清单身份；逐文件来源仍由阶段清单核验。

普通执行遇到既有尝试拒绝。显式恢复先重验输入及请求；complete 必须重验全部成功输出。running 若退出码和全部原生角色有效则补完成记录，不执行 EPW；否则先持久化归档目标并保留失败现场，再启动一次固定参数尝试。归档中断可继续，原目录与归档同时存在或归档证据丢失时拒绝。未知既有目录不收编。每个展宽分别登记，成功展宽检查点及文件保持不变，只有失败展宽重试；不重新执行 SCF／PH，不重复登记阶段预算。

质量三文件发布中断后仅补齐字节内容完全相同的缺项；已有内容不一致拒绝。质量失败重验同一原生输出，不能重算以规避质量门。工作流显式恢复可重验已发布阶段清单、补登记完成证据；不重启该阶段。旧的无检查点现场仍须保留，并通过受支持导入或单独审查处理，不能直接自动恢复。

## 三程序中的自动参考

不新增命令或第四个程序。`epw_wannier` 在同一已登记分配内、原生粗网格前补齐固定独立参考；生成脚本或检查配置本身不启动参考计算。不提供 `reference_manifest` 即采用该路径，显式提供时继续只读复用。

自动路径要求 `ph.x` 同目录存在 `dynmat.x`，且其身份已随初始运行冻结；后补或替换程序不能直接续算原绑定。选窗能力预检失败时不开始参考科学调用。参考缺失、坐标／模式／单位不符、原生退出／完成日志失败均停止第二阶段；`--resume` 才可重试固定失败任务，不重做 SCF，不扫描参数，不建立独立预算。

## 第二阶段自动比较接口

公共入口仍为 `epw_wannier`；在已冻结 KPATH 的运行中，路径、DOS 网格原生生成与四类图是阶段成功前置。配置、恢复、质量失败诊断和数据契约见[自动比较设计](../automatic-comparison.md)。比较生成属于 F3，collect／plot 仍只读消费。
派生 PH XML 必须符合 QE xmltools 复数矩阵记录布局：PHI 标签后另起九行实虚数对，完整保留 3×3 列主序数值。独立 dynmat 和比较 q2r 使用同一序列化边界，矩阵规范摘要不受文件布局修复影响。

自动参考显式恢复可更新编排程序；原请求保持不变。必须重验当前配置／launcher 对原请求的摘要、五项科学来源与全部原身份（包含旧编排程序），参考发布同时绑定新旧程序。非显式恢复或其他来源／输入变化拒绝，不自动重建未知检查点。
单 q 坐标模式（ldisp=false、非 qplot）的原生响应势固定在 out/_ph0 根目录，包括非 Γ q；网格模式的非 Γ 点继续用 prefix.q_N。消费者按模式映射且保持唯一性、物理 q 与响应完整性检查，不跨目录猜选或静默降级。
dynmat modes.out 的 q 显示明确支持四位／八位小数，允许对应半个末位单位的舍入差；跨舍入区间或其他格式拒绝。原 PH XML 与固定输入仍按原高精度校验，参考表使用固定 q，不用打印值替换物理采样点。模式完整性、单位、科学误差门均不变。
