# 0056 验收记录

## 实现与行为

公共 `archive_step` 保留阶段根目录，逐项归档旧内容；当前 Slurm 日志原位保留并通过硬链接建立历史入口。目的路径冲突和文件操作失败返回源／目标上下文，阻断新科学程序；既有失效登记及相容上游复用不变。

## 测试证据

- `CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --lib archive_tests -- --nocapture`：5 项通过。真实独立 Bash 进程持有 cwd，归档后看到新输入；目录 inode、历史字节、打开日志持续写入、链接不跟随、目标冲突与非法目标均验证。
- 扩展回归已观察到 library 44 项通过／2 项显式忽略、native_interpolation 13 项通过／1 项显式忽略、real_fine 2 项显式忽略。three_stage_execution 中新增的两个归档公开进程用例及 SCF 强制重算、原生恢复用例通过；该批最终汇总输出未留存，不据此声明整批全绿。
- 扩展批次中 `numbered_three_ph_modes_deliver_verified_save_and_local_scripts` 曾在科学调用前失败，stderr 为 `epw_phonon (deleted)`，无法按原路径定位入口。原始 stderr／stdout 留于忽略的 `docs/local/0057-history-cleanup/initial-three-ph-modes.*`。稳定制品后单独复测结果在下文补录，不弱化身份校验、不把重测计作首次全绿。

## 证据边界

测试使用受控科学 IO／Slurm，不提交真实 QE／EPW 作业，不改变科学参数。真实工作区清理和默认安装交付由依赖本项的 0057 记录。本项不自行重算现有 MgB₂ 阶段以验证 inode；稳定性由真实临时文件和独立驻留进程证明。

## 显式忽略的六项真实用例

这些用例均要求另行提供真实现场或科学预算，不计为通过，也不为本次文件归档改动启动计算。

| 用例 | 原因 |
| --- | --- |
| `delivery::decay::tests::real_bound_native_decay_files_render_without_scientific_calls` | 需要显式真实 coarse 回执和新的派生产物输出位置 |
| `interpolation::electronic_sampling::tests::real_workspace_wdata_readonly` | 需要显式真实 MgB₂ 配置 |
| `actual_lda_coarse_restart_and_wannier` | 历史 LDA 来源需显式配置；当前需求不恢复 LDA |
| `actual_lda_fine_matrices` | 要求真实 40³／20³ EPC 的科学预算与 Slurm，不属于本次任务 |
| `actual_lda_fine_readback` | 需要显式历史真实 EPC 只读现场 |
| `numbered_real_legacy_pw_is_read_only` | 需要显式本地 MgB₂ 配置 |

## 最终定向复测与制品

- 稳定构建后单独执行 `numbered_three_ph_modes_deliver_verified_save_and_local_scripts --exact`：1 项通过，0 失败，79 项过滤。日志 `retest-three-ph-modes.log`；此次通过不覆盖首次失败事实。
- 同一组已构建的真实 CLI／测试制品独立执行 `numbered_stage_archive_preserves_the_open_slurm_log`、`numbered_archive_failure_does_not_start_scientific_programs` 和 `direct_submit::explicit_scf_resume_recomputes_success_each_time_and_blocks_running_jobs`：分别 1 项通过，共 3 项通过。日志 `final-archive-process-tests.log`。
- 以上忽略日志路径均位于 `docs/local/0057-history-cleanup/`。验证使用 native 构建入口生成的 debug 制品；默认 release 安装与新清理 CLI 随 0057 交付，不能把 debug 契约结果当作已安装版本证明。
- 0056 本地文档链接、需求映射及任务 diff 检查通过，根 README、模块手册和 Overview 已同步。线上 Issue 保持用户自管，本地实现完成不等于平台关闭。
