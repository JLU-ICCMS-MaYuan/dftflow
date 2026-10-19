# 指定工作流对 epwflow 与 wannierflow 的复用审计

## 结论与证据范围

本次逐项阅读固定在 `1c0115cb7c77541a0bd549bc05005df435d651b6` 的指定子仓库，并核对本地生产源码。选窗迁移的精确行为见 [采用说明](upstream-adoption.md)。下表是可借鉴项与实际差距；“发现”表示源码审计结论，不表示已经运行真实程序复现，更不表示已经修复。

优先顺序按“影响正确执行 → 影响选窗输入 → 影响质量选择 → 影响效率”安排。以下建议不自动升级为新 Feature 或已确认任务，不另建线上 Issue。本次已落实共享选窗及表中第 2／3 项的脚本失败传播和模板空值修复；其余内容仍按各行范围验收。

## 优先改进项

| 顺序 | 上游做法与来源 | 本地证据／影响 | 建议与验收重点 |
| --- | --- | --- | --- |
| 1 | [主工作流](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/wannier90.py) 分开执行并检查 `-pp → pw2wannier90 → wannier90`，衔接实际父计算目录 | `epwflow/src/interpolation/wannier.rs::run_wannier` 写 `scf.win`，新建空 `out`，未执行 `-pp`，将 `scf` 作为 pw2 命令参数且未传入已生成的 `pw2wan.in`；`epw_config.rs` 的 seedname 来自配置 prefix，可能与固定 `scf` 不一致 | 先修正独立路径的数据绑定、统一 seedname 与执行顺序；用需要真实 `.nnkp`、stdin 和 `.save` 的边界测试验证，前序失败后不得产生后序调用。不能借此建立另一套重复的 EPW 科学链；与既有 0012／0015 衔接 |
| 2 | 同一主工作流每步都有运行与检查，并传播明确失败 | `wannier90flow/src/init.rs::create_run_script` 顺序正确，但生成脚本无失败即停控制；直接执行路径 `run_wannier90_pipeline` 已使用错误传播，两条路径行为不一致 | 借鉴逐步检查；验证 `-pp` 或 pw2 返回非零时 Slurm 脚本同样失败且不启动下一程序，不只检查脚本文本 |
| 3 | [pw2 基础工作流](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/base/pw2wannier90.py) 按 SCDM／PAO 路径设置投影生成与矩阵输出 | `wannier90flow/src/init.rs::ensure_pw2wan_input` 保留模板空值并格式化成 `'None'`，包括 prefix／seedname 及可选布尔项；默认生成输入可能非法 | 将必填身份字段与可选参数分开验证；缺省或空值不能写成 Fortran 的 `'None'`，已有合法显式值仍应保留。真实临时输入回读并验证生成器行为 |
| 4 | [运行时选窗](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/base/wannier90.py#L562) 使用实际能带及排除映射，两个入口共用规则 | epwflow 的 `EpwConfig` 只有配置／E_F；`epwflow/src/qe_data/nscf.rs::parse_xml` 已有本征值解析可复用。`wannier90flow/src/window.rs` 仍按带号估计，自动模式直接访问 `eng_full[0]`，空数据可触发 panic | 本需求的主路径：先建立来源一致的本征值／投影输入，再共享上游窗口算法；拒绝空数据、形状错误和无效排除带，验证最终原生输入确实使用计算结果 |
| 5 | [轨道与电子数核查](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/wannier90.py#L1394) 对比赝势与程序输出 | 当前固定五维基底不等于上游默认 PAO；`crates/qf-io/src/qe_proj.rs` 有投影读取能力，但不能凭其存在认定 NSCF 身份和规范已匹配 | 核查 UPF 轨道／电子数、实际投影数、k 点顺序与带数；保存投影集合和通道，防止错源投影产生貌似正常的窗口。不改变已暂缓的元素／轨道 DOS 展示范围 |
| 6 | [空带数估计](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/utils/pseudo/__init__.py#L292) 同时考虑电子数与投影数，并增加空带余量 | 当前 MgB₂ 有固定 16 条带；只按占据数或手填 nbnd 未必覆盖所需投影空间 | 借鉴可解释的 nbnd 建议与不足检查，保留用户显式设置。不能为了套用协议自动改赝势、基底或触发新 NSCF |
| 7 | [优化工作流](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/optimize.py) 保留初始解、有限候选与早停；有参考时优先能带误差 | 本地暂未形成投影阈值优化与真实独立质量的闭环；仅 spread 收敛不能保证目标能区准确 | 有界优化可以借鉴，但属于后续实验决策。先固定候选和预算、保留全部尝试；所有新候选更差时保留原始解。不能自动执行上游默认 30 组扫描 |
| 8 | [错误处理](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/workflows/base/wannier90.py#L710) 针对 bvectors、投影筛选态数不足、波函数绘图和实际 OOM 分类处理 | DFTFlow 已有错误／预算基础机制，窗口层还未连接这类有限修复；不能把所有不完整日志都称为 OOM | 只对识别明确的错误做有限重试，每次保留参数变化和预算；投影阈值自动调整应出现在选择记录。未知错误立即保留失败；优化扫描期间不暗中改阈值 |
| 9 | 上游选窗使用计算所得 Fermi／带边，而非绘图默认值 | `wannier90flow/src/bin/wannier90.rs` 的 plot Fermi 默认 `23.3313 eV` 是常量 | 借鉴明确能量来源；新增自动读取时应显示来源与覆盖优先级，避免选窗、能带和 DOS 使用不同零点 |

## 不能原样照搬的内容

### 质量排序不能替换当前验收

[能带距离源码](https://github.com/JLU-ICCMS-MaYuan/aiida-wannier90-workflows/blob/1c0115cb7c77541a0bd549bc05005df435d651b6/src/aiida_wannier90_workflows/utils/bands/distance.py) 使用两侧 Fermi 分布或 Gaussian 的组合权重；其 `max_dist` 是加权误差最大值的平方根，不是未加权的最大绝对误差。对比时按排除带后的前 N 条 DFT 带匹配；不能认为它已经实现任意目标流形的完整态匹配。

优化工作流的 `get_bands_distance_ef2` 选择 E_F+2 eV 的距离项，可作为候选排序的研究参考。DFTFlow 既有电子最大／RMS 0.05／0.02 eV 双门、固定样本及完整态匹配职责不被替换；无参考时按 spread 方差排序也不能宣称科学质量通过。

### 协议参数不能替换物理基线与预算

上游 `moderate` 协议包含 SSSP／PBEsol 赝势族、从赝势推导的 PAO 基底、k 点密度、空带倍数和 12 小时作业上限。用户明确要求完全采用的是选窗方案，并未要求改变当前 LDA／MgB₂ 基底或扩大共享资源限额。以上参数仅用于研究适用性，不能成套覆盖项目配置。

### 自动恢复需要保留物理输入与状态身份

上游有有限 `kmesh_tol` 调整、降低 `dis_proj_min`、增加绘图 supercell 和 OOM 后减少 MPI 等恢复策略。每项会改变的输入与实际程序能力必须可审计；不能把恢复后的结果冒充原配置，也不能复用与新规范不匹配的矩阵元／检查点。

## 交付取舍

本次子模块及调研文件可独立复核；生产默认模式、共享实现接口、真实验证与后续修复仍应分别验收。优先完成 0016 的选窗接入；前述独立执行链和脚本问题属于已有关联 Feature 的审计发现，后续实施时按实际职责挂接，避免把所有优化建议堆入一个不可验收的任务。

本次已经接入共享选窗、删除旧自动算法，并修复 wannierflow 的脚本失败传播及模板空值；验证见 [验收](acceptance.md)。epwflow 独立科学链缺口、投影身份增强、nbnd 建议与优化扫描没有被本次局部修复冒充完成；未运行新 QE／EPW、调整安装、改变质量门或关闭需求。
