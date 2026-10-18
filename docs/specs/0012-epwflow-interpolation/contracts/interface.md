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

数值阈值仍为 Spec 的当前 0.05/0.02 eV、15/2 cm⁻¹（2026-10-19 用户只调整声子最大误差）。质量检查先于细网格；通过的历史报告不能绑定到另一套粗网格目录。本轮尚未替换旧全部 coarse/fine 占位格式，不能把渲染和只读检查当成从头运行接口。

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


2026-10-19 当前范围：元素／轨道投影暂缓，严格投影契约保留供后续恢复，不以总量填充投影列。原生质量报告新增 `phonon.max_error_limit_cm1=15`、`rms_error_limit_cm1=2`，实测误差仍原样记录；`workflow_complete=false` 不变。旧报告作为当时标准的证据，不覆盖重写。
