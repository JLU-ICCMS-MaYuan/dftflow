# 验收记录

2026-10-26：严格版本 2 原生阶段配置已实现，回归和清理后八阶段重算通过；六个指定温度能隙点通过现有收敛与数据检查，Tc 和网格收敛不属于本次验收。

## 已有验证与待完成项

| 需求 | 实现与验证入口 | 当前证据 |
|---|---|---|
| FR-001～003、005～007、009 | `foundation/native_config.rs`、`tests/native_stage_config.rs` | 12 项配置契约通过；涵盖独立参数、原生数组/卡片、版本/类型/长度、双重定义及冲突路径 |
| FR-004、008 | 输入生成器、阶段指纹、电子叶任务与后处理回执 | 原生插值 13 项通过、1 项历史 LDA 检查未执行；公开 CLI 83 项经全量与定向复测去重通过，1 项旧现场只读检查未执行 |
| FR-010、SC-004 | 当前算例清理 | 已完成；134 项旧内容删除且无备份归档，保留八目录 inode、配置、README、KPATH、外部结构及账本哈希 |
| FR-011、SC-005 | 八阶段及两个方法各 20/30/40 K | 八阶段与六个温度点全部通过本轮新回执、收敛、数据及结果包检查 |
| SC-006 | Overview、模块与算例说明 | 协议、单位、参数来源、私有程序及本轮验收均已同步；按精确任务差异提交 |

中间回归发现并修正：旧测试使用科学默认值、原生数组 `temps` 索引、赝势卡片文件名、非编号 Wannier/粗网格合并路径、原生数组文本下标排序，以及旧配置导入的宽松假设。失败运行日志留在本地 `/tmp/dftflow-native-*.log`，不作为通过证据。

## 清理前只读调查

账本累计消费为 5482 节点秒，活动分配和预留均为空；0058 已取消时间/存储限制，本项不重置历史消费，也不恢复旧限制。当前队列首次核对为空。

旧活动指针对应作业 1370345、1370507、1370508 已完成。另两条旧 `submission_pending` 尝试通过相邻 `submission.json` 追溯到 1346626、1347062，`sacct` 均为 `FAILED`、退出 `0:53`。后续清理前必须重新检查队列、回执和账本，不能仅依赖本次较早的调查。

## 证据边界

配置/进程回归不替代原生科学验收。真实验收须绑定本轮构建、清理后新生成输入、原生日志和当前完成回执；不读取旧结果作为完成依据，不将能隙结果表述为 Tc。

## 并行构建干扰

第二次公开链冒烟已完成至细网格，并完成 aniso 搜索；iso 恢复期间另一个会话重建共享 `debug` 目录，进程记录报出当前可执行文件 `(deleted)`，本次不计通过。后续使用任务独立源码及构建目录验证，避免把共享制品替换误判为科学求解失败。

0060 正在并行实现后续 NSCF 简写网格与 FFT 接续校验，依赖本项原生协议。本项提交必须隔离其改动；已从 0060 的原话记录核实用户明确要求“开始执行落地”，真实重算采用该后续简写契约，不再要求重复授权；0059 独立提交仍保留其基线差异。

## 完整首轮矩阵与修正

release 首轮 EPW 287 项通过、5 项失败、11 项既有真实环境用例未执行；qeflow 34 项、qf-input 11 项通过。失败涉及旧错误字段断言、官方八位小数网格的舍入容差、非编号内部链的 F2 采样路径，以及旧公共采样用例。这些失败已在后续定向复测通过，首轮命令退出 101，不作为全量通过。

当前算例 TOML 属于仓库明确忽略的本机运行配置，已直接重写并预检；不强制纳入 Git。共享代码、原生配置夹具和说明按任务差异提交。

## 默认未执行的真实环境用例

下列用例不计通过；本轮 PBE 八阶段重算另列实际证据，不用历史 LDA 验证替代。

| 用例 | 原因 |
|---|---|
| `delivery::decay::tests::real_bound_native_decay_files_render_without_scientific_calls` | 需显式指定真实粗网格回执和新派生输出目录 |
| `interpolation::electronic_sampling::tests::real_workspace_wdata_readonly` | 需显式指定真实配置，只读核对 wdata |
| `real_electronic_dual_windows_and_pdos` | 需指定真实 PBE 工作区、新目录和 Slurm 分配 |
| `actual_lda_coarse_restart_and_wannier` | 本机 LDA 历史粗网格只读验证，当前范围为 PBE |
| `actual_lda_native_layout` | 需显式指定历史 LDA 来源，当前范围为 PBE |
| `existing_real_qe_data_plot_without_scientific_execution` | 需显式指定真实工作区和新图目录 |
| `actual_lda_fine_matrices` | 历史 LDA 40³/20³ 真实 EPC，不属于当前小网格 PBE 重算 |
| `actual_lda_fine_readback` | 需显式指定历史 LDA EPC，只读检查 |
| `actual_lda_native_nonlinear_diagnostic` | 历史 LDA 独立非线性诊断，不属于当前范围 |
| `actual_lda_native_tc` | 历史 LDA 两模式 Tc，不属于当前能隙任务 |
| `numbered_real_legacy_pw_is_read_only` | 需显式指定旧真实工作区，本轮从头计算 |
| `actual_sip2_fft_evidence_is_read_only_and_rejected`（0060） | 需显式指定 SiP₂ 历史 XML，默认套件只执行隔离 FFT 用例 |

## 最终回归覆盖与固定构建

独立源码和独立 release 构建目录为 `/tmp/dftflow-0059-optimized`，没有在公开 EPW 测试运行时重建其程序。

| 组件 | 去重后的通过覆盖 | 未执行 | 说明 |
|---|---:|---:|---|
| EPW（含获授权的 0060 网格和 FFT） | 303 | 12 | 全量 291 通过、12 失败；修正两份内嵌旧点表夹具后插值 8 项、超导 16 项通过，保留已绑定实际 NSCF 输入后导入用例通过 |
| qeflow | 34 | 0 | 本轮全量运行通过 |
| qf-input | 11 | 0 | 全量中 10 通过、1 失败；原有两个测试使用同 PID 临时目录并互删，整个组件串行复测 11 项通过 |

共 348 项通过覆盖；这是全量与有原因的定向复测去重结果，不宣称一次全量命令退出 0。首轮命令退出 101，最终定向命令均退出 0。原始日志保存在 `docs/local/0059-epw-native-stage-config/`，包括 `dftflow-0059-integrated-final.log`、`dftflow-0059-integrated-retest.log`、`dftflow-0059-integrated-import-retest.log` 和 `dftflow-0059-shared-input-retest.log`。

真实重算程序固定在 `docs/local/0059-native-bin/`，仅包含本轮三个计算 CLI。`SOURCE-SHA256SUMS`、`SHA256SUMS` 和 `build-source.json` 保存打包前后指纹及源码文件身份；构建基线 `fa5bf3f` 加本项及获授权的 0060 未提交覆盖。0060 依赖差异保留独立，不混入本项 Git 提交。默认安装不切换。固定程序对当前 TOML 的 `--check --json` 返回 `operation_status=ok`。

## 本轮从头计算与实际输入核对

清理前再次核对队列为空、五个旧指针均为明确终态、账本无活动分配或预留。删除八阶段内旧内容及旧状态、results、native-plan.json、RESULTS.md，共 134 个顶层删除项；未备份或归档，不跟随链接删除目录外文件。八阶段目录 inode 保留，新 TOML、README、KPATH、上级 POSCAR 和账本在清理前后哈希一致。记录为 `docs/local/0059-epw-native-stage-config/cleanup.jsonl`。

使用固定私有程序依次完成以下作业，无参数扫描、无失败重试，未使用旧科学结果作为完成依据：

| 阶段 | 作业 | 结果 | 节点秒 |
|---|---|---|---:|
| 1 SCF | 1372828 | 完成，5 次迭代收敛，普通 `-s` 归集 | 13 |
| 2 NSCF | 1372832 | 完成，普通 `-s` 归集，输入及输出 32 点通过核验 | 4 |
| 3 QE 电子 | 1372837 | 能带、DOS、元素 PDOS、图表与回执完成 | 47 |
| 4 PH／q2r／matdyn | 1372847 | 两个不可约 q 点与全部后处理、图表完成 | 114 |
| 5 Wannier | 1372859 | 解纠缠、局域化收敛，电子比较与 DOS 完成 | 25 |
| 6 coarse | 1372867 | 原生粗网格、比较、实空间衰减图完成 | 70 |
| 7 fine | 1372875 | 原生矩阵和 α²F／λ 完成，独立使用 20 K | 41 |
| 8 iso | 1372882 | 20／30／40 K 全部收敛且数据有效 | 3 |
| 8 aniso | 1372883 | 20／30／40 K 全部收敛且数据有效 | 11 |

所有作业为 `COMPLETED`、退出 `0:0`。本轮增加 328 节点秒，历史 5482 保留，累计 5810；用公开 `--budget reconcile` 结算后，分配、预留和队列均为空。

实际 SCF／NSCF 输入均采用各自的 40／320 Ry，阈值分别为 1e-6／1e-10 Ry。SCF 输出为 8 个约化点，NSCF 为 32 个完整点；两者密度 FFT 均为 36×36×40，平滑 FFT 均为 24×24×27。NSCF 使用后续已授权 0060 的 `[4,4,2]` 简写及官方格式展开，不能把本轮集成科学证据等同于仅含 0059 差异的提交。

PH 初始化和逐 q 输入分别指向自身配置，`tr2_ph=1e-12`；q2r／色散／DOS 的 ASR、路径和网格均来自各自配置段。电子 DOS／PROJWFC 的 `degauss=0.007349864435130998` 以原生 Ry 写入，能量范围与步长以 eV 写入。

Wannier 原生日志确认 5 个函数、`guiding_centres=true`，解纠缠和局域化均达到原有标准；实际 Outer 为 −3.40453～26.42576 eV，Inner 为 −3.40453～11.16990 eV，四界与自动选窗输入一致。第 7 步独立填写并实际采用 `nstemp=1, temps(1)=20` K，8×8×4／4×4×2 细网格，原生谱的总 λ 为 1.0809057。

第 8 步每个方法实际输入均为自己的 `muc=0.05, nsiter=1000, nstemp=3, temps=[20,30,40]`，`broyden_beta=-0.3`、历史长度 8、初值 `gap_edge=0.01` eV。iso 三个温度分别在 113／131／145 次迭代收敛；aniso 分别为 126／145／198 次。六个点均为 `converged=true, data_status=valid`，没有从另一方法或第七步复制温度。

`--status --json` 返回八阶段均可复用、`workflow_complete=true`。根 `results/` 索引检查返回 `stage_results_verified=true`；该接口按既有设计固定返回 `workflow_complete=false`，只负责阶段发布核验。单独检查 `results/8.epw-superconductivity` 返回 `native_complete=true, native_figures_verified=true, source_verification=verified, workflow_complete=true`。已核对八份额外输入来源记录的实际文件哈希，并实看 QE DOS 与能隙温度图。完整证据在本地 `run-evidence.json`、`final-status.json`、`results-check.json` 与 `gap-results-check.json`。

当前小网格声子比较的最大差为 232.7245 cm⁻¹，图和原始数值保留，不据此宣称网格、选窗或物理结果已收敛。EPW 比较的元素／轨道投影适配仍按既有边界标缺失，QE 第三步元素 PDOS 已交付。本轮满足固定输入下的八阶段与六温度能隙验收，不进行 Tc 搜索，不作为 Tc 验收。
