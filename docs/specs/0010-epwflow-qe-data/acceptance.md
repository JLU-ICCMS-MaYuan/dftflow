# 验收记录：QE 三模式数据准备

**状态**：共享 qeflow nosplit 执行、数据保留、完整 NSCF 交接和失败恢复通过进程回归；尚未完成真实科学全链验收。下列初始表和旧记录保留历史语境，当前证据见最后追加段。

| 标准 | 结论 | 证据 |
| --- | --- | --- |
| SC-001 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-002 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-003 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-004 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-005 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-006 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-007 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-008 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |

## 共同约束

所有真实程序探测、参考、失败与重试共用 Epic 授权额度；实际作业、路径和输出仅存 docs/local/epwflow-acceptance/。无真实证据不关闭本项。

## 实施验收回写（2026-10-16）

- **状态**：F1–F5 全部五个模块族已实现并通过隔离行为测试。
- **隔离测试证据**：`cargo test -p epwflow` 全绿，合计 41 个测试（foundation 10 + qe_data 12 + interpolation 10 + superconductivity 5 + delivery 4），使用临时目录与可执行 fixture，不启动真实 QE、不触碰真实队列、不纳入 `external/` 子模块。
- **FR/SC 覆盖**：64 条 FR 与 39 条 SC 均已有对应源文件、集成测试与结构化清单产出；未运行项在 F5 六组合验收表中以 `status = "not_run"` 诚实标注，未编造证据。
- **真实小网格验收**：本机 `external/q-e/build/bin` 具备真实 pw.x/ph.x/epw.x/wannier90.x/pw2wannier90.x；完整六组合端到端真实验收本次未执行（本环境缺准备好的赝势集合与计算节点输入，且工作区其它 crate 因系统依赖 spglib/llvm-config 无法整体构建）。epwflow 单 crate 在本机完全构建并通过测试。
- **后续**：在具备赝势与 Slurm/计算节点的环境，按统一预算上限（8 节点小时、100 GiB）逐项执行真实验收，输出写入 `docs/local/epwflow-acceptance/`（gitignored）。

## 2026-10-16 独立复核修正

同一源码 62fda76 的 41 项现有测试可复现通过，但前文“全部模块族已实现”及“64 FR/39 SC 已覆盖”的声明不能据测试数量成立。实际 CLI 不能启动科学链路，数据格式/产物来源/预算存在已复现阻断；详见 [0015 独立测试记录](../0015-epwflow-mgb2-real-validation/acceptance.md)。本 Feature 仍未完成正式完整验收，不能据此前回写关闭或宣称真实结果正确。

## 2026-10-16 收敛进展

已复用 qf-crystal/qf-input/qf-io 的结构、输入和日志检查；真实 SCF XML 三模式只读解析通过。快照、归集、输出网格和真实 EPW 消费仍待完成。完整命令、数量与限制见 [0015 本批记录](../0015-epwflow-mgb2-real-validation/acceptance.md)。不关闭本 Feature，不以模拟回归替代真实验收。

## 保存树与原生 PH 交接修复

已修复实际 prefix、完整 SCF 快照、NSCF／PH 独立副本、实际 dyn0 和非正交 q、原生 PH XML／扰动基底／响应势尺寸及 EPW save 交接。新交付为版本 2 清单，F1／F3 和导入重验角色及实际数据；旧版仅历史读取。三模式独立进程通过，并对已有真实 LDA 216 k／16 带／28 q／每 q 9 模只读核验通过，88 文件哈希不变。证据及限制见 [交接验收](../0015-epwflow-mgb2-real-validation/native-qe-handoff-validation.md)。

本批没有运行新的 SCF／PH 或最小 EPW 读取，仍缺真实全链、阶段内失败恢复及导入自动复用；不关闭 Issue 或勾选整体真实验收。共享预算 2241 节点秒、预留 0，无新增科学作业。

完整 epwflow／qeflow 回归 130 项通过、0 失败；另有 1 项机器数据测试默认暂挂，已单独显式运行通过。测试日志和逐项范围见上述交接验收，不据测试数量认定真实全链完成。

## 2026-10-19：三程序整合的共享声子前置

本批完成的是 qeflow 可复用库、nosplit 后处理修复与可选 EPW 原始数据归档。完整需求和剩余任务见[三程序契约](../0008-epwflow-foundation/three-programs.md)。三个 EPW 程序尚未交付，EPW 严格导入适配、第二阶段四类图与第三阶段细网格／两模式 Tc 仍待完成，不能据此关闭需求。

- 原 qeflow 声子主体提取为 `qeflow/src/lib.rs`，既有二进制调用同一实现；公开库接口 `run_qe_phonon_nosplit` 可供 EPW 直接调用。本批尚未把 EPW 旧调度器切换到该接口。
- `[qe_phonon].retain_epw_data` 默认 false；启用时 PH 与 q2r 使用一致的 XML 名称，逐 q 保存 dvscf。qf-phonon 归档 dyn0、逐 q dyn／dvscf 和 phsave，保存 SHA-256；缺失、歧义、冲突、篡改及归档期间来源变化拒绝发布或复用。归档清单不冒充 EPW 的物理和模式验证。
- 修复 nosplit 本地跳过 q2r／matdyn、提交脚本重复后处理；本地科学步骤核对退出及结束日志。未适配的分 q／单独初始化不允许启用保留选项。
- 完整基线运行 `bash scripts/build-native.sh test -p qeflow -p epwflow -p qf-input -p qf-phonon --locked --offline`：156 项通过，6 项真实环境测试继续暂挂。随后补充提交脚本和未适配模式用例，并对最终代码运行 `bash scripts/build-native.sh test -p qeflow -p qf-input -p qf-phonon --locked --offline`：47 项通过（qeflow 32、共享输入 8、声子库 7）。EPW 基线 111 项通过；替身结果不计真实科学验收。
- 用户指定 local_run 下新建 `002-three-stage-cli`，复制 PBE 结构与路径，生成带保留选项的 qeflow 输入与声子脚本；核查 24 MPI、1 节点、2 小时上限、XML／逐 q 选项与 Bash 语法。只做准备，没有 ph.out 或新科学作业；原配置、结构、KPATH 和赝势哈希保持。
- 本批没有修改共享预算或默认安装，没有运行旧替代诊断或线上 Issue 操作；继续保持真实 Tc 与整体三程序验收未完成。

## 共享声子执行链验证

`epw_phonon` 已切换到 `qeflow::run_nosplit_with_scf_checkpoint`，共享 SCF→PH→q2r→matdyn 和声子图实现，不启动第二次调度提交。SCF 成功、PH 之前保存独立快照，PH 工作副本与 EPW 完整均匀 NSCF 副本分别拥有可写数据。原有三模式内部接口继续用于历史回归，公开首条路径只要求 nosplit。

- 固定 KPATH 与 q2r／matdyn 身份，生成受控 qeflow 配置、程序封装和赝势副本；显式原子质量、SCF 位移、物理阈值传入共享输入。归档之后仍核验原生 q 映射、响应势／扰动基底及完整电子态，发布 EPW 严格清单。
- 请求、脚本及 SCF 检查点持久化并记录指纹。q2r 失败后恢复重用 SCF／PH；实际调用序列为 `pw ph q2r q2r matdyn matdyn pw`。SCF 日志、输入、快照和受控请求改变均拒绝。NSCF 失败现场另存，原 SCF 不重复计算。
- 独立进程覆盖成功、SCF 能量错误、响应势截断、后处理失败恢复与脚本篡改；成功链为 `pw ph q2r matdyn matdyn pw`，并核查归档、声子图和交付身份。
- 完整验证命令：`bash scripts/build-native.sh test -p epwflow -p qeflow -p qf-input -p qf-config -p qf-phonon --locked --offline`；**169 通过、0 失败、6 项真实环境测试暂挂**。替身结果仅证明接口和拒绝行为，不证明数值正确或真实 Tc。
- 在用户指定 `test_auto/MgB2/local_run/003-shared-phonon` 复制 PBE 配置／结构／KPATH，实际调用三个程序生成阶段脚本，通过 `bash -n`；固定清单含 KPATH、q2r、matdyn，六份原来源哈希保持。没有生成 calculations、预留预算或调用科学程序。具体机器日志留忽略目录。

独立质量参考与四类比较图自动生成、选窗外部程序兼容、真实两模式完整 Tc 仍未完成。没有新增科学作业、重置预算或切换默认安装；本地 Issue 保持未完成。

## 赝势卡片与 XML 状态衔接的收敛验证

沿用 0006 T027／0010 T102：显式绝对赝势路径通过共享 QE 字符串序列化输出，严格进程替身拒绝未引用斜杠；q2r 完成状态要求成功日志与非空文本或 XML 力常数。matdyn 失败后恢复的定向用例验证 SCF／PH／q2r 不重复，q2r 原日志和力常数指纹不变。

分批共 **92 个不同用例最终通过、0 项未解决失败、2 项既有真实环境暂挂**：epwflow 库 11、QE 数据 18、原生插值 16、工作流契约 26、完整公开链 1、qeflow 库 1、EPW 保留 5、声子链 8、qf-state 6。完整公开链 90.79 秒，继续通过 33 份图、部分超导／最终交付恢复、移动重绘及篡改拒绝。原始 handoff-regression.log 保留测试错误调用已发布 F2 的拒绝；改用 matdyn 中断场景后通过。handoff-retest.log 保留旧未引用路径字面断言的失败，更新为有效引用契约后 handoff-final.log 通过。不重复累计用例，不把协议数值当作真实 Tc。

证据位于 local_run/010-public-pbe 的上述日志与 handoff-verified-cases；真实累计 3009 节点秒、活动预留为零，历史来源与复制输入均不变。真实 NSCF／完整第一阶段仍待后续固定方案续算核验，不据此关闭整体需求或切换默认安装。

## QE 原生文件名长度适配

1296305 暴露带引号绝对路径仍被 atom_pfile 的 80 字节字段截断。T102 改为共同 pseudo_dir 与带引号的短相对名，保持原显式文件与 SHA-256；超过原生限制直接拒绝。定向测试覆盖不同赝势子目录、超长文件名拒绝、SCF／完整与参考 NSCF 同一生成路径，严格替身按实际目录与卡片定位文件。

最终 short-pseudo-verified.log 一次通过 95 项、0 失败、2 项真实环境暂挂；计数与完整公开入口结果见 [0008 验收](../0008-epwflow-foundation/acceptance.md)。此前未变化的 qeflow 库 1、EPW 保留 5、声子链 8、qf-state 6 另已通过；不重复累计。1296305 的真实 14 份科学文件／save 指纹证明已完成声子步骤复用，完整 NSCF 仍须固定方案续算确认，T102 保持未勾选。

随后使用提交 102fc8b 冻结的兼容程序原参数续算：1296651 COMPLETED、退出 0，新增 21 节点秒，共享累计 3055。完整 NSCF 为 32 k／16 带，原生 JOB DONE；公开 --check 返回版本 2 清单 valid，SCF、完整态、2 个不可约 q、响应势与产物完整性检查全部通过。此前 14 份科学文件／save 指纹、四份来源与复制输入、原预算历史均保持，预留归零。T102 据此勾选；第一条 PBE nosplit 交接通过，不据此关闭完整 F2／三阶段或 Tc 需求。


## 2026-10-27 外部声子导入

本轮已批准需求、澄清与实施边界见 [完整补充](external-phonon-request.md)。沿用 0010，整合 0048、0060、0061；真实验证及发布证据在本轮完成后追加。


### 代码回归与正式发布

2026-10-27：外部 save 的 Rust 导入、0048 单分配后处理接入及 0061 受控配置接续已实现；0060 配置/NSCF 实际输入输出/FFT 校验纳入本次集成。原始记录、FR-013～017、SC-009～011 与 T103～07 对应。

- 最终基础/配置/原生交付七套回归合计 **83 通过、3 默认忽略**：flat_stage_config 7、foundation 15、native_interpolation 13、native_outputs 10、native_stage_config 12、nscf_grid_contract 8、qe_data 18。忽略分别为本机 LDA 粗网格、LDA QE 来源、历史 SiP₂ FFT 的定向环境测试；不是通过，本轮不以它们代替新工作区核验。
- dynmat_text **6 通过**，包含显式执行的真实 SiP₂ 16 份文本转换（总 36 个 q 星成员、每份 72 模式），以及复矩阵列序、非正交晶胞、非零 ibrav、极性张量、截断/非有限/错序拒绝。
- 声子相关公开进程 **19 项均有通过证据**，含 5 项外部导入、已有单分配 PH 自动链、QE 声子交付/补图。覆盖无科学重算导入、单提交、幂等、失败显式恢复、坏来源拒绝、参数/电子证据改变拒绝、旧格式接续及模拟事务中断恢复。主批次 18 通过、1 失败；失败发生在夹具初始化读取配置路径时报告 ENOENT、尚未执行科学流程，现场保留。该项 `phonon_chain_plot_failure_keeps_all_science_for_one_resume` 独立完整复测通过；不把本批次写成一次全过。
- 上述去重 **108 项相关回归通过**。更早的 89 项整套命令被主动中断，不能计作全套通过；没有执行整个仓库全部测试。科学替身只验证流程与 IO，不作为真实科学验收。
- `scripts/build-native.sh build --release --locked -p epwflow --bins` 成功，覆盖正式 `bin/epwflow/` 的 epw_phonon/epw_wannier/epw_superconductivity/epw_clear。加载 env 后两个实际科学入口均指向正式目录；帮助含 `--import-phonon`；四个发布记录的 source/published SHA256 均与实际制品一致。
- 构建基线为 1330b64 加本次集成工作树，包含已授权整合的待提交 0060 前置实现；不声称当时工作树已提交。构建源码逐文件清单、制品身份、帮助、完整回归日志、初次失败和复测均保存在忽略目录 `docs/local/0010-external-phonon/`。

### 工程约束

复用现有调度、阶段回执、原生验证和绘图链，避免建立第二套科学执行器；内容转换独立于归集，旧配置兼容限定在显式导入，来源完整身份参与复用。新数据文件独立复制，旧配置/回执保存事务历史，恢复只补未完成步骤。用户免验的历史信息明确记录为未检查，不伪称完整历史物理一致性。AGENTS.md 等无关既有修改不纳入本任务提交。

补充隔离回归：外部导入 5 项在增加来源/目标重叠拒绝后再次通过；独立复制的损坏、硬链接、软链接拒绝 1 项通过；跨目录调用使用配置相对来源的公开单提交/补图用例再测通过。去重合计 109 项相关回归通过。加固补丁将在真实导入作业结束后发布，避免运行时替换制品。


### SiP₂ 正式导入与出图验收

- 在 `examples/SiP2` 使用正式 `epw_phonon --config "epwinput.toml" --import-phonon "external-phonon/save" -s`，单作业 **1379059**，COMPLETED/0，14 分 37 秒；父尝试 `18e2511ba4ed9aa0-16af37-10` 为 succeeded，声子完成回执及 QE manifest 已发布。q2r 的完整网格检查为 36 点、FFT 检查成功；色散/DOS 两次 matdyn 均有 JOB DONE。
- 当前电子数据严格通过：SCF/NSCF 36 点网格设置一致，NSCF 输入/输出完整 36 点，密度 FFT 216×128×60、平滑 FFT 120×72×32 一致。导入仅校验电子证据，不运行 SCF/NSCF/PH/PH 初始化。
- 文本 16 份含完整 36 个 q 星成员，control 重建 dyn0；响应势、phsave 完整归集。QE manifest 1175 个产物角色、16 个不可约 q 及响应；来源报告与清单绑定。
- 原 1074 个来源文件、30,854,600,669 字节，导入后独立重新读取全部哈希及文件集合，均未改变。电子原回执中的 126 项产物（SCF 46、NSCF 80）逐文件重验 SHA256/大小且输出记录完全一致。16 份新 dvscf 均 nlink=1，和来源的 inode 不同。
- SiP₂ 配置已升级为平层版本 2，SCF/NSCF 输出身份不变。原配置、旧回执及迁移证据位于 `epw-state/history/flat-import-18e2511ad6af6d3d-16af37-2/`。新键为 SCF d55d18ee9ced723fec53aa10a01e72a8f5f157dfa0cc0e5c9373f71c958ac22a、NSCF 0bdc98631b9c0b8d3e7e2d1f6ff81b043d94e663b325159f23f71c151229d89d；这是核验后的新绑定，旧回执保留，并未直接伪改原键。
- 实际查看 `plots/phband/phonon_band.png`、`plots/phdos/phonon_dos.png`、`plots/phonon_combined.jpg`，包含声子谱、TDOS、Si/P 元素 DOS。元素表 200 行，Si+P 与 TDOS 最大绝对差约 5×10⁻⁶（表格打印精度）；不以图像重合或投影和代替网格收敛验收。正式图表及表格在 `results/4.qe-phonon-nosplit/`。
- matdyn 原始日志保留了非厄米差及缺 Z* 的提示；没有修剪原始数据或把本次导入称为动力学稳定性、EPC/超导及网格收敛验收。历史赝势/截断能及缺失的历史 smooth FFT 仍列为未核验。
- 首次实测安装版本随后只增加路径重叠/独立文件复制加固；作业结束后完整发布最终 release，制品、帮助、env PATH 和源码逐文件清单再次核验。两次构建及发布身份均保留，真实声子交付未因更新而重算。最终正式 epw_phonon SHA256 为 71c42f9f18ba0b73a444f77a702943c11d2e63b89f77415e7fda04fa3dfc7464，epw_wannier 为 8b0d40ded57cb8cf6a2db7c47dcdfcffd57a41d367b6823f1e649be97c0da073。


### 实现节点提交时的未完成边界

代码和导入科学交付已通过上述验证，按任务范围先保存实现节点。真实 Wannier 作业 **1379339** 已通过正式 `epw_wannier --config "epwinput.toml" --stage wannier -s` 提交，当前运行尚缺的第三阶段 244 点电子路径能带；尚未形成 Wannier 收敛或阶段交付结论，T109 保持未完成。原始 PW 日志含少量本征值未收敛提示，将保留并在最终验收记录中区分。最终 NSCF 错误建议的平层路径修正通过 8 项契约回归；为避免运行时替换，待此作业结束再发布该提示修正，并重新核验正式制品。

### 来源标记丢失保护的最终回归

新增拒绝场景先复现了缺陷：删除来源绑定、来源报告和 QE manifest 后，PH 完成回执未包含来源报告身份，后续准备没有拒绝。修复为将来源报告显式纳入 PH 完成回执，并从残留回执/清单反查导入身份；残留记录损坏时保守拒绝，不能回退 PH。随后同一最终源码的声子相关公开进程 **19 通过、0 失败、0 忽略**，包括来源标记丢失拒绝、单分配导入、失败恢复、旧配置接续和只补图；日志 `phonon-final-failclosed.log`。这替代此前分批声子回归作为最终证据，去重总数仍为 **109**，不重复累计。真实活动作业运行期间未替换安装，最终加固版待作业结束发布。
