# 0064 实施与验收记录

本项已获得用户“开始落地”的授权。验证使用临时目录、公开 CLI、实际 shell 脚本和受控 QE／EPW／Slurm 替身，不运行真实科学计算、不切换安装。

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
