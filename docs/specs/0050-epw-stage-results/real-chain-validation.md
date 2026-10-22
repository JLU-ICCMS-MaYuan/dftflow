# 0046～0051 真实 MgB₂ 全链验收

## 目的与授权

用户要求重新编译当前合并实现、编写真实测试脚本并完成全链，之后再清理 examples 现场用于手动测试。完整原文见 [Spec 补充授权](spec.md#真实全链验证补充授权)。沿用线上 Gitee 用户自管约定，本记录关联既有需求，不新增同范围 Feature，不冒充线上关闭。

## 固定输入与执行范围

测试脚本为 `test_auto/MgB2/epwflow/real_chain.py`；诊断现场为 `test_auto/MgB2/local_run/011-stage-results/`，最终科学现场为 `test_auto/MgB2/local_run/012-stage-results/`。从 examples 当前 TOML 复制科学配置，仅隔离 work_dir 和结构路径。PBE、SCF／NSCF 4×4×2、PH 2×2×1、细 k／q 8×8×4／4×4×2、5 个 Wannier 函数、显式 μ*=0.05、iso／aniso 20／30／40 K 均保持。此为小网格功能验收，不作收敛、插值质量评级或 Tc 搜索。

顺序为 SCF → NSCF → QE 电子 → Wannier 电子对比 → QE 声子 → coarse 声子对比 → fine → 双方法 gap。第 5 步特意先于第 4 步执行，检验无 PH 的电子交付。科学程序只在登记的 Slurm 分配中执行，不在登录节点调用 MPI 科学程序。

共享账本仍为 `docs/local/epwflow-campaign.json`，上限 28800 节点秒／100 GiB，同时最多两个分配。开始前账本累计 4405 节点秒，尚登记 1331914；Slurm 实查其已完成、0:0、1 节点×5 秒，通过既有预算入口结算，不手改账本。运行前冻结受测程序及全部配置，保留失败记录；恢复前先核对身份，不能覆盖运行中的程序。

## 需求、实现与证据对应

| 范围 | 已确认设计与组件 | 本轮验收 |
|---|---|---|
| 0046 FR-001～008、SC-001～004 | native_jobs 的固定脚本、合并 Slurm 日志、真实 out、移除工作区控制锁 | 完整回归覆盖中断／旧锁／特殊路径；真实生成脚本、实际 WorkDir 和科学日志另存 |
| 0046 FR-009～010 | 原 budget 分配记账及来源保护 | 运行前后账本和原生任务记录；不重置消费 |
| 0047 SC-001～006 | workspace_qe_plot、qf-phonon 真实逐原子投影 | 第 3／4 步即时图、Mg／B 表及图片实看；完整回归覆盖缺图恢复 |
| 0048 SC-001、004～005 | 父分配内 PH、save、q2r、两次 matdyn 与发布 | 一次公开提交即有完整产物；没有下一阶段产物；完成重复命令无新任务 |
| 0048 SC-002～003、006 | 子任务检查点及失败保护 | 完整回归故障注入，真实主路径不人为破坏 PH 数据 |
| 0049 SC-001～003 | batch 的原生多温度及内部收尾 | 两独立作业、各一次 EPW，三温度及逐点 Δ／Z／收敛与最终发布 |
| 0049 SC-004～005 | batch 显式恢复及历史兼容 | 完整回归覆盖中断／未知作业／旧 Tc；真实成功链检验幂等 |
| 0050 SC-001～003、006 | wdata 内嵌采样、postw90 DOS、stage_results | 无 PH 时两电子图、真实坐标、四窗口和调用记录；作业内发布 |
| 0050 SC-004～005、007 | 第 5／6 步职责、复用决策与派生检查点 | 相容结果不增加科学任务；必要 q2r／matdyn 的事前技术原因；回归覆盖损坏拒绝 |
| 0050 SC-008 | 自包含结果与独立重绘 | 复制根 results，在 PATH 不含科学程序的环境重绘、表哈希不变 |

## 当前进度

0046～0050 的独立完整回归为 277 项通过、0 失败、9 项默认暂挂；真实八阶段完成，失败现场及修复证据保留。用户随后明确将 0051 纳入联合交付，合并回归及新增衰减发布验收继续单独记录。部署通过之前不清理 examples，不提前宣称手动测试已就绪。

## 实测发现与修复

首轮真实 SCF（1334098，21 节点秒）、NSCF（1334110，5 节点秒）均完成且复用未重算。第 3 步 1334118 在一个分配内自动完成并发布电子单侧结果（155 节点秒）。发现阶段初始化归档目录时只恢复 phonon.sh，其他目标的当前父脚本及固定名 slurm-<功能>.log 被移入历史目录。科学结果不受影响，但用户依据提交输出找不到当前日志。修复推广既有父脚本恢复至当前阶段，固定名日志通过硬链接保留 Slurm 持有的同一 inode；不改变科学任务或状态所有权。回归增加实际打开句柄、归档后继续写入和当前脚本字节核验。待复测通过后记录最终制品，不将首轮旧制品当作修复后全链证据。


第 5 步首轮 1334183 在 EPW 内嵌 Wannier90 初始化失败。输入有完整路径坐标，但实际生成的 `.win` 每行只有 80 字节（含首空格），原 15 位小数行被截断。部署程序为 EPW 5.9；本地源码声明的容量与实际制品行为不同。改用去除无意义末尾零的十位小数，保证 wdata 路径行不超过 79 字节；不改路径点、物理参数、拟合窗口或外部科学程序。真实 183 点路径加 32 点均匀网格对应的最长配置行为 72 字节。输出继续按真实坐标提取，不以横轴缩放代替点位核验。

优化制品的独立现场 `012-stage-results` 重新从 SCF 开始。1334318（4 秒）、1334322（5 秒）、1334327（37 秒）、1334339（32 秒）、1334348（110 秒）均为 COMPLETED、0:0。电子图在尚无 PH／coarse 时已发布；声子作业一次完成全部后处理和 Mg／B 投影。以上结果均先核验作业内完成及根 results，再调用公开入口验证幂等；不是再次调用公开入口才完成出图。

1334375 的粗网格原生 EPW 成功（Slurm step 为 0:0），父作业 40 秒后因后处理误报第 5／6 步坐标不同而失败。SCF／NSCF XML 倒格矢末位相差约 1e-15；原代码用 JSON 完全相等比较包含累积路径距离的结构，误拒绝同一条路径。修复核验点数、顺序、分段、晶体坐标、倒格矢及距离，并允许 1e-10 的数值舍入；均匀网格沿用既有 1e-7 坐标容差并补足长度检查。新增单测确认真实坐标变化、分段变化、点序与数量变化仍拒绝。成功粗网格检查点保留，后续用新制品显式恢复，不重算成功科学任务。

首轮 debug 全量运行有三项进程契约失败，原始日志 `/tmp/dftflow-0046-0050-full-tests.log` 保留；其中一项是正在运行的程序路径变为 `(deleted)`，另外两项在初始配置路径解析时报告文件不存在。随后确认工作区另有 0051 正在编辑和编译，不能把该轮混合状态计为通过。再次在主工作区启动 release 回归时，因未完成的 `delivery/decay` 测试模块缺失而编译失败。为隔离并发影响，从已提交 0050 基线 `5f07b7e` 导出源码，仅叠加本轮修复，在独立 Cargo target 中进行完整串行回归；不删除或提交 0051 的修改。真实运行始终调用已冻结副本，不受主工作区二进制替换影响。这份快照作为首批 0046～0050 验证依据保留；用户随后要求纳入 0051，最终交付以追加的合并制品为准。


## 真实全链结果

实际科学现场为 `test_auto/MgB2/local_run/012-stage-results/`，第 6 步修复后使用独立源码快照的优化制品继续公开 `--resume -s`。所有旧制品、失败作业和检查点保留；没有手改科学输入或完成回执。

| 阶段 | Slurm 作业 | 结果与节点秒 |
|---|---|---|
| 1 SCF | 1334318 | COMPLETED，0:0，4 秒 |
| 2 NSCF | 1334322 | COMPLETED，0:0，5 秒 |
| 3 QE 电子 | 1334327 | COMPLETED，0:0，37 秒；两张单独图即时发布 |
| 5 Wannier／电子比较 | 1334339 | COMPLETED，0:0，32 秒；此前不存在声子或 coarse 完成记录 |
| 4 QE 声子 | 1334348 | COMPLETED，0:0，110 秒；PH、save、后处理、元素 DOS 作业内完成 |
| 6 原生 coarse | 1334375 | 原生子任务成功；父作业因舍入检查误判失败，40 秒，成功科学检查点保留 |
| 6 恢复比较／发布 | 1334435 | COMPLETED，0:0，59 秒；没有重复粗网格计算 |
| 7 fine | 1334453 | COMPLETED，0:0，43 秒 |
| 8 iso 三温度 | 1334469 | COMPLETED，0:0，5 秒 |
| 8 aniso 三温度 | 1334470 | COMPLETED，0:0，22 秒 |

外部程序由实际日志确认：PW／PH 为 QE 7.4.1，EPW 为 5.9，内嵌 Wannier90 与 postw90 均显示 3.1.0。初始程序、输入和结构哈希见现场 `baseline.json`；修复制品见 `active-programs.json` 与 `evidence/bin-revision-*.json`。最终编排器的源码／打包 SHA-256 记录在 `docs/local/0050-stage-results-real/package-final/build-identity.json`。

`audit.json` 核对到原生任务：PW 3 次（SCF、NSCF、路径 bands）、PH 1 次、bands 1 次、dos 1 次、q2r 2 次、matdyn 3 次、EPW 6 次（拟合、coarse、声子比较求值、fine、iso、aniso）、postw90 1 次。全部任务 validated；没有独立 wannier90.x 绘图调用。每阶段完成后重复公开命令均无新科学任务或提交。

第 6 步新增 q2r／matdyn 的事前必要原因在 `comparison-native/phonon/{force-constants,frequencies}/reuse-decision.json`：第 4 步采用 simple ASR，比较采用同源 Γ onsite 加 Hermitian 修正且不再次施加 ASR；所需频率还包含完整路径和 DOS 网格。仅转换已有动力学矩阵及力常数求值，不重做 PW／PH。首次相容插值数据不存在时，EPW 读取既有 coarse 结果求值；后续完整重执行不增加调用。

根 results 的第 3／4／5／6／8 步包均在作业结束后、再次调用阶段公开命令之前存在且核验通过。第 7 步没有新增独立绘图任务；其 α²F／λ 在第 8 步已有图中交付。实际查看第 5 步能带及 DOS、第 4 步 Mg／B DOS、第 6 步四面板总览和第 8 步能隙温度图。Mg+B 与 TDOS 的 CSV 最大残差约 1e-6，符合六位小数输出精度。

第 5 步核验 183 路径点、32 均匀网格点；wdata 含 137 条路径行，最长 72 字节。实际相对 Fermi 外窗约 [-12.574,17.256] eV、冻结窗 [-12.574,2.000] eV；电子路径最大能量配对差 2.000654 eV、网格 0.411649 eV。声子路径最大配对差 251.9238 cm⁻¹、网格 232.7245 cm⁻¹。这些是人读诊断，保留全部未配对态，不自动评级、调窗或判网格收敛。细网格原生日志 λ=1.0810509。iso／aniso 的 20、30、40 K 六点分别 execution_complete、converged、data_status=valid；运行清单 execution_complete 与 data_complete 均 true。当前 task=gap，不作 Tc 结论。

完整根 results 已复制到 `portable-results/`，在 PATH=/nonexistent 环境执行公开 `--plot` 和 `--results-check`，均通过；CSV／DAT／EIG 表哈希不变，详见 `portable-verification.json`。没有调用科学程序或提交 Slurm。

共享账本最终累计 5070 节点秒，无 allocations／reservations；本轮相对结算后的起点 4410 增加 660 秒，含首轮失败与修复作业。最终 local_run 实占约 11 GiB，未重置历史消费。账本快照和结算输出保存在现场。


## 合并 0051 的验收范围

用户明确要求“把 0051 也纳入联合验收后再交付”。从首批独立快照叠加已提交的 0051（基线 `4e9eae0`）及本轮三项缺陷修复，逐文件记录 SHA-256 到 `docs/local/0050-stage-results-real/source-freeze-merged.json`。使用相同独占 Cargo target 增量编译；先等首批全部测试退出，再开始编译，避免替换运行中的受测程序。

首批完整 EPW／QE 回归 277 通过、0 失败、9 默认暂挂，日志归档为 `docs/local/0050-stage-results-real/full-regression-0046-0050.log`。已显式执行 `real_workspace_wdata_readonly` 通过；另外八项历史 LDA／指定来源测试没有作为本次 PBE 小网格、gap 验收条件。0051 新增只读真实衰减测试也需要显式来源，不把默认暂挂计作通过。

0051 只消费第 6 步原生文件，不改变科学参数。联合真实验收复用 `012-stage-results` 的成功 coarse，先冻结科学完成回执与四份 decay 数据，再用合并制品执行公开 coarse 目标补派生发布；前后核对全部原生调用记录和源文件哈希，避免因新增图片重跑成功计算。15 图、逐行重复距离数据及包含衰减图的完整结果移动重绘另外核验。


### 0051 合并制品的真实发布

合并制品已冻结到 `package-merged-0051/build-identity.json`，真实现场后续入口升级为 `bin-revision-1792681853986615774`；保留前两版制品，不覆盖既有脚本绑定。公开 coarse 提交作业 **1334753**，COMPLETED、0:0、42 节点秒。作业结束后、再次执行公开 coarse 之前，`results/6.epw-coarse/decay/` 已有全部 15 图；随后重复命令无新提交。

冻结的 18 份科学完成记录及原始衰减文件哈希全部不变。四份绘图副本与原始前两列逐行一致，保留重复距离；原生任务计数由脚本再次核对，不允许增加 EPW／PW／PH 等调用。已实际查看总览 PNG、SVG 栅格化结果及 PDF 栅格化结果，面板顺序、紫色散点、Å／Ry 单位与对数纵轴一致。

合并版本另外显式执行真实路径输入只读测试及真实回执绑定的衰减只读测试，各 1 通过；不调用科学程序。原生版本保持 QE 7.4.1／EPW 5.9／W90 3.1.0；不升级外部科学程序。


### 合并回归的进程中断与补验方式

合并 `cargo test` 已完成前序各套件，在 `three_stage_execution` 运行期间，整个会话返回 143（SIGTERM）；日志没有失败断言，未确认外部终止来源，不能把该次全量命令写成通过。保留 `/tmp/dftflow-0046-0051-merged-tests.log`，按测试名核对已完成部分；剩余 57 项（含一项指定旧来源的默认暂挂）分为编号阶段、声子链、绘图发布和多温度／长链四组，直接执行同次编译生成的测试二进制，不再次编译或替换制品。各组日志为 `/tmp/dftflow-merged-{numbered,phonon,plots,remaining}.log`，最终按测试名去重核对全部覆盖，逐组检查退出码。

已完成的真实联合 `audit` 和 `portable --portable-name portable-results-0051` 均退出 0：57／7／57／7 个原始点逐行保留，15 图完整，科学调用计数保持不变；空 PATH 下完整结果包重绘及校验成功，原生表与衰减 JSON／副本哈希不变。旧移动副本原样保留。


### 最终回归结论

合并 EPW 全部套件按上述方式核验，**248 项通过、0 失败、10 项默认暂挂**；QE 源码及共享依赖没有新增变化，首批完整 QE 回归 **34 项通过**。合计 **282 项通过**。主进程套件 75 个测试名逐项核对：74 通过、1 默认暂挂，没有遗漏；四个补验组分别 26／10／11／9 项通过，退出码均为 0，窗口套件另外 10 项通过。完整合并 Cargo 会话仍如实记录退出 143，不冒充一次全量命令返回零。

10 个默认暂挂中，两项真实只读测试已显式通过，另外八项指定历史 LDA／旧来源／独立 Tc 的测试没有执行：`actual_lda_coarse_restart_and_wannier`、`actual_lda_native_layout`、`existing_real_qe_data_plot_without_scientific_execution`、`actual_lda_fine_matrices`、`actual_lda_fine_readback`、`actual_lda_native_nonlinear_diagnostic`、`actual_lda_native_tc`、`numbered_real_legacy_pw_is_read_only`。这些依赖专门历史来源或另一科学任务，不以本轮 PBE gap 数据伪造，也不把暂挂计作通过；逐项原因沿用 [初次验收表](acceptance.md)。

最终汇总保存在 `docs/local/0050-stage-results-real/merged-regression-summary.json`，相关原始日志已复制进同一证据目录。13 份合并源／测试文件与主工作树逐字节一致；待安装三个制品与真实现场当前冻结副本 SHA-256 完全相同。真实科学链、替身故障回归、安装与科学收敛仍分别记账。


## 默认安装、手动现场和文档交付

已将 `package-merged-0051` 的三个优化兼容程序安装到仓库 `bin/epwflow/` 与用户 `~/.local/share/dftflow/current/bin/`，并核对 `~/.local/bin/` 三个入口的真实目标、SHA-256 与 `--help`。九个路径读到的程序分别与本次真实受测副本完全一致；只更新 EPW 三程序，未替换其他模块或外部科学库。旧程序备份与完整身份核验见现场 `installation-before-0051/installation-verified.json`。

全部作业已终态且无活动预留后，将 examples 的八阶段目录、`epw-state/`、`native-plan.json` 移到 `test_auto/MgB2/local_run/012-stage-results/example-before-manual/`。归档前后 **1394 项**文件／目录／链接校验一致；配置、KPATH、上级 POSCAR 与两份赝势哈希不变，用户原 README 另存 `.before`。原计算目录只保留 `README.md`、`RESULTS.md`、`epwinput.toml`、`KPATH.in`，没有残留完成记录或运行文件。原始科学证据保留在归档，历史回执内绝对路径不冒充当前可续算现场。

[手动 README](../../../examples/MgB2/sugon-amd/run/epwflow/README.md)已按真实当前输入重写：1／2 作业结束再公开核验，3～8 正常作业内完成后处理与发布，第 8 步一次提交两个方法批次、μ*=0.05、20／30／40 K，明确 task=gap。图表位置与 0051 衰减说明在同目录 RESULTS.md；[测试说明](../../../test_auto/MgB2/epwflow/README.md)保留完整复现命令与旧现场边界。0046～0051 的本地 Issue／Spec／验收按实际证据关联，原始 text 围栏核验保持不变；线上 Gitee 仍用户自管，未关闭或推送。

在清空后的 examples 目录 source 仓库 env，再执行默认公开 `epw_phonon -c epwinput.toml --check --json`，退出 0：operation_status=ok、workflow_status=configured，configuration／input_files／program_paths／mpi_pool_image 全部通过；workflow_complete=false 正是全新现场。没有提交 examples 新作业，用户可从第 1 步开始。命令及入口记录见现场 `fresh-example-check.json`。

共享账本最终 **5112 节点秒**、allocations／reservations 均为空；较结算后起点 4410 新增 **702 秒**，含失败诊断、恢复及最后 42 秒派生发布。最终账本见 `ledger-final-0051.json`。真实科学输入和粗网格诊断差异保留，不将流程验收等同于选窗、网格收敛或 Tc 科学结论。


文档收尾核验：0046～0051 及受影响手册共 66 份 Markdown、404 条本地链接均有效，原始用户 text 围栏保留；测试脚本 Python 语法通过，Git whitespace 检查通过。最终 local_run 实占约 11 GiB，低于共享上限；未自动提交 examples 科学作业。代码修复仅扩展已有归档、紧凑路径输出与真实采样一致性核验，复用现有状态机和发布机制，不增加第二套执行链。
