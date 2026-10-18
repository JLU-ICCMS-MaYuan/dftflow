# 0015 验收与问题记录

**状态**：初始检查与首批失败回归已执行；生产修复及真实验收尚未开始；Issue/Spec 草案待确认。

## 已执行检查

- `git pull --ff-only --no-recurse-submodules origin master`：已是最新；HEAD 与 origin/master 均为 `62fda76621c17806f01b5831841fa44ed4239fb0`。
- `cargo test -p epwflow --locked --offline`：41 项通过（10+12+10+5+4）；未启动真实科学计算。
- 实际 CLI：submit/run 退出 1，提示科学阶段未接入；submit --until interpolation 退出 2，参数不存在。帮助检查通过不抵消运行失败。
- 三种来源各有相同的 2×2×1 dyn0、2 个不可约 q 点；SCF 日志显示收敛/JOB DONE，nosplit PH 或分 q 初始化正常结束；响应文件/保存树存在不代表 EPW 已接受。
- source inventory 记录 nosplit 13 项、index 29 项、coords 29 项文件及哈希。真实路径、CLI 输出和 sacct 查询仅保存在忽略的 docs/local/0015-epwflow-smoke/。
- 当前 squeue 查询无活动作业；近两日 sacct 有两项名字含 EPW 的 SiP2 作业，不因名称含 EPW 将其归入本次 MgB₂。未查到本次真实预算账本，历史消耗归属仍须核对，不能假定额度未使用。

## 阻断清单

| 编号/严重度 | 证据 | 影响 |
| --- | --- | --- |
| B01 HIGH | foundation/cli.rs submit/run 无条件 not_implemented，--until 缺失；snapshot.rs resume 未接入 | 一次入口不能运行 |
| B02 CRITICAL | interpolation/scheduler.rs 使用 0.5*(1-exp(-abs(w)/200)) 生成 α²F | 生成谱不来自真实电子声子矩阵元 |
| B03 CRITICAL | superconductivity/iso.rs、aniso.rs、gap.rs 使用经验 Tc/BCS，both 采用 iso Tc | 不是原 Spec 的两种原生 EPW Eliashberg 求解 |
| B04 HIGH | qe_data/reuse.rs 对 snapshot_dir、wavefunctions 用 fs::copy；仅比较复制时来源与副本哈希 | 目录不能导入，也未校验保存指纹；旧 qeflow 没有所需 manifest |
| B05 HIGH | process.rs 写 .epwflow/logs，阶段却要求 epw.out；tests fixture 自行造输出文件 | 真实 stdout 不会自动出现在阶段要求的位置 |
| B06 HIGH | foundation/budget.rs reconcile 不查 sacct，直接清 allocations/lanes | 活动作业/未知状态可被误结算，不能靠其证明资源受控 |
| B07 HIGH | interpolation/epw_config.rs 固定 prefix=scf，写 energy_reference；qe_config.rs SCF/NSCF/PH prefix 分别固定为 scf/nscf/ph | 输入与保存数据不衔接，必须按目标程序核验 |
| B08 HIGH | interpolation/scheduler.rs 无参考时跳过 quality_valid；测试使用空 F2 数据加 valid 标记 | 缺参考和数据仍可能标记可用 |

## 真实格式补充复现

通过临时 Rust 程序直接调用受测 crate 的 `qe_data::nscf::parse_xml`，输入三套来源的实际 SCF 保存 XML（同 QE 7.4.1 schema），均退出 1：`data_invalid: k 点缺少 kpoint_coord 坐标`。这证明当前 XML 格式适配失败；不是已做完 NSCF 的证据。实际坐标在 `<k_point weight="...">x y z</k_point>`，本征值为相邻 `<eigenvalues>`。

新增 B09（HIGH）：`qe_data/nscf.rs` 与真实 QE XML 不匹配，模拟测试嵌套结构掩盖了问题；修复需单位、权重、带数及网格完整性一起核对。对应回归纳入 T003/T004。

## 成功标准

| 标准 | 当前结论 |
| --- | --- |
| SC-001 | 初始审查完成；缺陷未修复，未通过 |
| SC-002 | 真实三模式 EPW 未运行 |
| SC-003 | 独立数值参考未运行 |
| SC-004 | 完整链路与资源恢复未通过 |

问题属于实测前的实现/验证缺口，不是队列尚未运行所能解释。未改变旧输入、运行目录或默认安装。

## 分工确认与实现交接

用户已确认“保持分工：其他模型按缺陷清单补齐实现，你继续测试和 debug（推荐）”。[交接清单](handoff.md) 按 B01～B09 列明修复顺序和逐批复测条件。当前仅文档交接完成，缺陷、三模式实际运行、数值对照均未因此转为通过；没有新增真实作业或修改科学实现。完整草案确认状态独立保留。

## 首批契约回归（2026-10-16）

用户“开始”后再次快进拉取，远端没有新实现提交；受测生产源码仍为 62fda76，工作树基点为 4640151。本轮新增测试，不修改生产实现。

- 默认 `cargo test -p epwflow --locked --offline`：原 41 项与新增 1 项 stdout 对照通过，8 项已知失败回归明确 ignored，不计入通过。
- 显式运行 `cargo test -p epwflow --locked --offline --test real_contracts -- --ignored --test-threads=1 --nocapture`：退出 101，0 通过、8 失败，复现结果如下。失败是待修复缺陷，不能据此声称验收通过。

| 用例 | 实际失败证据 | 对应缺陷 |
| --- | --- | --- |
| b01_run_and_submit_validate_missing_configuration | run 未加载实际配置就返回 not_implemented | B01 |
| b01_until_is_a_recognized_execution_boundary | --until 被参数解析拒绝 | B01 |
| b04_import_rejects_empty_manifest_despite_valid_label | 无任何数据、检查或角色却返回 imported | B04 |
| b05_scf_consumes_stdout_and_rejects_nonconvergence | 替身只写 stdout，阶段报缺少 scf.out，未进入收敛判断 | B05 |
| b06_unknown_scheduler_preserves_active_allocation | sacct/squeue 替身不可查询时仍清空 allocations | B06 |
| b07_qe_stages_use_bound_prefix | 配置 MgB2，SCF 实际写入 scf | B07 |
| b08_interpolation_rejects_empty_f2_manifest | 无电子态及 PH 的 F2 清单仍可绑定 F3 | B08 |
| b09_reads_qe_schema_text_coordinates_and_sibling_eigenvalues | 原生 schema 层级样本报缺少 kpoint_coord | B09 |

XML 样本根据已盘点的 QE 7.4.1 层级构造，数值为合成值，只验证格式，不是新的 MgB₂ 物理参考。预算用例通过独立 CLI 进程、隔离状态目录和不可用调度替身执行，未读写正式账本。原始输出、受测提交与测试文件哈希仅留在 `docs/local/0015-epwflow-smoke/real-contracts-known-failures.{json,log}`。

已知失败用例用 `#[ignore = "Bxx: ..."]` 明确暂挂，修复后应显式运行并取消对应 ignore；默认绿灯不能关闭缺陷。B02/B03 等完整科学来源回归仍待补齐，SC-001～004 均未因此转为通过。未运行真实 QE/EPW 或提交 Slurm，原有三套声子结果保持不动。

## 第二批回归与程序路径澄清（2026-10-16）

远端仍无新的生产修复。本轮只读核对用户后续指定的 QE 7.4.1 配套 pw.x、ph.x、epw.x：文件均存在并有执行权限。原 `.my_scriptrc.py` 仍声明旧 7.1 路径，本地算例环境已指定 7.4.1；遵循用户后续选择，不修改该配置。路径与文件检查结果保存在忽略的 `program-path-check.json`。没有执行科学程序，此检查不能替代计算节点启动、动态库或运行版本验证。

当前阻断出现在 epwflow 内部，不能归因于用户没有提供路径：公开 submit/run 在读取配置、调用外部程序前直接返回 not_implemented。即便绕过入口调用当前库函数，也存在下列可执行复现：

| 新用例 | 实际结果 | 缺陷 |
| --- | --- | --- |
| b02_rejects_interpolation_without_native_spectrum_or_quality_evidence | 无合法 F2、无原生谱/矩阵元/质量参考的简化替身仍得到 valid 插值清单，且 quality 为 None；α²F 由生产公式生成 | B02/B08 |
| b03_rejects_superconductivity_without_successful_native_solver | 配置了必定失败的 EPW 替身，生产代码未调用原生求解，仍返回 located/converged/valid 的 Tc 和能隙 | B03 |
| b08_wannier_final_state_does_not_override_nonconvergence | 合成日志明确 Convergence not reached，却因包含 Final State 和有限 Spread 返回 converged=true | B08 的数值质量门补充 |

新增样本均为检测错误放行的合成数据，绝不是 MgB₂ 结果或独立物理参考。插值用例在独立子进程使用隔离预算注册表，不修改正式账本。

当前 `real_contracts` 默认运行 1 通过、11 ignored；显式运行 ignored 用例退出 101，11 项已知失败全部复现。首批 8 项与本批 3 项分别保留记录；最新输出及测试哈希为本地 `real-contracts-second-batch.{json,log}`。ignored 不计通过，修复后取消对应标记。B01～B09 现均有相关行为回归，但树复制、指纹篡改、完整网格与未知状态恢复等覆盖尚未完成，T003 继续未完成。没有真实作业、原始数据或安装变更。

## 2026-10-16 共享 QE 输入与基础阻断修复

用户追加要求复用 qeflow；本批完成 qf-crystal 结构解析、qf-input 的 PW/PH 输入与均匀 NSCF 网格复用，以及从 0006 提取到 qf-io 的日志完成检查。epwflow 删除独立结构/输入实现，统一配置 prefix、共享原子质量，修正 PH 网格/coords 位置与 SYSTEM.nosym/noinv。共享脚本执行、完整快照和 EPW 归集尚未全部接通，不能称为完整链路可运行。

同时修复：固定输入重复 prepare 覆盖/篡改检查、Slurm 作业号严格解析、未知作业预算保留/实际用时结算/step 去重、活动分配继续占用预留、空校验清单拒绝、Wannier 否定收敛优先，以及真实 QE XML 的原生结构与 Hartree→eV 转换。预算相对路径归一化，status 不创建新账本；完整并发/提交流程仍待实现。

### 本批证据

- `bash scripts/build-native.sh test -p epwflow -p qf-input -p qf-io --locked --offline --no-fail-fast`：79 项通过、4 项明确暂挂；其中 epwflow 53 项、共享输入 8 项、共享 IO 18 项。日志 `docs/local/0015-epwflow-smoke/shared-input-tests.log`。
- `bash scripts/build-native.sh test -p qeflow --locked --offline`：27 项通过，覆盖基础 CLI、单 SCF 分 q 链、初始化、快照和后处理；日志 `qeflow-shared-regression.log`。
- `real_contracts` 中 7 项原失败回归取消 ignore，新增输入物理量/完整网格、调度解析、已绑定预算计数、配置篡改检查；默认 12 项通过。该数字包含原 stdout 对照，不代表 12 项科学验收。
- 对已有 nosplit/index/coords 三套 MgB₂ 的 SCF XML 只读调用修复后的解析器，均读出 8 个 k 点、12 带；证据 `actual-xml-parser-fixed.json`。不把 SCF 不可约点当作完整 NSCF 网格，不修改原始文件。
- 剩余 4 项暂挂回归显式运行仍全部失败（退出 101）：B01 两项入口、B02 非原生谱、B03 非原生超导。B02/B03 增加非空自述 passed 项，避免空清单防护掩盖真实缺口；日志 `remaining-contract-failures.log`。这些已验证的失败复现不计入通过数。

### 未完成与限制

B04 树复制/来源核对、B06 完整提交账本与并发、B07 输出路径和数据承接、B08 原生质量证据等仍有未完成项；空列表和输入文本修复不等于对应缺陷全部关闭。PH 快照复用、q 映射/响应势校验、完整输出网格验证、原生 Wannier/插值/超导以及真实小网格验收继续保持未完成。显式赝势卡片暂拒绝带空白路径，受控文件物化待补。

旧 F2/F3 模拟测试只说明其局部接口行为，仍包含简化输出，不是独立物理参考。本批无真实 QE/EPW 启动、Slurm 提交、安装切换或线上 Issue 操作，原共享预算未因本批重置。

## 2026-10-16：真实 MgB₂ 原生 EPW 冒烟

用户明确要求在 test_auto 做真实计算。本轮在 `test_auto/MgB2/epwflow/local_run/0015-native/` 隔离运行，真实参数、程序身份、作业号、输入/输出哈希、debug 栈及账本只留本地忽略目录。汇总证据为该目录 `audit.json`，共用手动测试账本为 `docs/local/epwflow-campaign.json`。本记录更新此前“未提交真实作业”的历史状态。

### 实际结果

| 检查 | 结果与范围 |
| --- | --- |
| 运行版本 | 计算节点实际输出 QE 7.4.1、EPW 5.9 |
| 三种 PH 来源 | nosplit、coords、核实后的 index 均原生运行正常结束；不是公开 epwflow CLI |
| NSCF | 三次实际执行，均完整 4×4×2 网格、32 点、14 带；成功版本保留 24 个晶体对称操作 |
| SCF/PH | 新执行次数均为 0，复用已有保存树和响应数据 |
| Wannier | 解纠缠、局域化分别有明确收敛日志 |
| EPC 产物 | 粗网格读取、细网格插值、500×10 原生 α²F；每列导出谱积分与原生累计 λ 符合既定积分容差 |
| 原始数据保护 | 三个不同来源树共 1020 个文件哈希及文件集合前后不变；重复盘点记录不重复计文件数 |
| 检查器负向测试 | 对副本注入非零退出、未收敛 Wannier、全零谱和缺 k 点，4 项均拒绝；源数据不变 |
| Rust 回归 | real_contracts 12 通过、4 明确 ignored；已知失败不计通过。最初缺少本机构建头文件环境的构建失败，加载既有 native 配置后通过 |

三套直接求和 λ：nosplit **0.6509531**，coords **0.6509533**，index **0.6508502**。对应第一列积分 λ 为 0.6509929、0.6509931、0.6508900。λ 单项模式一致性容差通过。

**谱一致性未通过完整验收。** 相对 nosplit，coords 与 index 的第一列同序号 L1 诊断分别约 3.94×10⁻⁶、9.71×10⁻³；能量轴最大差分别为 1.3×10⁻⁶、6.198×10⁻⁴ meV。因此同序号差仅用于定位，不冒充同轴严格对照；index 谱形差异还需查明。未做平滑、重采样或放宽容差，不能用 λ 接近替代全谱和独立参考。

### 真实失败与修复证据

九个已结算作业中六个失败、三个成功，所有现场保留：

- 首次拒绝 phonselfen 与 mp_mesh_k=true 的输入组合，改用完整均匀细 k 网格。
- 8 MPI 和单进程均复现 NSCF 无对称元数据引起的 EPW 崩溃；单进程 debugger 留栈，保留晶体对称性后越过崩溃点且实际网格完整。已修复 epwflow 的输入适配并更新 0010 FR-004，不修改科学求解器。
- coords 的 patterns.2.xml 内部仍为局部 q=1；只在测试副本按已核实映射改为全局 2，位移向量不变，随后原生 EPW 可读。
- 一次原生计算已生成结果，但测试脚本错误要求 JOB DONE 导致 Slurm 非零；重新运行以显式进程退出码、结束计时、数值产物检查获得真正成功作业，不改写旧失败状态。
- 旧 index 合并根的 Γ 响应势为零字节，原生读取报 too short；从对应 q 作业取完整响应并配套原始 patterns 后成功。没有重算 PH。

只有 NSCF 对称性兼容已修回生产输入层；其余归集与 EPW 结束验证本轮在独立测试适配中落实，生产接入列 T014，不能宣称自动链已修好。

### 预算与未完成项

本轮新增 **67 节点秒**；保守计入此前仓库相关计算的 600 秒后共 **667 节点秒**（约 0.1853 节点小时），所有本轮预留已按 Slurm 终态结算。每作业预留 900 秒，常规 8 MPI、单进程调试 1 MPI，同时至多两个活动本轮作业。新增运行数据约 1.59 GB（含保留 core），低于 100 GiB；原始来源另保留。未使用生产账本缺口绕过预算，也不把手动登记当生产预算测试通过。

严格同轴谱比较、独立 QE/PH 插值质量、原生各向同性/各向异性 Eliashberg、Tc、六组合及公开自动入口仍未完成。EPW 附带经验 Tc 不作为原生 Eliashberg 结果。本轮没有安装切换、线上 Issue 操作或推送。

## 2026-10-18：讲义参数、独立质量与错误交付阻断

### 参数依据与实际计算

按用户指定 `Wed.9.Mori.pdf` 第 16～19 页的 MgB₂ FSR 示例建立新 LDA 数据：展示输入 8³ SCF、6³/16 带 NSCF、3³ PH、5 个 Wannier 函数、B:pz 与三个键心 s、冻结窗上界 8.8 eV。正文 12³ 与展示输入不一致，记录后采用展示输入，不自动扫描。指定 NC LDA 赝势与旧 PBE 数据不同，新 SCF/PH 为必要的新绑定，不覆盖旧数据。原始讲义、机器路径、精确输入和作业号只存本地。

新增真实工作包括：

- 旧输入的同 SCF 快照 index 两点 PH 重算及原生 EPW，另有旧输入六 k/四 q 独立参考；旧输入质量未通过，后续按用户指定讲义转入隔离的新 LDA 计算。
- 新 LDA 一次 SCF、六个不可约 q 点 PH、216 点/16 带 NSCF、原生 Wannier/粗网格 EPC。因 MPI pool 不相容和超长路径两次失败后复用已完成数据，仅重试 EPW，不再重做新 SCF/PH。
- 六个固定 k/四个固定 q 的独立 QE 参考；PH 完成后对派生副本施加与 EPW 相同的 Γ onsite ASR，由真实 dynmat 对角化。原始 PH 不改写。
- 原生 EPW 重读在全 q 选择下生成诊断频率。未越过质量门生成 40³/20³ 矩阵元，未运行 Eliashberg。

### 数值结果与判断

| 项目 | 实测 | 标准及结论 |
| --- | --- | --- |
| 旧 index 同 SCF 来源后的谱形诊断 | 第一列同序号 L1 约 7.46×10⁻⁶；峰值约 8.88×10⁻⁶ | 显著降低原差异；能量轴仍不同约 1.2×10⁻⁶ meV，严格同轴验收未通过 |
| LDA 电子质量 | 最大 0.013027809、RMS 0.008787088 eV | 当前六点/能窗指标满足 0.05/0.02 eV；不代表整个费米面覆盖 |
| LDA 声子质量 | 最大 48.530593309、RMS 10.610753468 cm⁻¹ | **失败**，原定标准为 5/2 cm⁻¹ |
| 粗网格点核对 | 相同 ASR 下频率差小于 0.001 cm⁻¹ | 网格外点主导误差，保留全部九模和符号 |
| 最大误差位置 | q=(1/6,0,1/6)，排序第八模：569.397344 与 617.927937 cm⁻¹ | 保留逐模式 CSV，不删点、不换能窗、不放宽容差 |
| Rust 真实目录检查 | JSON 为 quality_failed，进程退出 1 | 与独立数值计算一致，明确阻断下游，workflow_complete=false |

本轮电子能窗为均匀 NSCF 的 EF±0.5 eV；参考共有四个能窗内态，仅一个网格外 k 点落入该窗。因此不能把“当前指标通过”扩展成完整电子结构或材料收敛结论。研究与源文件身份见本目录 Research 及本地 `mori-quality-01/quality-reference.json`、`quality-check.stdout.json`、`phonon-comparison.csv`；ASR 参考见 `mori-quality-check-01/`。

### 生产代码与回归

- 共享 QE 输入支持显式原子质量；EPW 按结构元素顺序绑定 amass，不将 B 的 10.811 改成默认 10.81。
- 原生 EPW 输入区分粗网格与细网格，正确使用 nkf/nqf、能量单位、能窗与重读开关；检查短 dvscf_dir 和 Fortran 字符串。全 q 诊断不改变 Fermi 窗口。
- 原生谱直接解析真实能量轴、α²F、累计 λ、积分/求和 λ；完成判定使用进程退出与 EPW 原生结束计时，不要求 JOB DONE。
- 独立电子匹配一一覆盖参考态、不自由平移；声子比较覆盖 3N 模、保持虚频符号。清单读取重新核对指纹，超差结果仍输出完整诊断，不能发布成功清单。
- F3 删除公式造谱，并在细网格之前要求当前粗网格的有效质量来源。F4 移除经验 Tc/BCS 的调度发布路径；原生求解未接通时明确报错。这是防止错误结果，不是原生 F4 已实现。
- 受测原生构建环境下 `cargo test -p epwflow -p qeflow --locked --offline --no-fail-fast` 共 **95 通过、2 ignored**：epwflow 68、qeflow 27；包含 12 个原生输出/质量/输入用例和 real_contracts 15 个通过。B02/B03 已取消 ignore；剩余两项 B01 仍不能被计入通过。旧经验模型的局部数学测试也不能当作 Eliashberg 验收。

测试使用现有 Linux 配置和 native 头文件环境，完整命令/输出为本地 `docs/local/0015-mori-regression.log`。本次构建的是源码测试制品，没有覆盖默认安装。

### 预算、原数据与未完成项

本轮新增加 **348 节点秒**（含旧输入诊断、新 LDA 及独立参考），原保守累计 667 秒之后共 **1015 节点秒**，约 0.282 节点小时；全部作业已按 Slurm 分配终态结算，当前预留为零。节点启动失败、EPC 输入失败和参考均计入；没有为新输入重置 8 节点小时额度。三个独立历史来源共 1020 文件的集合与内容再次核对不变；新增原生运行目录占用约 2.88 GB，详细数据占用和哈希只留忽略目录。

严格同轴谱、讲义输入的声子质量、完整 F3 原生矩阵元/重读适配、原生 iso/aniso 的 Δ/Z/Tc、公开 submit/run/--until 与六组合验收仍未完成。0015 SC-003 已实际发现失败，不能再写成“仅未运行”；整体不关闭。后续不以讲义的教学残差 10⁻³ 替代原 10⁻⁸，也不默认追加物理参数扫描。
