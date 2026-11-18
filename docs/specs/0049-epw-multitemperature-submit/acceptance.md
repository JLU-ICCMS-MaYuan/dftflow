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


## 2026-10-22 联合真实验收补充

用户要求重新编译并在指定 test_auto／local_run 完成 0046～0050 全链，原文与授权边界见 [0050 补充需求](../0050-epw-stage-results/spec.md#真实全链验证补充授权)。本次是既有 Feature 的修复与验收，不另建同范围 Issue；线上 Gitee 仍由用户管理。

一次公开提交得到 1334469／1334470 两个独立作业；各执行一次 EPW 的 20／30／40 K 完整列表，六点均执行完成、收敛且数据有效。显式 μ*=0.05 保留，实际目录为 iso／aniso-mu0.05-20-40；较新的 0050 规定作业内收尾，无需再次公开命令归集。

实际作业、制品、失败修复、完整回归、安装及示例清理状态统一见 [联合验收记录](../0050-epw-stage-results/real-chain-validation.md)。当前真实链已完成；部署及文档收尾以该记录的最终状态为准，不把旧替身证据改写成科学验证。


## 2026-11-18：新版 #49 独立展宽与参数目录落地

### 本轮范围与事实

用户要求“现在你来落地实现 #49”，随后明确“按照新的 #49 重新执行！”。开始时工作区已存在本功能 Spec 更新及部分命名代码；实施过程中重新读取了更新后的规则，采用第 8 步显式 degaussq 优先、第 7 步 spectrum_index 回退。没有采用更新前仅从第 7 步命名的方案。

- FR-003／004、SC-002：目录使用 mode、μ*、完整列表首末温度、有效展宽；2.0 规范为 2，示例为 aniso-mu0.1-Tc20-40-degaussq2meV。恢复剩余子批次仍使用完整原计划目录。目录与原生输入共用 smearing 解析，非法显式值不能静默回退。
- 独立展宽：将有效值纳入已有 NativeInputs 的配置视图，显式值默认选择细矩阵路线，不继续读取旧 α²F；显式非空 fila2f 与独立重算冲突时报告具体原因，不伪称新展宽已用于旧谱。未填写时保留谱选择机制并回填所选值。
- 旧结果保护：第 8 步参数更新仅重置当前状态，保留 iso-mu／aniso-mu 方法目录；不同展宽结果并存，不把旧成功点登记为新参数成功。不实现本轮范围外的细网格分层目录，不自动迁移旧格式路径。
- FR-006～008：双方法独立提交、未知回执防重提、异常温度／数据、早停恢复及已有单点证据继续核验。扩展回归发现旧单点 aniso 未交接 selecq.fmt，补齐相同文件交接后通过原用例。

### 验证

证据目录为 docs/local/0049-directory-parameters-20261118/，受控进程的临时工作区为 /var/tmp/dftflow-0049-tests/；均不纳入 Git。不修改用户 TMPDIR 或系统挂载配置，仅为本次命令指定测试临时目录。

| 验证 | 结果 | 证据 |
| --- | --- | --- |
| 最终 release 库单元 | 90 通过，4 项既有外部来源测试忽略 | final-regression-v2.log |
| 超导／旧单点／原生交付回归 | 10 通过、0 失败 | final-regression-v2.log |
| 多温度公开进程 | 7 通过、0 失败 | multitemperature-v3.log |
| 固定入口构建与发布 | 4 个产品与构建缓存制品哈希一致；4 个 --help 成功 | build.log、verification.json |

合计 107 项通过、4 项忽略。忽略不计通过；本轮没有提交真实 QE／EPW／Slurm 科学作业，不宣称新展宽的真实 Tc 或能隙通过科学验收。

公开用例实际执行受控原生脚本，验证新目录、0.5→2 meV 的输入值、两方法独立回执及早停恢复。参数切换前后旧方法 epw.out 哈希保持，fine 原生日志哈希保持，QE 与 fine 调用计数不增加。夹具先显式建立 1～7 步来源，补全矩阵路线的 DOS 交付并隔离旧模拟作业号；未恢复生产隐式补上游，也未放宽科学校验。

中间失败如实保留：initial-contract.log 的断言仍使用旧目录；multitemperature.log 的新增测试类型推断失败；multitemperature-final.log 的派生／显式 fila2f 冲突及旧夹具缺 DOS；final-regression.log 的旧单点 aniso 缺 selecq 交接。最终代码和夹具分别修复后重跑上述最终集合，无失败被改成 ignored。

### 构建与协作边界

按 scripts/build-native.sh build --release -j 2 -p epwflow --bins --locked 成功构建，直接覆盖 bin/epwflow 的四个固定产品，不建立公开版本目录或旧命令备份。当前共享工作树还有并行 #61 等未提交改动，构建使用当前工作树，不能把制品表述为仅本次 Git 提交；源码边界与产品哈希见 verification.json。本次提交只覆盖 #49 的实现、测试、设计和 native-superconductivity 总览，不纳入 #61、线宽、算例配置或 PDF 删除。

已使用 big-project-overview-maintainer 回写当前行为，并更新本 Feature 快速开始。模块手册和根 README 的既有导航仍能到达相应说明，本轮未覆盖它们正在进行的其他功能编辑。GitHub #49 只读核对为历史 CLOSED、唯一 type:feature；本轮不修改线上状态、不推送，新版 Spec 的远程发布仍待后续授权，不能把本地交付当成线上新一轮关闭验收。
