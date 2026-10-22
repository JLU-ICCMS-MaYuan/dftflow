# 验收记录

日期：2026-10-22。当前：本地实现及验证完成。

## 交付行为与依据

- gap 的 both 一次提交两个独立方法作业，每个脚本直接调用一次 EPW，输入以 nstemp／temps(i) 包含该方法全部温度；单方法只提交一个。
- 各方法分别共用 iso-mu<实际值>-<首温度>-<末温度>／aniso-mu<实际值>-<首温度>-<末温度>，省略 mu_star 使用 0.13，显式其他值保留；不新增温度子目录。
- 逐点核验原生日志、收敛及完整 Δ／Z；提前停止后保留已执行点与不可变快照，显式恢复只补未执行完成点。活动与未知回执不重提，两方法独立保存状态及共享预算预留。
- 点清单版本 4 读取版本 3，批次点证据版本 2 兼容旧单点版本 1；旧 Tc 搜索及已登记分配的执行路径保留。

实现复用原脚本／预算／调度接口、单点解析和结果交付；批次模块只负责分组及多温度证据，不新增公共程序、依赖或第二套结果包。

## 验证

完整回归使用以下命令，结果为 232 项通过、0 失败、8 项忽略（含单元测试与公开进程测试）：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow
```

此后仅补充提交前的脚本／输入身份复核与相应负向用例，最终源码的批次专项 6 项全部通过：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution batch_
```

共享提交入口的最终兼容性复测 16 项通过、0 失败，覆盖旧 Tc 双方法完整公开推进：

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash "scripts/build-native.sh" test -p epwflow --test three_stage_execution direct_submit::
```

最后源码的 6 项批次与 16 项共享提交专项均通过。以上专项与完整回归存在重复，不累加宣称为独立测试数。完整日志位于本机 /tmp/dftflow-0049-full-tests.log、/tmp/dftflow-0049-final-batch-retry.log 及 /tmp/dftflow-0049-final-submit-tests.log，/tmp 记录不作为长期科学证据。

公开进程用受控 Slurm／原生 IO 替身实际执行所生成脚本，不将替身数值作为科学结果。批次检查覆盖两个作业各一次 EPW 调用及完整温度集合、逆序结束、重复命令不重提、非等距／小数／单点与显式 μ、第二提交未知、未收敛提前停止、非零退出后保留前缀、NaN／缺 Δ/Z、缺正常结束、错温度、来源变更拒绝、结果归集与移动重绘。

测试调度中曾因并行构建替换运行程序导致路径带 (deleted) 而失败，后改为串行；最后专项首次运行另有同类程序路径失败，发生在上游准备且未进入批次，使用当前构建重新执行整组后 6 项通过。失败运行不计通过，也不据此放宽程序身份核验。

文档原话、9 条 FR／5 条 SC 映射、14 份 Markdown 的本地链接及新模块格式核验已通过。算例 README 仅追加本任务说明并隔离暂存，不纳入用户既有段落修改；AGENTS.md 既有变更及并行任务 0050 均不属于本次提交。

## 保留的真实验收入口

以下八项既有忽略测试均未执行，不计通过：

| 测试 | 未执行原因 |
| --- | --- |
| actual_lda_coarse_restart_and_wannier | 依赖本机忽略的真实 LDA 粗网格，须显式指定只读核验 |
| actual_lda_native_layout | 需要 EPWFLOW_REAL_QE_ROOT 指向既有 LDA 证据 |
| existing_real_qe_data_plot_without_scientific_execution | 需要显式指定真实工作区和新的图像目录 |
| actual_lda_fine_matrices | 真实 40³／20³ EPC，须配置、预留共享预算并在 Slurm 执行 |
| actual_lda_fine_readback | 只读重验既有真实 EPC，需要相应本机证据 |
| actual_lda_native_nonlinear_diagnostic | 固定非线性诊断，须核对历史次数、用户授权和原共享预算 |
| numbered_real_legacy_pw_is_read_only | 需要显式设置本机 MgB₂ 配置，仅只读核验既有 PW |
| actual_lda_native_tc | 真实两模式 Tc，须冻结来源、预留共享预算并显式执行 |

## 边界与状态

本轮不启动真实科学任务、不修改历史科学来源／显式 μ、不更新仓库或默认安装的交付程序、不操作线上 Issue 或推送。原生多温度科学收敛尚无本轮实测；旧活动单点现场不宣称已经自动迁移，完整旧点清单和原执行接口按回归边界兼容。

本地实现按用户“认可这个计划。开始执行”授权完成。线上 Gitee 由用户管理；远程 Issue／Spec 双向链接和单独完整 Spec 审查检查项仍开放，不冒充线上关闭。
