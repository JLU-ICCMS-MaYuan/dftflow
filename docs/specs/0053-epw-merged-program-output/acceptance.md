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

## 后续：显式 SCF 强制重跑与默认安装

用户明确选择“修改 --resume：即使 SCF 已成功，每次执行也强制重新计算”。本次将编号工作区的 `--stage scf --resume -s` 改为创建新批次；无需用户删除 `1.qe-scf` 或完成回执。旧目录由程序归档至 `epw-state/history/`，当前路径重新生成脚本。普通 `--stage scf -s` 归集及复用，其他阶段和依赖恢复不增加强制重算。活动／未知任务仍先阻断。提交输出及 `native-plan.json` 提醒 SCF 归集时不带 `--resume`。

### 定向与完整回归

新增两个公开 CLI／真实 Bash 回归：

- `failed_version_two_scf_resumes_with_merged_script_and_preserves_old_directory`：不手工删除旧目录，将版本 2 失败任务恢复为版本 3，旧脚本及 stdout／stderr 完整归档，新脚本仍在原路径，成功归集不依赖 `.err`。
- `explicit_scf_resume_recomputes_success_each_time_and_blocks_running_jobs`：完成回执与快照仍在时，连续两次显式恢复均重新提交、产生新批次；原回执／快照完整归档；运行中再次恢复拒绝；普通 `-s` 完成归集并保持复用。另核验提交及待办提示明确说明不带 `--resume` 归集。

共享 QE 与基础库回归 63 通过、0 失败。EPW 最终完整回归及提示调整后的定向复验见下方最终统计。两项新增用例计入完整回归，不与单独执行次数重复相加。

第一轮测试过程中重新编译覆盖了公开进程用例所绑定的可执行文件，出现已删除可执行文件路径的身份拒绝；该轮已停止，退出 143，不计为完整回归通过。构建稳定后另起完整回归，日志为 `docs/local/0053-epw-merged-program-output/retest/final-tests.log`。后续仅调整归集提示，待完整测试进程结束后再编译并定向复验，避免运行中的制品再次被覆盖。

### 真实 MgB₂ SCF

在原 `examples/MgB2/sugon-amd/run/epwflow` 执行用户指定命令：

```bash
epw_phonon -c "epwinput.toml" --stage scf --resume -s
```

- 新作业 **1344407**：1 节点、32 MPI，`COMPLETED / 0:0`，实际 9 节点秒；`scf.out` 有 `JOB DONE.`。
- `1.qe-scf/scf.sh` 已重新生成，科学命令使用 `> scf.out 2>&1`；`bash -n` 通过，无新科学 `scf.err`。
- 旧阶段归档到 `epw-state/history/scf-18e11313eecf2f47-7d314-c/calculation/`；运行前登记的 **18 份文件全部 SHA256 不变**。没有手工删除阶段或改写旧任务版本。
- 新任务版本 3，stdout／stderr 绑定同一 `scf.out`。用不带 `--resume` 的 `--stage scf -s` 归集后，任务为 `validated`，新 `completion.json` 的 **30 份产物逐一大小与 SHA256 核验通过**。
- 旧作业 1343700 的 4 节点秒在本次首次归集时补结算，新作业新增 9 节点秒；账本由 5124 到 **5137／28800 节点秒**，预留归零。未提交 PH、Wannier、EPC 或超导作业。

仓库 `bin/epwflow`、固定 `current/bin` 及默认用户入口的三个 EPW 程序已更新并核对制品指纹、解析路径和 `--help`。使用既有运行库，不覆盖其他模块及共享依赖。原安装备份、前后清单、现场哈希、调度证据和测试日志仅保存在忽略的 `docs/local/0053-epw-merged-program-output/retest/`。

### 后续回归统计

完整 EPW 回归进程正常跑完全部测试目标，退出 **101**：**250 通过、1 失败、10 忽略**。唯一失败为 `multitemperature::batch_wrong_temperature_log_and_active_upstream_change_are_rejected`，在准备上游时拒绝定位带 `(deleted)` 后缀的测试 `epw_superconductivity` 路径；后续检查该路径的程序仍存在。未为此放宽程序身份核验，具体环境成因未确定。

同一用例使用既有测试可执行文件单独复验 **1 通过、0 失败**（`wrong-temperature-diagnose.log`），因此 EPW 已覆盖 **251 项成功用例**，但不将这一合计冒充单次全量命令退出 0。10 项未执行用例的名称及条件沿用本文前述清单。共享 QE 与基础库单次完整回归 **63 通过、0 失败**。

归集提示调整后，最终源码的三项 SCF 定向回归 **3 通过、0 失败**（`final-scf-resume.log`）；全量失败的同一用例用最终测试程序再次复验 **1 通过、0 失败**（`final-failed-case-retest.log`），不重复计入 251 项覆盖。随后三个 release 程序重新编译成功；最终源码指纹、打包制品与安装路径记录于 `final-source-identities.json`、`package-final/` 及 `installation-final.json`。真实 SCF 只执行上述一次，最后仅刷新归集提示与运行制品，不重复提交科学作业。
