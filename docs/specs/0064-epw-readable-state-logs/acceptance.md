# 0064 实施与验收记录

首轮实现获得用户“开始落地”的授权，以下“验证结果”至“证据边界”记录该轮受控回归。2026-10-28 用户追加全仓库构建发布、清除指定 MgB₂ 旧现场并真实重算的授权，新增执行结果单列于末尾。首轮验证使用临时目录、公开 CLI、实际 shell 脚本和受控 QE／EPW／Slurm 替身，不运行真实科学计算、不切换安装。

## 验证结果

2026-10-28，本地源码实现与受控回归完成。全量扫描后对变更及失败项定向复验，共 343 项唯一用例通过、14 项保持原有忽略条件。此数去除了重复执行，不代表一次全量命令全部成功。

使用构建包装器编译，固定测试二进制后执行进程测试，期间没有重建其可执行文件。

- 全量命令 `CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --no-fail-fast -- --test-threads=12` 的记录为 `/tmp/0064-all-tests-final1.log`：340 通过、1 失败、14 忽略。唯一失败是 Wannier 负向测试仍精确断言“未收敛”，实际已正确拒绝并报告“缺少原生 Wannier 局域化收敛证据”。已修正这条断言；不将该次全量命令记成成功。
- 最后两项生产修改是补查日志丢失时尚存的 `.jobs`，以及 Tc 日志中文状态和只从 `.jobs` 恢复作业号。再次整体编译成功，记录 `/tmp/0064-compile10.log`。
- 重编译后的全部单元回归：55 通过、0 失败、3 忽略，记录 `/tmp/0064-unit-tests-final2.log`。新增 Tc 状态读写测试实际恢复搜索次数、中文状态和运行中温度点，故意给旧内存对象不同作业号，验证恢复值来自 `.jobs`。
- 末轮公开进程回归：10 通过、0 失败，耗时 733.17 秒，记录 `/tmp/0064-process-tests-final2.log`。包含全部 8 项新协议用例、上述 Wannier 失败恢复，以及完整上游到双方法 Tc 搜索。原生任务和整阶段分配在日志丢失、作业活动／查询失败时均零覆盖、零重提，终态后可按当前输入重新准备。Tc 在多次公开调用间从中文日志和 `.jobs` 接续，双方法均完成搜索及必需能隙核验。

测试中的 14 项忽略均保持原条件，忽略不计通过；此前调试轮次不重复累计为验收数量。具体原因如下：

| 用例 | 忽略原因 |
| --- | --- |
| `delivery::decay::tests::real_bound_native_decay_files_render_without_scientific_calls` | 需要显式指定真实粗网格回执及全新派生输出目录；只读科学数据 |
| `foundation::phonon_import::tests::historical_electronic_binding_diagnostic` | 仅供显式指定旧配置的只读接续诊断 |
| `interpolation::electronic_sampling::tests::real_workspace_wdata_readonly` | 显式提供既有 MgB2 配置，只读检查 wdata，不运行科学程序 |
| `real_sip2_all_text_stars_convert_without_loss` | 需要本地已复制 SiP2 save；仅解析，不运行科学程序 |
| `real_electronic_dual_windows_and_pdos` | 需要指定真实 PBE 工作区、新目录及 Slurm 分配；只执行必要 projwfc 后处理 |
| `actual_lda_coarse_restart_and_wannier` | 依赖本机忽略的真实 LDA 粗网格，只读核验须显式指定 |
| `actual_sip2_fft_evidence_is_read_only_and_rejected` | 需要显式指定 EPWFLOW_SIP2_EVIDENCE_ROOT；只读历史 XML，不执行计算 |
| `actual_lda_native_layout` | 需要 EPWFLOW_REAL_QE_ROOT 指向已有 LDA 证据，显式执行只读核验 |
| `existing_real_qe_data_plot_without_scientific_execution` | 需要显式指定已有真实工作区和新的图像输出目录 |
| `actual_lda_fine_matrices` | 真实 40³/20³ EPC；须显式配置、预留原共享预算并在 Slurm 中执行 |
| `actual_lda_fine_readback` | 只读重验已经终止的真实 EPC；不启动科学程序 |
| `actual_lda_native_nonlinear_diagnostic` | 单次固定非线性诊断；须用户确认、历史次数与原共享预算 |
| `actual_lda_native_tc` | 真实两模式 Tc；须冻结来源、预留原共享预算并在 Slurm 中显式执行 |
| `numbered_real_legacy_pw_is_read_only` | 需显式设置本机 MgB2 配置；只读验证，不提交或执行科学程序 |

## 成功标准与证据对应

| 标准 | 主要验证入口 |
| --- | --- |
| SC-001～003：文件集合、可读性与职责 | `readable_0064_*`、`eight_human_logs_and_jobs_survive_independent_updates`；核对八日志、imports 例外和编号目录不存在共享 manifest；人工查看测试现场实际日志 |
| SC-004：原文配置版本 | `raw_config_versions_capture_a_a_b_a_without_expanding_toml`、`readable_0064_versions_preserve_raw_text_and_old_missing_input_never_blocks_prepare`；字节、数量和不覆盖断言 |
| SC-005：悬空旧输入恢复 | 上述公开配置／旧输入测试、`failed_wannier_generation_error_recovers_without_archived_inputs`、旧布局单元回归 |
| SC-006：细分续算 | PH 链、电子／PDOS、`matdyn_recovery_reuses_successful_force_constants_and_native_coarse`、多展宽与双方法多温度回归；直接计数科学调用 |
| SC-007：调度与科学状态分离 | 活动／未知提交、`numbered_deleted_stage_never_bypasses_queued_or_unknown_submission`、输出失败／缺失／未收敛与多温度数据无效回归 |
| SC-008：外部导入 | `readable_0064_external_phonon_import_keeps_only_imports_and_never_runs_pw_or_ph`、外部声子导入公开链；来源文件集合及字节、零新增科学调用 |
| SC-009：覆盖与上游保留 | `numbered_stage_archive_preserves_the_open_slurm_log`、`numbered_native_retry_preserves_open_logs_and_ignores_legacy_log_inputs_only`、SCF 显式重算／粗网格重试 |
| SC-010：真实依赖检查 | NSCF 网格、FFT、PH 响应数据、UKK／CHK、比较坐标损坏及上游参数变化回归 |
| SC-011：旧布局退出 | `interrupted_cleanup_keeps_layout_until_retry_succeeds`、`numbered_legacy_migration_preserves_history_budget_and_reuses_without_hidden_tree`、活动迁移拒绝 |
| SC-012：并发与不完整状态 | 独立日志并发更新、半写拒绝、快速阶段作业结束／回执竞态、多方法第二次提交未知；另验证日志丢失后仍核查 `.jobs` |
| SC-013：交付与清理 | `clear_history`、QE 单阶段图、阶段 results、实空间衰减图、移动结果包重绘与来源检查 |
| SC-014：证据及文档 | 本记录、Overview／README／示例和本项设计产物；源码验证、默认安装与科学验收分开描述 |

仓库所在文件系统另外做过目录 `flock` 实验：第二进程非阻塞取锁被拒绝，持锁进程退出后可取得，目录内没有新增锁文件。八日志写入锁定稳定目录 inode，日志本身原子替换不改变锁对象。

本地 Markdown 链接已核查 693 项，无缺失；`git diff --check` 通过。完整命令和新布局检查入口见 [快速说明](quickstart.md)。

## 证据边界

[Tasks](tasks.md) 的本地实现、验证和文档任务已完成，SC-001～014 的证据如上。正常编号运行不再读写被删除的状态树；旧路径只用于固定范围的兼容退出和旧历史清理。科学结果包及其原生数据记录继续保留，不作为另一套调度状态。

源码、受控进程测试、安装和材料科学结果是不同证据。本次没有新增真实 QE／EPW、Slurm 科学提交或材料科学验收结论，没有转换现用 SiP₂／MgB₂ 科学现场，没有更新默认安装或推送。线上 Issue 及远程链接仍由用户管理。原有无关未提交修改按任务开始基线保留。

## 追加真实重算（2026-10-28）

本轮固定已发布的新 CLI，从清空后的指定工作区完成全部八阶段。最后一次公开 `--stage superconductivity -s` 返回 `stage_complete: true`、`workflow_complete: true`，测试驱动退出 0。独立结果包及布局检查也退出 0：iso／aniso 的 20／30／40 K 共六点全部收敛且数据有效。执行证据位于忽略目录 `docs/local/0064-real-validation/`。

已完成全工作区 `bash scripts/build-native.sh build --release --workspace --locked`，编译及发布退出 0，本轮发布九个产品（qeflow、vaspflow、scflow、wannier90、substitution 和四个 EPW CLI）；另执行 `bash scripts/build-native.sh build --manifest-path tools/qf-gen-dyn0/Cargo.toml --release --locked`，独立工具编译与发布退出 0。共十份产品发布哈希均与安装文件一致；补建独立工具不替换正在测试的 EPW 程序。通过 `scripts/env.sh` 解析的三个 EPW 入口来自仓库 `bin/epwflow/`。

在队列为空、旧记录及共享账本中作业全部终态后，清除指定工作区八个计算目录、`epw-state/` 和 `results/`，未归档旧科学数据。输入 TOML、KPATH、README、POSCAR 与两份 UPF 共六个保留文件逐字节校验一致；README 随后按本轮行为另行更新。Parastor 对一棵无文件目录骨架持续返回 `ENOTEMPTY`，将其移到忽略证据目录 `empty-filesystem-remnants/` 后，十个原路径均确认不存在再启动新计算；不将空骨架称为已物理删除。

共享账本未重置；先通过公开 CLI 结算此前已完成的 1382723，补记 220 节点秒后，本轮科学重算前累计为 11585 节点秒。

### 已观察到的数值与性能边界

本轮输入为 PBE 小网格功能测试，不搜索 Tc，也不扫描网格或自动调窗。实际 Wannier 原生输出在 582 次局域化迭代满足收敛条件；阶段完成不等于插值精度收敛。新生成比较报告的电子路径有序能量配对最大差为 0.6891326649965119 eV，位置 K=(1/3,1/3,0)；声子路径最大差为 251.9238 cm⁻¹，位置 (0,0,0.425)，DOS 网格最大差为 232.7245 cm⁻¹，位置 (0,0,0.5)。未放宽数值、改网格或隐藏这些点。

`results/6.epw-coarse/comparison-summary.json` 保持 `comparison_complete=false`，明确缺少复数原子投影适配器支持的电子元素／轨道比较；第 3 步独立 QE 元素 PDOS 已实际生成，二者不混称。实际查看了 QE DOS 与粗网格四面板 PNG。

公开 CLI 的重复依赖核验仍有明显性能开销。本轮粗网格从首次调用到最终核验用时约 18 分钟；其中四个 Slurm 科学／比较子任务分别只用 5、3、2、3 节点秒。进程观察到多次读取上游波函数，单次调用逻辑读取达到约 19.8 GB（`/proc/PID/io` 的 `rchar`；不能当作实际磁盘流量，`read_bytes=0`）。本轮未改动此性能行为，不将完成八日志改造写成已解决重复读取性能问题。

本轮细网格原生谱文件 `7.epw-fine/smearing-0001/MgB2.a2f` 含 500 个频率点，声子展宽 0.5 meV；末端积分 λ=1.0810779，原生求和 λ=1.0810327。该数值只记录本输入实际结果，不代替网格收敛检验。

### 作业清单与用量

本轮 13 个独立 Slurm 分配均为 `COMPLETED`、退出码 `0:0`，各使用一个节点。原始 `sacct` 记录在 `docs/local/0064-real-validation/all-jobs-accounting.txt`。阶段内的多个同步子任务共用父分配，不重复计费。

| 作业号 | 实际任务 | 节点秒 |
| --- | --- | --- |
| 1384600 | SCF | 14 |
| 1384602 | NSCF | 4 |
| 1384603 | QE 电子能带、DOS、PDOS 及发布 | 50 |
| 1384606 | coords PH 初始化、两 q 点、q2r／matdyn 及发布 | 130 |
| 1384616 | EPW Wannier | 5 |
| 1384617 | postw90 电子 DOS | 1 |
| 1384621 | EPW 粗网格 | 5 |
| 1384626 | 比较专用 q2r | 3 |
| 1384631 | 比较专用 matdyn | 2 |
| 1384638 | EPW 声子比较求值 | 3 |
| 1384657 | EPW 细网格 | 3 |
| 1384668 | iso 20／30／40 K | 3 |
| 1384669 | aniso 20／30／40 K | 5 |

新增科学用量合计 228 节点秒；共享账本累计 11813 节点秒，活动分配、预留和占用并发数均为零。此前作业补结算的 220 节点秒单独计入历史，未误算为本轮重算。

两种方法的原生输出都显示三个温度正常达到迭代收敛。iso 在 20／30／40 K 分别为 113／131／145 次，aniso 为 126／145／198 次；六份新能隙文件均通过最后公开调用及独立结果包检查。

### 最终交付与协议核查

- 根 `results/` 检查返回 `operation_status=ok`、`stage_results_verified=true`；其 `workflow_complete=false` 仅表示阶段导航包不宣告全流程完成。
- 第 8 步结果包检查返回 `native_complete=true`、`native_figures_verified=true`、`workflow_complete=true`、`source_verification=verified`；`gap_run.execution_complete` 和 `data_complete` 均为 true，六点分别为 `converged=true`、`data_status=valid`。
- 实际 `epw-state/` 恰有八份中文阶段日志，全部为科学已完成，无旧 JSON 或状态子目录；每个计算目录恰有一份同名 `.jobs`，作业号与 Slurm 终态记录一致。未创建新的 requests、native-jobs、history 等共享状态树。
- 全部公开调用后，配置原文仍只有一份逐字节一致的 `0000-config.toml`；原 TOML、KPATH、POSCAR、两份赝势哈希与清理前一致。十个已发布产品哈希再次核对一致；科学测试期间三个 EPW 二进制未重建或替换。
- 结果包包含 25 张有效 PNG，均检查文件头与尺寸；实际查看 QE 元素 DOS、QE／EPW 四面板、原生能隙与谱总览。数值限制仍以上节为准。
- 最终核查用最后一次公开阶段调用的全流程完成结论，加根／第 8 步两个独立结果包检查；没有额外重复整条昂贵状态扫描。证据文件为 `real-chain.log`、`final-root-results.json`、`final-native-results.json`、`final-summary.json` 和 `final-verification.log`。

本轮从首次 SCF 调用到全流程完成约 48 分 33 秒，最后一次超导归集调用约 8 分 4 秒。实际 Slurm 科学分配合计仅 228 节点秒，重复依赖读取／后处理检查的性能开销仍未解决，不把本次功能通过写成性能通过。当前小网格不是 Tc、选窗或网格收敛验收。

总览按实际发布和真实验收事实增量同步；当前未提供专用 explore／understand-explain 分析 Skill，以源码搜索、文件读取和已完成运行证据完成同等只读分析。保留首轮受控回归和 0059 历史记录，未修改其他材料计算现场或线上 Issue，不推送。

文档核查：本项设计、总览与示例共 145 个本地 Markdown 链接可达；任务开始时已有的七份无关脏文件逐字节保持不变。仅提交本轮九份文档，不纳入原始科学数据、构建产物或共享账本。

## SiP₂ 迁移命令补充文档（2026-10-28）

按用户批注在 `examples/SiP2/README.md` 末尾追加正式 `--migrate-state` 命令、无提交选项、删除与保留范围、转换后的八日志检查及失败处理。明确“撤下”为删除旧状态及历史，不移动、不自动备份；历史内科学副本随历史删除，当前计算目录、results、声子来源及 imports 保留。没有重写上文历史内容；旧 README 是修改后文件逐字节一致的前缀。

对照 `program::dispatch`、`readable_state::register_config` 与 `legacy_state::finish` 核查真实入口和删除范围。修改所涉文档共 30 个 Bash 代码块通过 `bash -n`，31 个本地链接目标存在，记录在忽略目录 `docs/local/0061-eight-stage-layout-20261028/migration-doc-checks.txt`。只做文档和语法核验，不运行转换、科学程序或 Slurm 提交，不宣称真实 SiP₂ 已转换。

用户追加的八主阶段 TOML 命名由 0061 继续澄清；当前用户要求先列重名字段的实际例子，尚未确认子段方案。本次文档提交不修改配置语法、CLI、实际 TOML 或科学数值，不触及线上 Issue 或推送。

## SiP₂ 旧能带赝势目录兼容修复（2026-10-28）

### 原因与处理

旧 `3.qe-electronic/qe_eband/eband.in` 的 `CONTROL.pseudo_dir` 和 QE XML 仍记录已删除的 `epw-state/attempts/.../pp`。原比较包含目录字符串，使已保存有效结果被判输入冲突。本次保持原始输入和 XML 字节不变，仅在完整输入比较时允许该目录字段位置不同；实际保存 XML 的元素／赝势文件映射与配置一致，保存的 UPF 和当前配置逐个 SHA-256 相等，才能接续。字符串内的空格、注释符与斜杠均保留语义，不能忽略其他原生参数、结构卡或路径变化。

`--migrate-state` 增加已转换电子阶段的显式重验：检查活动／未知任务、历史配置、上游科学状态及批次，复用已有子任务验证和完成登记；不运行输入生成器或科学程序，不提交 Slurm。职责分别归入输入兼容检查和迁移编排，避免第二套完成标记和额外持久状态。

### 受控回归

- release 库测试：61 通过、0 失败、4 项既有真实数据测试忽略；新增引号／注释／空格、重复字段、截断能和 k 路径区别检查。
- 相关公开进程：八日志、电子／Wannier／粗网格恢复及 NSCF 契约共 23 个用例。首次 22 通过、1 失败：失败发生在 `Case::numbered` 读取临时 `inputepw.toml` 的构造阶段，返回文件不存在，尚未进入产品检查。该用例独立复跑五种失败场景全部通过；保留初次失败和复测日志，不将初次失败算作通过，不推断未证实的文件系统原因。
- 新增迁移回归验证旧目录不存在时完成重验；保存的赝势缺失／内容改变、截断能、k 路径及依赖批次不符时拒绝。恢复证据后可以重验成功；科学调用、后处理调用、提交记录和原电子目录内容保持不变。

### 正式安装

使用 `scripts/build-native.sh build --release -p epwflow --bins --locked`，覆盖 `bin/epwflow/` 四个入口。实际命令加载 `scripts/env.sh`，解析至仓库正式 `epw_wannier`，帮助信息及四份发布清单的源文件／发布文件 SHA-256 均核对一致。

`epw_wannier` 构建产物 SHA-256：`925feb73cb8c2035b168e7881f5a0c63ae6ec1b2986eb0f306059860c64f92a1`；正式发布 SHA-256：`e89149e59443d6935e0792d381aab781af4b195783456d4a0f5b0578abe63abc`。构建来源为 `bdb9b96` 加本任务源码补丁；安装后未在真实重验期间替换程序。

### SiP₂ 实际无计算验收

在 `examples/SiP2` 使用正式入口执行 `epw_wannier --config "epwinput.toml" --migrate-state`，返回 0，输出“第 3 步旧电子结果已重新核验；科学文件未改写，未提交计算”。第 3 步日志的阶段／科学状态均为“已完成”，问题分区已清除；仍使用 `converted-electronic` 批次和 `0000-config.toml` 计算配置。重复同一命令返回 0，不重复核算或提交。

- 现场 1784 个文件集合保持不变；除八份阶段日志外，1776 个文件的尺寸和修改时间逐项一致。
- 另比较配置、KPATH、科学输入、XML、UPF、输出日志和能带／DOS 表格等 42 份选定文件，SHA-256 全部一致。未对所有大文件额外执行前后全量哈希；复用检查自身仍按原有规则读取保存树和哈希证据。
- 原 `eband.in`／XML 保留旧路径，旧 `attempts/` 未重建；Si/P 保存的 UPF 均与当前配置赝势相同。现有 SCF、NSCF、PH 和电子结果均未重算，未执行新 Wannier 或更下游任务。
- 科学配置 `epwinput.toml` 的 SHA-256 仍为 `fcfa97159645a8c60a15fe82e28e1625341d6d7d1b4fc85345b1716bdb41ff95`；按配置原文版本规则新增逐字节相同的 `0001-config.toml`，原 `0000-config.toml` 不改。八日志更新当前输入版本，第 3 步登记完成，imports 不变。

真实重验存在多次读取保存树的既有性能开销，未将其作为本轮优化。源码／回归、正式安装和现场验证证据留在忽略的 `docs/local/0064-sip2-pseudo-relocation-20261028/`；包含构建／测试日志、两次公开调用输出、发布身份、前后文件清单及最终核验 JSON。README 与 Quickstart 共 30 个 Bash 块语法通过，修改涉及文档的 133 个本地链接目标存在。不操作线上 Issue、不推送。

## 追加验收：配置原文集中存入 epw-state（2026-10-28）

用户最新要求覆盖此前“在输入旁保存”的位置约定，沿用本项 FR-031～032／SC-016、T024～T027。原始需求完整保存在 Issue 与 Spec，既有引文及历史验收保留。

### 行为与实现

`foundation/config_versions.rs` 集中处理原文字节、最大编号比较、来源解析及旧快照迁入，既有 `backup_config` 统一委托；使用状态目录锁和原子落盘，不增加独立命令、JSON 或来源旁文件。A/A/B/A 保存三份，不覆盖旧版。每版原输入目录和原文 SHA-256 写第 1 步日志，普通及受控嵌套加载器共用解析基准，SCF 重算保留记录。

旧版本先核验归属、目标和作业终态，独立复制、更新阶段／子任务引用后再撤下旧位置；冲突或未知任务保留原文件。隐藏状态的显式迁移在旧任务／锁检查后才建立新状态。整体日志丢失只在唯一当前版本、其他日志来源绑定和任务终态证据完整时恢复，不猜测历史来源。

预存来源日志不等于编号布局，旧非编号初始化允许仅含已核验快照的目标，未知内容继续拒绝；`--work` 旧现场查询沿用固定绑定，不读取无关的外部输入。职责按快照存储、配置解析、布局识别与旧式初始化拆分，复用现有日志和锁，避免第二套科学状态或配置生成器。

### 回归证据

- 首轮公开进程 98 通过、5 失败、1 项本机数据专用忽略；失败定位为预存快照触发旧初始化冲突、隐藏状态检查时机及来源日志丢失，均保留初次证据并修复，不将失败计为通过。
- 固定兼容构建完整复跑：公开进程 **103 通过、0 失败、1 忽略**。覆盖三声子模式、原始声子导入、电子／Wannier／粗细与超导续算、冻结版本、已删除目录、活动／未知提交及图表恢复。
- 随后细化旧式 `--work` 的无关输入隔离和基础检查断言，最终安装候选复测：库测试 **65 通过、4 项真实数据专用忽略**；基础测试 **15 通过**；跨目录相对路径与排队版本冻结、丢失阶段日志的任务号保护、旧工作区导入 **3 项定向进程复测通过**。
- 配置单元覆盖八线程同文写入、A/A/B/A、原文损坏拒绝、外置及含空格输入目录、旧版冲突／未知提交拒绝、复制后中断恢复和全部阶段引用更新。基础检查的断言按新需求改为仅允许原文及来源登记落盘，仍要求零科学调用；无效输入不提前创建工作区。

构建和测试通过 `scripts/build-native.sh test --release -p epwflow --lib --test three_stage_execution --test foundation --no-run` 固定二进制后执行，进程测试运行期间不覆盖其二进制。证据留于忽略的 `docs/local/0064-config-snapshots-in-state-20261028/`，不提交本机计算数据。

### 正式编译与安装

使用 `scripts/build-native.sh build --release -p epwflow --bins --locked`，覆盖仓库 `bin/epwflow/` 四个正式入口。加载 `scripts/env.sh` 后，四个实际命令路径、帮助信息及发布清单的原始构建文件／发布文件 SHA-256 均核对通过。构建来源为 `caa3fa1` 加本轮源码补丁；十份实现文件的哈希保存在本轮本地证据中，安装及现场验收期间不再修改实现或替换二进制。

`epw_wannier` 原始构建 SHA-256：`15831e7d82b65ef9719929a4af333fe7f7b9382b803ca45d58f1436f43e1a4c2`；正式发布 SHA-256：`8ac82cc5abf3f046a223ab6bc8e6454c9925319b2966cc0dc29cb1e594615cba`。四份完整身份及帮助信息见本轮证据的 `release-verification.json` 和对应帮助文本。

### SiP₂ 实际迁移与幂等验收

加载正式环境，在 `examples/SiP2` 两次执行 `epw_wannier --config "epwinput.toml" --migrate-state`，均返回 0。原根目录 `0000-config.toml`、`0001-config.toml` 独立迁入 `epw-state/`，根目录旧位置已移除；编号和原文字节不变，目标不是软链接或硬链接。当前可编辑输入保持原位。

- 迁移前后文件总数均为 1922；授权修改的 README 单列，1892 个输入／科学文件及 19 个 imports 文件的尺寸、修改时间、文件类型逐项不变。另核对 1120 份选定文件的 SHA-256，包含原输入、两份迁移快照、结构、路径及编号计算目录中的科学输入／XML／输出日志，全部一致；不宣称对其余大文件执行了额外全量哈希。
- 八日志的既有字段逐项核对，只允许配置位置和更新时间变化；新增两份来源记录。阶段科学状态、批次、依赖均保留，第 3 步仍为已完成及 `converted-electronic`，计算配置改指 `epw-state/0000-config.toml`；当前输入版本为 `epw-state/0001-config.toml`。
- 重复调用未新增 `0002-config.toml`；十个顶层状态文件的 SHA-256 和修改时间与首次迁移结束时一致。迁移前后 Slurm 队列为空，本轮没有 `-s/-r` 科学调用，没有重算 SCF、NSCF、PH、电子参考或 Wannier。
- 版本 0000 SHA-256 保持 `deba4df68f6e1ffa480d402806cc4ae576b3c0bf04428e66793d61d7853df59e`；版本 0001 与现用输入保持 `fcfa97159645a8c60a15fe82e28e1625341d6d7d1b4fc85345b1716bdb41ff95`。第 1 步来源日志记录原解析目录为 SiP₂ 根目录。

原始证据、首轮失败及复测、最终编译／发布清单、两次实际调用、文件身份与幂等核验 JSON 均保留于本轮本地证据目录。Documentation Impact 已落实到本项全部设计／任务／操作文档、根及模块 README、SiP₂ README 和三份相关 Overview；文档 Bash 语法及本地链接核验通过。线上 Gitee 仍由用户管理，不推送。

## 2026-11-16：全部程序输出英文

- 用户已确认包括运行提示、阶段状态文件、帮助及报错，覆盖此前中文设计。当前输出、生成说明与图表文案统一为英文；所有复用标记由编号阶段命名函数生成，例如 reuse: 3.qe-electronic /projwfc。文档与普通注释继续使用简体中文。
- 旧中文阶段日志、任务号表头、具名子任务与依赖字段在读取边界规范化；原始配置、路径、批次与作业号不改写。读取不改变文件，正常更新写英文。截断、混合别名重复和未知提交仍被拒绝。旧中文状态实际进程回归未增加科学调用；八阶段日志均检查英文内容。
- 内部推导说明参与既有科学身份，保留原来的确定性身份值，不因翻译触发重算。历史绑定夹具验证真实配置生成的 SCF 与细网格内部绑定；旧参数来源注释的读取同样保留兼容。
- 最新 EPW release 单元回归 73 通过、4 项既有忽略；英文公开帮助和缺失／非法配置回归 2 通过；完整公开进程回归 111 通过、1 项既有忽略。其他共享库单元回归共 57 通过。按不同用例计为 243 通过、5 项既有忽略，不重复累计复跑。
- 文案修订后的 NSCF 网格负向／复用回归 3 通过，qf-io 回归 17 通过，均已包含在上述不同用例范围内。旧检查点固定引用与动态子任务标题、依赖字段保持相同英文拼写；首轮发现的陈旧文本匹配均已修复，没有放宽数值或失败判据。
- 全部相关 Rust 文件通过语法与面向输出字符串清查；兼容输入的词条与标记集中在兼容模块。实际公开入口的帮助及缺失输入错误均为英文；四份已安装程序与验证制品逐字节一致。
- 源码以 b7ad94ff 加本任务补丁在独立检出验证与构建；共享工作区未完成的绘图修改未混入制品。已覆盖固定的 epw_phonon、epw_wannier、epw_superconductivity、epw_clear，不生成版本入口或旧命令备份，不推送。
- 本轮仅使用受控科学替身验证执行、续算和输出契约，没有提交真实 QE／EPW 作业，也没有改写用户计算现场。不以替身回归宣称新的科学数值验收。详细日志和安装校验位于忽略的 docs/local/0064-english-output/。

复查命令：使用构建包装器执行 release 的 epwflow --lib、english_output 和 three_stage_execution 回归；共享库使用各自 --lib 回归。复用、未知提交与科学损坏的正反向场景均包含在实际进程测试中。T028～T031 对应 FR-033～035 与 SC-017，以上结果作为完成依据。
