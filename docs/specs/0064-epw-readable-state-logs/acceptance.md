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
