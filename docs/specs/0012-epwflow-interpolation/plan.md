# 实施计划：Wannier 化与 EPW 插值

**Issue**：[本地 0012](issue-draft.md)；**日期**：2026-10-15；**Spec**：[spec.md](spec.md)。

## 摘要与技术上下文

Rust 2021 edition，当前工具链 rustc 1.99.0；现有 Cargo workspace，新增 epwflow 家族 crate。使用 clap、serde、serde_json、toml、anyhow、regex；哈希/进程控制按基础契约实现。科学计算调用现有 QE/EPW，绘图复用 plotters/svg2pdf。存储为受控文件与 JSON，不引入服务或数据库。目标首先 Linux x86_64 的 sugon-amd，macOS arm64 配置保持独立，未实测不写通过。规模是单用户的一次受限验收活动，可跨多个运行目录共享预算。

性能与容量遵循 Spec 的次数、并发、8 节点小时和 100 GiB；无未经测量的速度承诺。真实路径和输出仅存忽略目录。生产实现只用 Rust/Bash，测试可使用仓库既有 Python 测试习惯，但不形成 Python 运行依赖。

## 设计与数据所有权

目标适配器以 QE 7.4.1 配套 EPW 的真实运行版本为准。严格分开首次 Wannier/粗矩阵、epwread 重读、质量验证点、均匀细网格 ephwrite、谱函数重读。epbread/epbwrite 与 epwread/epwwrite 不是同一缓存。生成的 wdata 只来自类型化允许字段，不透传任意文本。

Wannier 输出核对解纠缠和局域化两种迭代；解析实际 .wout 的原生收敛信息、最终展宽和旋转矩阵数量。投影按轨道展开，不按字符串条数计数。窗口统一转换为 QE 电子能量参考并保存原参考/Fermi 来源，冻结态数逐 k 核对。

质量参考固定在输入快照中；至少四个粗 k 网格外点与两个粗 q 网格外点，参考通过独立 NSCF/PH 任务从 SCF 副本产生，计入共同预算。比较覆盖全部声明能窗的态，不为达到阈值删点。电子/声子误差分别计算 max/RMS，固定单位和能带/简并组排序；失败不交付下游。

细网格检查 k/q 每方向整除、实际 k+q 映射、fsthick 内态集合和矩阵记录。目标 EPW 的 ephmat 数据包含 freq、egnv、ikmap 及按并行布局划分的矩阵文件；完整角色清单依据 io_supercond 中的读取例程建立，并以读取所有记录验收，不依赖单个文件名。格式、字节序/记录标记、pool 布局写入适配身份；未知格式拒绝。

谱函数使用目标 supercond_driver 中 liso=false/laniso=false 的实际数据读取与 find_a2f 路径，不调用温度能隙求解。经验 Tc 打印不进入 Eliashberg 结果。规范表保存 meV、每个展宽与累计 λ；按照同版本离散求积复核。虚频原值保留，按 Spec 的 -1 cm^-1 边界阻断，不自动改截断。

粗数据/细数据/谱/质量报告分别有依赖哈希；新配置创建新运行并 import 相容节点，温度与 μ* 不进入无关数据哈希。独立手工输入验证首次/重读；不以重排原始矩阵强求逐字节一致。

## 源代码结构

本项新增模块 epwflow/src/interpolation/，公开入口由 epwflow/src/main.rs 统一分派；共享状态、预算与路径只由 foundation 拥有，后续模块不得复制规则。测试位于 epwflow/tests/interpolation.rs。具体 FR 到文件见下表；尚未存在的文件为明确计划触点，不作为已实现证据。

## Feature 文档结构

spec.md、plan.md、research.md、data-model.md、contracts/interface.md、tasks.md、quickstart.md、checklists/requirements.md、acceptance.md；数据/CLI 均适用，因此不省略条件文档。

## 质量门

| 来源 | 约束 | 满足方式 | 状态 |
| --- | --- | --- | --- |
| 用户确认 | 完整草案与边界 | spec.md 原文/确认记录，保留 FR/SC | 已核查，见共同分析 |
| F1–F5 | 共享契约一致 | 下游链接基础契约；单一账本与阶段状态 | 已核查，见共同分析 |
| AGENTS | 已授权 Rust/Bash、隔离提交 | 不纳入无关 Si 删除；文档完成后自动提交 | 适用 |
| 用户本地约定 | 不操作线上 Issue | 本地 issue-draft 双向链接，不伪造远程验证 | 远程门按用户约定不适用 |
| 真实验收 | 不放宽/不造证据 | acceptance 保持未运行，真实通过后更新 | 尚未执行 |

## 需求到设计与验证的映射

| 来源 | 任务 | 设计组件/接口 | 验证方式 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/interpolation/cli.rs | epwflow/tests/interpolation.rs |
| FR-002 | T005/T006 | epwflow/src/interpolation/epw_config.rs | epwflow/tests/interpolation.rs |
| FR-003 | T007/T008 | epwflow/src/interpolation/adapter.rs | epwflow/tests/interpolation.rs |
| FR-004 | T009/T010 | epwflow/src/interpolation/wannier.rs | epwflow/tests/interpolation.rs |
| FR-005 | T011/T012 | epwflow/src/interpolation/coarse.rs | epwflow/tests/interpolation.rs |
| FR-006 | T013/T014 | epwflow/src/interpolation/quality.rs | epwflow/tests/interpolation.rs |
| FR-007 | T015/T016 | epwflow/src/interpolation/fine.rs | epwflow/tests/interpolation.rs |
| FR-008 | T017/T018 | epwflow/src/interpolation/spectral.rs | epwflow/tests/interpolation.rs |
| FR-009 | T019/T020 | epwflow/src/interpolation/compare.rs | epwflow/tests/interpolation.rs |
| FR-010 | T021/T022 | epwflow/src/interpolation/reuse.rs | epwflow/tests/interpolation.rs |
| FR-011 | T023/T024 | epwflow/src/interpolation/scheduler.rs | epwflow/tests/interpolation.rs |
| FR-012 | T025/T026 | epwflow/src/interpolation/manifest.rs | epwflow/tests/interpolation.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/interpolation.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/interpolation.rs 与真实/安装证据 |

## 阶段与依赖

准备和基础协议先完成，随后按 tasks 的测试→实现→验证顺序执行。当前 Feature 完成独立验收、quickstart、Overview 回写后才推进依赖阶段；不得用尚未实现的下游编排替代本项接口验收。全部文档门通过前禁止业务代码。

## 必要复杂度

| 复杂度 | 原因 | 拒绝的简化 |
| --- | --- | --- |
| 版本化清单和依赖指纹 | 固定路径不保证固定内容，复用有真实数据风险 | 仅检查文件存在或 JOB DONE |
| 两种状态：操作与科学结果 | 提交/绘图成功不等于科学成功 | 一个完成布尔值 |
| 独立参考与固定阈值 | 相同错误流程可产生自洽假结果 | 用自己输出生成唯一期望值 |

## 2026-10-19：成对参考、投影与 DOS 设计

本段落实 FR-014/015；是待实施设计，不是已验证功能。数据由 F3 唯一拥有，0014 只收集／绘图，0015 核验真实结果。复用 qf-input 的 QE 输入、qf-crystal 的晶胞／倒空间、qf-phonon 的 q2r/matdyn 和现有绘图数据能力；EPW 版本适配、投影算符及配对校验集中在 interpolation，避免维护另一套 QE 工作流。

1. 准备时冻结电子／声子完整路径、分段与坐标单位、DOS 均匀网格、共同能量／频率轴、参考 E_F、展宽及投影定义。路径和 DOS 网格是不同数据集；原质量验证点另存，不能用少量验证点连接出完整色散。
2. 电子曲线使用直接 QE 能量与同点 Wannier 插值。共同有效窗至少覆盖 E_F±0.5 eV 和全部 Fermi 窗口；检查 DOS 展宽贡献需要的边界态。只有 Wannier 子空间覆盖该窗全部参考态时，才可标为该窗的总 DOS；不增加轨道、不自由平移或裁掉错误能带。
3. QE 电子 PDOS 读取同一 NSCF 规范下的复原子投影与原子／轨道标签。以 Wannier 旋转及解纠缠矩阵将完整投影算符变换到同一 Wannier 基底，再插值到 DOS 网格。保留非对角项和相位；只有 PDOS 平方权重、键中心 Wannier 标签或不同波函数规范的数据不足以实现原子投影。具体原生格式必须经目标版本适配和粗网格重构验证，失败时报告投影不可用。
4. 声子连续 QE 参考来自 q2r/matdyn；直接 ph.x 点独立保存、标记并继续执行原质量门。元素声子权重从同质量／原子顺序／ASR 的质量加权动力学矩阵归一化本征矢获得。禁止把 EPW 按模式输出的列按元素改名。
5. 共同 DOS 累积器处理两侧相同网格、权重、核函数与展宽，复用单位转换和表格写出。标准高斯约定明确为 exp[-(x/σ)²]/(σ√π)，若原输入为其他约定则显式换算并记录；不得无记录改变已有物理展宽。冻结覆盖范围及尾部处理，不对两条 DOS 分别缩放以制造一致。
6. 校验通过的数据写独立 comparison 清单；质量失败但数据完整时只发布诊断状态，不发布 F4 就绪。必要参考计算与存储使用 F1 既有账本；预算不足保留缺项，不能为画图启动受质量门阻断的细网格 EPC 或超导。

| 来源 | 任务 | 设计／事实来源 | 验证 |
| --- | --- | --- | --- |
| FR-014、SC-009 | T039/T040/T043 | QE 原始路径／均匀网格、EPW 原生插值、统一坐标和固定输入 | 路径／单位／零点／覆盖／来源负向测试，真实参考核查 |
| FR-015、SC-010 | T041/T042/T043 | 原子投影算符、Wannier 变换、质量加权本征矢与共同 DOS 累积 | 相位／模式误标／缺投影／积分测试，0015 真实对照 |

新数据文件与字段见 [契约](contracts/interface.md)；表格及投影实体见 [数据模型](data-model.md)。实现路径为计划触点，不以文件存在证明功能完成。

## 2026-10-19：诊断适配实施顺序

先将 comparison_data 放入 F3，严格读取带哈希清单；comparison 只解析已有原生数据并累积总 DOS。F5 依赖此模型收集／绘图；后续原子投影算符和声子本征矢生产者仍在 F3 补齐，不能在绘图层猜元素。新增 export-comparison 是只读诊断适配，不形成第二套执行器。现有受控 run/submit 尚未接通，本轮必要计算由登记预算的测试脚本完成并明确标记。


## 2026-10-19：本轮执行范围变更

原子投影算符与声子元素本征矢适配按用户最新指令暂缓，已有未完成项不勾选；恢复时仍遵守原可靠性契约。当前先完成总量叠图、声子 15/2 cm⁻¹ 质量复核、固定补充网格外点，再进入已指定细网格和原生求解。只改最大误差，RMS／虚频／覆盖／求解与资源标准不变；历史失败报告不重写。共用质量阈值与原生报告记录、边界／RMS 拒绝回归和真实续算证据见本次验收追加段。

## 2026-10-19：当前标注实现与验证计划

当前执行以 Spec 最新 50/2 cm⁻¹ 为准，先前 15/2 记录作为历史保留。F3 从现有质量成对频率计算采样／路径最大，使用倒格矢对偶基恢复晶体坐标；F5 消费带哈希标注并绘制紫点与独立诊断区，0015 复核真实模式和三格式。回归覆盖非正交转换、路径内外分类、排序模式、标注语义拒绝、50 边界及 RMS 失败。使用已有结果重绘，不新增科学作业，不越过失败质量门。

## 2026-10-19：外窗数据导出

沿用 comparison::export，Sources 可选 wannier_wout={path,sha256}。先核验日志身份并解析唯一 Outer 行，以实际绝对边界减共同 E_F 覆盖显示 window_ev；电子 CSV 仍保存全部原始带值，DOS 重新以共同核累计 501 点。metadata 增加可选 disentanglement_window_ev 绝对窗，严格校验其与 window_ev、E_F 的关系；旧包不带字段保持原显示语义。质量参考窗和状态不变。T046 对应 FR-014/SC-009。


## 2026-10-19：窗口边界的数据与绘制

F3 复用单一窗口解析器读取已核验 wout 的 Outer／Inner，将绝对边界写入比较元数据；frozen_window_ev 为可选字段，旧包无字段继续可读，但不会补造四条边界。加载时检查冻结窗包含关系。F5 只读元数据，用共同 E_F 将四边界定位到能量轴：能带水平、DOS 竖直；复用绘图函数服务独立图与总览。四个颜色的虚线和图例分别识别参数，相同能量在原坐标按像素相位交错绘制，避免遮盖。CLI 回归检查实际 SVG 的方向、位置、颜色、图例、重合和旧包；用真实 MgB₂ 只读重导出并查看三格式。

## 2026-10-19：50／20 门与原生代码收尾

对应 FR-009/012、SC-003/007。本次只调整共享 RMS 常数与边界回归。五点、45 模及相同 ASR、单位、电子窗保持不变；旧失败报告不改写。新的只读报告留在编号目录 029。

科学求解与真实验收共用原预算；9³ PH 暂缓，不新增需求或另建线上 Issue。阈值调整不表述为插值改进，真实 Tc、独立手动参考与完整自动链仍须各自提供证据。

## 原生主路径修复：F2 保存数据到细网格

落实 FR-003～009/012 的主路径缺失，沿用 F3 的唯一科学数据所有权。先核验 F2 版本 2，再冻结独立电子／PH 参考、ASR、来源、配置点序和共同 Fermi；`quality.energy_window_ev` 是相对 Fermi 的区间，来源文件保存绝对 eV。旧质量清单中的历史插值输出不作为首次执行前置。

从 F2 复制完整 NSCF outdir 和已规范化 EPW save，粗网格在一次 EPW 执行中完成内置 Wannier90。只认原生局域化／适用时解纠缠收敛、最终 WF 展宽和迭代记录，不执行原有不完整的独立 Wannier90 链。stdout／stderr 和真实退出状态先归档，再检查原生结束语义。

按目标版本源码 io/io.f90 核查 epwdata／crystal／wigner／vmedata／dmedata 和小端复数 epmatwp 的尺寸、有限值和完整性，替代旧 coarse.ephmat 简化角色。用当前粗网格 band.eig／phband.freq 派生工作目录内质量清单及来源绑定，报告未通过则停止。每次细网格调用前重验绑定与粗网格身份。

各展宽分别使用 fine/smearing-NNNN，复制同一粗网格重启数据，重新生成全细网格选择；不沿用验证点 selecq。fine 主数据绑定 spectrum_index 所选目录；其余展宽角色加 smearing_NNNN_ 前缀共同交付和核验，避免覆盖与错配。F1 完成门及 F4 执行门使用同一质量绑定和原生适配。

本次不改变 0016 选窗规则；当前源码的投影关键字能力门仍可能阻断现有安装，替代设计尚待收敛。粗／细阶段内恢复和历史粗网格导入、真实重读／超导仍单独追踪，不以进程 fixture 代替科学验收。


## 原生细网格的真实阶段验收

沿用 D01/D04/D05/D06/D11 和 T051 的已确认范围，先用生产函数在已登记分配中验收现有 LDA 6³ 粗网格重读，再调用同一个 `fine::run_fine_at` 执行 40³/20³。重读输入显式 `epwread=true, epbread=false, wannierize=false`，不触发 0016 选窗；新产生的六 k／五 q 输出经 `quality_binding` 复核后才允许细网格。细网格输入显式采用配置首个固定温度，避免 EPW 未指定温度时默认 300 K。

真实测试默认忽略；显式执行必须绑定冻结配置、粗网格全部角色、原 `.wout`／输入／输出、原 NSCF XML、赝势及实际程序哈希，验证实际 Slurm 分配和原共享账本。每次试验使用新编号目录，失败保留现场；不重复 SCF／NSCF／PH。测试只报告生产函数的阶段证据，不生成虚构 F2 来源，不宣称公开自动链的历史导入已完成。完整 F2/F3 来源契约、阶段恢复、三来源六组合仍由 T051 跟踪；不以真实测试替代这些主路径任务。

## 三程序阶段边界更新

按已确认的[三程序契约](../0008-epwflow-foundation/three-programs.md)，Wannier／粗网格与质量独立发布 wannier/manifest.json；细网格 EPC／α²F 在第三个程序执行，并另行发布 interpolation/manifest.json。阶段分离和来源不变进程回归已通过，见[0008 验收](../0008-epwflow-foundation/acceptance.md)；自动参考／四类比较图及真实完整科学验收仍未完成。

## 粗／细阶段恢复收敛

统一入口显式 --resume 才允许恢复失败的原生尝试；默认调用不重试。每个原生粗网格和细网格展宽拥有持久请求，绑定固定配置、程序、启动参数及 F2／粗清单／独立参考指纹。受控目录原先存在而无请求时拒绝。准备完成后固定全部已物化输入；成功输出记录逐文件指纹，完整输出但发布中断时只重验归集，不再启动 EPW。

失败或未完成的原生现场只在显式恢复时归档至 native-attempts/history；归档状态先持久化，再移动目录，可在归档中断后继续，未知冲突不覆盖。已成功展宽单独复用；完成后被篡改的证据直接拒绝，不通过重算掩盖。SCF／PH、已发布 Wannier 粗清单和历史失败现场保持。质量失败只复核原结果，不重新选择参数或绕过门限；预算仍由原分配统一登记。


## 缺省独立参考自动生成

T055 收敛已确认 FR-006 的主路径缺失，不改变物理输入、独立点或数据所有权。显式 reference_manifest 仍只读使用；省略时 F3 在 quality-sources 创建派生清单，配置快照保持不变。check／prepare 不执行参考；统一分配内先验证程序能力和 F2，再复用其固定 SCF 副本计算固定参考，计入同一分配，不新增阶段预算或重做 SCF。

电子参考复用 qf-input 的显式 crystal 点输入；保持原带数、赝势和阈值，核对实际坐标、完整波函数和能量。Fermi 仍来自 F2，不重新定义共同零点。PH 使用同一共享输入和原生响应校验，每个参考 q 独立保存树；已有同点原始 PH 可直接只读复用。Γ onsite 修正来自同一 F2 的原始 Γ 矩阵，按 EPW io.f90 的 dynmat_asr 处理上三角 onsite，再由 QE dyndiag 的相同 Hermitian／质量归一化对角化。原始、修正 XML、dynmat 输入／输出／退出码、执行和转换程序身份全部保留；派生 XML 仅留第一个实际 q，不能保留未重新计算的原频率尾部冒充修正结果。

复用上批 native-attempts 检查点作为共用外部进程尝试管理，退出码名和不可变输入清单按角色明确提供。QE 可写保存副本不当作执行后不变文件；其执行前来源由原 SCF 完整快照与固定请求绑定，成功输出和不可变输入另行核验。每个参考任务分别完成，失败仅显式恢复时归档重试，成功任务不重复求解。最终质量来源只在全部直接参考完整且来源仍有效时发布；粗网格仍在质量通过后交付下游。

依赖程序 dynmat.x 从已配置 PH 所在目录解析并在 F1 物化时固定身份，与 q2r／matdyn 同一目标版本；缺失或变化时明确拒绝，不引入新的公开程序或任意命令透传。完整自动图表另由 T056 收敛，不以参考清单生成代替四类图或真实 Tc 验收。

## 第二阶段自动比较实施设计

T056 的已确认范围与需求、数据、执行和验证映射见[自动比较设计](automatic-comparison.md)。代码与进程契约已验证，证据见[专项验收](../0015-epwflow-mgb2-real-validation/automatic-comparison-validation.md)；真实 PBE 全链与 Tc 验收仍未完成。
