# 0053：验收记录

## 结果与边界

2026-10-23，本地实现与技术验收完成。EPW 全套回归进程退出 0：**249 通过、0 失败、10 项既有条件用例未执行**。三个 release CLI 重新编译，`--help` 均退出 0。未更新仓库 `bin/epwflow/` 或默认安装，未提交真实科学作业、切换分支、推送或操作线上 Issue；无关 `AGENTS.md` 修改保留。

## 需求与证据

| 标准 | 验证结果 |
| --- | --- |
| SC-001 | `direct_submit::stage_allocations_complete_upstream_once_then_both_native_solvers` 通过：实际生成并执行 pw、ph、q2r、matdyn、bands、dos、postw90、EPW 脚本，逐份 `bash -n` 通过，原生任务均为版本 3。 |
| SC-002 | `scientific_stdout_is_live_and_arguments_are_quoted_without_shell_expansion` 通过：进程仍在运行时 stdout／stderr 已同时出现在 `.out`，没有 `.err`；特殊路径不展开，退出 17 保持。 |
| SC-003 | 上述全阶段用例核验每个原生任务 `.out` 同时包含两路输出；单温度超导和两方法多温度收尾通过。`batch_default_mu_and_explicit_value_use_method_range_directories` 明确核验无 `.err` 且日志身份一致。声子链、归档、失败／提前停止恢复、未知提交和篡改拒绝回归通过。 |
| SC-004 | `legacy_task_logs_are_adopted_before_new_format_next_task`、`legacy_version_two_keeps_separate_logs_and_rejects_missing_stderr` 通过；版本 1／2 保持原读取契约，缺少旧 `.err` 拒绝，不自动创建。5 份现场脚本前后 SHA256 一致。 |

开发时的全阶段回归暴露电子回执对 `eband.err` 的强制依赖，已按当前原生任务版本调整产物登记；旧回执与旧日志归档集合保持原样，最终全套回归包含修正后的验证。

Slurm 日志位置的 6 项专项用例全部通过，现有 `--output`／`--error` 指令行为保持。普通本地执行器仍保留原分离日志，合并针对本需求的原生 Slurm 科学脚本。

## 现场脚本清点

清点 `examples/` 和 `test_auto/` 中路径含 `epwflow/` 的全部现存 `.sh`，共 5 份：

- `examples/MgB2/sugon-amd/run/epwflow/1.qe-scf/scf.sh`：被原生任务记录绑定，作业号 1343700，保留。
- `examples/MgB2/sugon-amd/run/epwflow/epw-state/history/scf-18e10fce1ce433e9-3ecde1-2/calculation/scf.sh`：历史冻结副本，保留。
- `test_auto/MgB2/epwflow/submit.sh`：测试入口，无科学命令分离重定向，无需迁移。
- `test_auto/MgB2/epwflow/replot.sh`：重绘入口，无科学命令分离重定向，无需迁移。
- `test_auto/MgB2/epwflow/replot_window.sh`：重绘入口，无科学命令分离重定向，无需迁移。

**可迁移目标为 0，未改写任何现场脚本。** 两份科学脚本 SHA256 均仍为 `5181295c6407bcfc499783826e8578ab736546988af728d7e39e72beecf3970a`。新格式由后续新任务正常生成，不重捕获哈希冒充历史绑定。

## 验证命令与编译产物

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --no-fail-fast
bash scripts/build-native.sh build --release --locked -p epwflow --bins
```

最终 249 项通过来自同一次完整测试运行，包含定向回归，不与开发阶段用例数重复相加。公开进程测试组为 75 通过、1 忽略；整组正常退出，未中断。构建有既有未使用代码与依赖未来兼容性警告，无编译错误。

产物位于 `target/linux-x86_64/x86_64-unknown-linux-gnu/release/`：`epw_phonon`、`epw_wannier`、`epw_superconductivity`。三程序均已实际运行 `--help`；这是启动检查，不代替真实科学验收，也不表示默认安装已更新。

本机完整日志、统计、CLI 帮助及产物 SHA256 保存在忽略的 `docs/local/0053-epw-merged-program-output/`：`tests.log`、`tests-summary.json`、`build.log`、`build-artifacts.json`、`scripts-before.json` 和 `scripts-verification.json`。

## 10 项未执行用例

以下是既有条件用例，未计为通过，不作为 0053 新增科学验收义务：

- `delivery::decay::tests::real_bound_native_decay_files_render_without_scientific_calls`：需显式指定真实粗网格回执及新的派生输出目录。
- `interpolation::electronic_sampling::tests::real_workspace_wdata_readonly`：需显式提供既有 MgB₂ 配置进行只读核验。
- `actual_lda_coarse_restart_and_wannier`：依赖本机忽略的真实 LDA 粗网格来源。
- `actual_lda_native_layout`：需 `EPWFLOW_REAL_QE_ROOT` 指向既有 LDA 证据。
- `existing_real_qe_data_plot_without_scientific_execution`：需指定真实工作区及新的图像目录。
- `actual_lda_fine_matrices`：真实 40³／20³ EPC，需冻结配置、共享预算及 Slurm 分配。
- `actual_lda_fine_readback`：需指定已经终止的真实 EPC 证据。
- `actual_lda_native_nonlinear_diagnostic`：固定非线性诊断，需单独授权、历史次数与共享预算。
- `actual_lda_native_tc`：真实两模式 Tc，需冻结来源、共享预算及显式 Slurm 执行。
- `numbered_real_legacy_pw_is_read_only`：需显式设置本机 MgB₂ 配置，只读验证旧 PW。

## 文档与关闭边界

规格、设计、数据契约、任务、模块 README、MgB₂ 说明和 Overview 已同步。保留线上远程链接待核验项，不宣称线上 Issue 已创建或关闭。根 README 没有新增操作入口，无需修改。按明确路径提交本任务，编译产物与本机证据不纳入 Git。
