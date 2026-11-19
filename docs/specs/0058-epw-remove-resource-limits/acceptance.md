# 验收记录

日期：2026-10-26。实现及 SC-001～SC-005 隔离验收完成；未部署，未进行真实科学验收。

不执行真实科学计算、不操作线上 Issue、不自动部署或推送。后续逐项记录命令、结果、忽略用例和历史保护证据。

## 首轮定位记录

首次全量回归识别并修正三类问题：

- 旧字段填入超过 TOML 整数范围的值时，解析错误缺少配置文件路径；已为语法解析和字段解析统一补充文件上下文。
- 两项旧 real_contracts 测试仍传入已删除的 CLI 参数；同步接口，并将累计时长额度断言改为原有并发约束断言。
- HEAD 的 Cargo.toml 已包含 0057 新增的 epw_clear，但 three_programs 测试仍断言只有三个二进制；修正为三个阶段程序加独立清理工具，不修改其他生产功能。

初轮识别上述四项失败后，主动终止仍使用旧构建的测试进程组，退出 143；该轮未完成，不计完整通过。日志位于忽略的 docs/local/0058-resource-limits/epwflow-tests.log。停止全部该轮进程后才重新构建，随后在共享工作树复测，日志为 epwflow-tests-final.log。

## 共享工作树隔离

第二轮共享工作树复测退出 101：另一项 0059 的配置重构正在修改 Config／RuntimeConfig 和接口模板，中间状态出现 validate_native 缺失及可变借用编译错误。未覆盖、修复或纳入该项改动。

最终验证从基线 98dc02c328889ad82e4d1fd80f3c96b1748b0c70 建立临时 git archive 副本，仅加入 0058 专属修改；config.rs、mod.rs 和接口模板均从基线重建本任务差量。构建缓存使用独立副本，测试过程中不重新构建同一套正在执行的 EPW 二进制。最终日志为忽略的 docs/local/0058-resource-limits/epwflow-tests-isolated.log。隔离结果不能用来宣称共享目录中未提交的 0059 已通过验证。

共享 QE 检查：CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p qeflow --locked，34 项通过、0 失败、0 忽略，退出 0。日志为 docs/local/0058-resource-limits/qeflow-tests.log。

## 需求与文档质量核对

用户已完整给出本版方案并要求实施，未追加需求访谈或未获要求的限制。逐字原文、严格拒绝选择及后续实施指令分别保存在 spec.md、issue-draft.md 和 original-implementation-request.md。原 Slurm 头部的行尾空白为原文的一部分，有意保留。

FR-001～006、SC-001～005 已全部映射到 Plan 的职责／任务／验证表；Resources 是配置事实来源，budget.rs 负责身份、原有并发及历史实际计量，三个脚本生成路径统一取消时间注入，结果摘要复用同一账本状态。没有新增无限额开关、宽松配置分支或第二份账本。

Spec、Plan、Research、数据模型、CLI／Slurm 契约与 Tasks 核对无范围或兼容性冲突，无未决高影响问题。需求不涉及新 UI、图片、外部网络服务或平台权限变更，未建立相关空白设计产物。代码修改以删除额度逻辑为主，保留现有严格解析、登记及摘要接口，避免重复实现。文档中的本地相对文件链接已核验；线上类型／状态与远程 Spec 链接未经发布和核验，仍由用户管理，本地完成不代表线上关闭。

## 验收映射

| 标准 | 可复查生产路径与回归 |
| --- | --- |
| SC-001 | config.rs 严格解析；foundation 的 removed_resource_fields_are_rejected_with_the_config_path、readonly_check_all_modes_and_cross_directory；公开 CLI、零／超大／错误类型字段与冻结副本拒绝 |
| SC-002 | native_jobs.rs、workspace.rs、workflow.rs 及提交 argv；three_stage_execution 完整链、编号粗细网格／超导链的脚本递归断言，support/direct_submit.rs 检查实际执行脚本，包括双方法 gap 子任务 |
| SC-003 | historical_time_and_storage_limits_do_not_deny_registration_or_accounting 覆盖旧账本；numbered_submit_and_execution_ignore_exhausted_time_and_storage_limits 覆盖现版账本和公开 -s、执行、归集与结果账本摘要 |
| SC-004 | b06_unknown_scheduler_preserves_active_allocation；attached_allocations_still_enforce_existing_concurrency_limit；numbered_submission_freezes_config_and_blocks_duplicate_queued_job；numbered_deleted_stage_never_bypasses_queued_or_unknown_submission；失败恢复及历史保护由完整 EPW 回归核验 |
| SC-005 | 默认隔离回归使用工作区私有 sbatch／sacct／scontrol 和科学程序替身；不启用真实条件测试，不向真实 Slurm 提交 |

## 交付边界

源码与本地 Issue／Spec／Overview／使用说明随本次任务提交。两个忽略的可维护示例源配置也删除了旧字段，不强制纳入 Git。历史冻结配置、交付配置、账本原文、已提交脚本与科学数据不批量改写。未更新默认安装、未推送、未操作线上 Gitee；并发 0059 及 AGENTS.md 等无关更改不纳入本次提交。

## 未启用的 11 项条件测试

以下为既有条件用例，不计入通过数；本次不执行真实计算，也不恢复 LDA 验收要求。测试属性里的历史预算措辞不构成当前有效时间／存储额度。

| 文件与用例 | 保持忽略的原因 |
| --- | --- |
| `epwflow/src/delivery/decay/tests.rs`：`real_bound_native_decay_files_render_without_scientific_calls` | 需要显式指定真实粗网格回执及全新派生输出目录；只读科学数据 |
| `epwflow/src/interpolation/electronic_sampling.rs`：`real_workspace_wdata_readonly` | 显式提供既有 MgB2 配置，只读检查 wdata，不运行科学程序 |
| `epwflow/tests/electronic_0055_real.rs`：`real_electronic_dual_windows_and_pdos` | 需要指定真实 PBE 工作区、新目录及 Slurm 分配；只执行必要 projwfc 后处理 |
| `epwflow/tests/native_interpolation.rs`：`actual_lda_coarse_restart_and_wannier` | 依赖本机忽略的真实 LDA 粗网格，只读核验须显式指定 |
| `epwflow/tests/qe_data.rs`：`actual_lda_native_layout` | 需要 EPWFLOW_REAL_QE_ROOT 指向已有 LDA 证据，显式执行只读核验 |
| `epwflow/tests/qe_stage_plots_real.rs`：`existing_real_qe_data_plot_without_scientific_execution` | 需要显式指定已有真实工作区和新的图像输出目录 |
| `epwflow/tests/real_fine.rs`：`actual_lda_fine_matrices` | 真实 40³/20³ EPC；须显式配置、预留原共享预算并在 Slurm 中执行 |
| `epwflow/tests/real_fine.rs`：`actual_lda_fine_readback` | 只读重验已经终止的真实 EPC；不启动科学程序 |
| `epwflow/tests/real_tc.rs`：`actual_lda_native_tc` | 真实两模式 Tc；须冻结来源、预留原共享预算并在 Slurm 中显式执行 |
| `epwflow/tests/real_tc.rs`：`actual_lda_native_nonlinear_diagnostic` | 单次固定非线性诊断；须用户确认、历史次数与原共享预算 |
| `epwflow/tests/three_stage_execution.rs`：`numbered_real_legacy_pw_is_read_only` | 需显式设置本机 MgB2 配置；只读验证，不提交或执行科学程序 |

## 最终完整验证

在基线加 0058 专属补丁的独立副本中执行：

```bash
CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4 bash scripts/build-native.sh test -p epwflow --no-fail-fast --locked
```

命令退出 0，单次完整执行 280 项通过、0 失败、11 项条件忽略；其中 three_stage_execution 为 80 项通过、1 项条件忽略。没有把首轮中断或第二轮并发重构的编译失败拼接计入最终通过数。最后的编号 QE／Wannier／粗细网格／超导及恢复链通过，并完成新脚本／提交参数无时限断言；超旧额度的公开提交、执行和归集回归通过。SC-001～SC-005 均满足本项定义的隔离验收。

源码和测试 SHA-256 在独立验证期间保持不变，暂存内容按相同字节核对；所有任务文档相对文件链接通过。新增行空白检查通过，逐字原文的 8 行 Slurm 行尾空白有意保留。最终仅提交 0058 清单，保留共享工作树 0059 和 AGENTS.md 的未提交变化。

## 2026-11-17：彻底移除预算账本

本次采用用户新要求，撤销原先保留账本的设计。源码删除预算模块、配置、命令、全局状态目录、预留／绑定／结算与两作业限制；提交、执行、状态、恢复和交付不再访问历史账本。保留工作流锁、严格回执、作业身份与科学结果核验。

### 验证来源与范围

从本任务专属暂存内容导出独立源码副本，以独立构建缓存运行完整 EPW release 回归：**395 项通过、0 项失败、14 项条件测试忽略**。忽略项需要真实科学数据或显式环境，不计为通过，本次未启用。源码不混入既有 qf-plot／qf-io 及其他需求的未提交修改。测试过程中不重新构建正在执行的同一套程序。

```bash
cargo --config "/home/mayuan/code/dftflow/.cargo/linux-x86_64.toml" test --release --locked -p epwflow -j 6
cargo --config "/home/mayuan/code/dftflow/.cargo/linux-x86_64.toml" build --release --locked -p epwflow --bins -j 6 --message-format=json-render-diagnostics
```

两条命令均在独立源码目录执行，使用本机现有编译环境与独立 `CARGO_TARGET_DIR`。完整日志及安装核对保存在忽略目录 `docs/local/0058-budget-removal/`；不增加公开编译版本、旧命令备份或第二套用户入口。

### 需求与证据映射

| 需求／成功标准 | 实际验证 |
| --- | --- |
| FR-007，SC-008 | `foundation` 验证 `[budget]` 报配置路径与未知字段；三个公开程序帮助及参数解析均拒绝 `--budget`、`--ledger`、`--label`、`--token`、`--job` |
| FR-008，SC-006～007 | `obsolete_budget_files_and_locks_do_not_affect_submission_or_completion` 使用损坏账本、损坏注册表、两笔旧活动记录及占用中的旧账本锁，提交和实际 shell 执行替身均成功，旧文件逐字不变 |
| FR-008，SC-007 | `three_independent_workspaces_submit_with_one_shared_state_directory` 在同一全局状态目录下分别提交三个工作区，没有两个作业上限，不创建新账本 |
| FR-009，SC-006 | 原生／生成脚本执行、完整阶段、状态、迁移和结果交付回归均通过；新运行绑定和任务记录不生成预算关联字段，生产源码无预算功能引用 |
| FR-010，SC-007～009 | 严格回执、未知提交、活动作业防重复、失败恢复、原生错误和科学产物检查全部通过；12 项日志及恢复定向回归通过 |
| SC-009 | 四个固定命令已覆盖；安装文件与本次受测源码的构建产物逐字节一致，帮助启动成功；三个安装入口使用本任务的无预算 MgB₂ 示例副本完成 `--check --json` |

首次回归发现旧用例仍强制预算登记，按新契约调整。对 `-r` 保留控制器运行分配检查后，三项恢复测试暴露调度替身漏传作业号；仅修正测试控制器向私有 sacct 传入 `-j JOB`，随后定向及完整独立回归均通过，未放宽生产检查。

### 部署、文档和历史边界

已用同一份验证源码构建，并通过仓库发布器覆盖 `bin/epwflow/epw_phonon`、`epw_wannier`、`epw_superconductivity`、`epw_clear`。未启动真实 QE／EPW 或提交科学作业，没有修改 Slurm 设置、历史账本、机器注册文件、冻结输入或科学数据。

0007／0008、F2～F5、相关提交和配置 Issue／Spec 已注明旧预算条款撤销；0058 记录用户完整新要求、设计、任务、接口与验收。当前 Overview、手册和活动示例同步。新增相对文档链接全部核验可达；旧历史绝对链接不冒充新验证通过。线上 Gitee 仍由用户管理，未操作或宣称关闭线上 Issue，也不自动推送。

现场 MgB₂ 输入同时存在其他任务的 `[baseparams]` 和格式改动。本次只提交删除预算区段的差量，其他改动原样保留。当前解析器拒绝该额外区段；安装验证使用不含其他改动的任务专属示例副本，并将计算工作目录指向独立临时目录，不改写现场运行状态。
