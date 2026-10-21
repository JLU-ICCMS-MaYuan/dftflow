# 原生超导执行适配

2026-10-22 当前 PBE 公开八阶段实测已完成：iso／aniso 的 20／30／40 K 六点均为原生退出 0、明确收敛、完整结束且能隙数据有效；最终结果包 execution_complete、data_complete 和 workflow_complete 均为 true。原始证据与边界见 [0045 验收](../../specs/0045-epw-gap-temperatures/acceptance.md)。

当前原生 EPW 5.9 适配支持指定温度能隙（gap）和原有区间搜索（tc），两类任务分别验证和交付。当前算例目标为 PBE MgB₂ 的 iso／aniso 20／30／40 K 正常执行；真实公开流程结果见 [0045 验收](../../specs/0045-epw-gap-temperatures/acceptance.md)。下文注明日期的 LDA 结果仅为历史证据，不属于本次要求。

## 已实现的执行边界

- F4 先核对 F3 清单及当前粗细网格身份，再绑定原生细网格角色。旧简化 fine 文本不能冒充原生二进制；常规比较差值沿用 0043 的记录语义，不以历史抽样数值门阻断。
- 编号工作区的公开 `-s` 每次最多提交一次原生调用，作业结束后重复命令核验并推进；既有 `-r` 保留已登记分配内的执行语义。每次新调用检查来源和原共享预算，不另建资源账本。
- 一次尝试只含一个模式、一个 centikelvin 温度和一个固定初值；使用独立编号目录，固定 nstemp=1、temps(1)、imag_read=false、μ*、截断、混合与低频规则。
- 显式重启开关跳过 PWSCF 初始化；两模式均把已有 α²F 复制为带哈希的本地 `source.a2f`，避免目标版本 80 字符字段截断或隐式重算谱。
- tc 任务的 iso 使用原生 LAPACK 最大本征值分类，另做必需温度和区间超导侧端点的非线性双初值；aniso 使用非线性双初值逐元素分类。Allen–Dynes、BCS 和归一化形状不作为正式求解路径。
- stdout、stderr、退出状态、执行时间、输入、程序及来源身份随尝试持久化。未收敛、非有限残差、错温度、缺频率、模式冲突或来源变化均不能分类。
- tc 任务每模式独立保存搜索区间、方法、阈值、全部尝试和停止原因；使用中点、四分之一、四分之三温度，不用不确定点缩小区间。有效温度复核后复用；失败也消耗计数，已有活动或未知尝试不重提。

## 指定温度能隙任务

在 TOML 的 `[superconductivity]` 设置 `task="gap"`，由 mode 与 gap_temperatures_k 构成固定计划。每个点仅一个 initial_gap_mev，缺省为 10 meV；不要求 Tc 区间、精度或 initial_gaps_mev 双初值。每次均设置 nstemp=1、目标 temps(1)、tc_linear=false，完整保存请求和来源。省略 task 时保留旧 tc 验证、清单读取和序列化身份。

温度必须有限、为正、不重复、精确到 0.01 K 且不大于 999.99 K，频率截断须包含首个 Matsubara 频率。mixing_beta 负值采用原生线性混合，正值为 Broyden，绝对值在 `(0,1]`；混合历史仍限制为 1～8。当前 PBE 示例显式设置 -0.3，不暗中改变混合、初值或收敛标准。

正常结束需要原生退出 0、EPW 5.9 标识、完整结束计时且没有原生错误。清单分别记录 execution_complete 与 data_complete；每点 gap_status 区分明确收敛、未收敛／证据不明确、有效、无效或缺能隙。正常结束但数据无效的点保存原因并继续，不能填零冒充有效数据；真实零能隙允许作为有效结果。非零退出、缺正常结束、预算或来源失败停止，显式 --resume 以新尝试补未完成点。成功点、失败日志及退出码保留，活动或未知任务不能重提。

编号布局在 `8.epw-superconductivity/` 保存版本 3 的 manifest.json、固定计划及模式／温度尝试目录，每点有 epw.in、submit.sh、epw.out、attempt.json 及可用的原生 Δ／Z。完成检查重新解析原生证据并核对程序、配置、F3 和文件身份。第 7 步仍使用温度列表首项生成输入；首项变更时须按身份重算细网格，不能修改旧哈希强行复用。

结果包输出 points.csv 逐点执行／收敛／数据状态，以及有效的 gap_summary.csv、gap_states.csv。有效单初值 aniso 可进入分布；不要求双初值 Tc 分类、不插值温度，不生成 tc.csv 或 Tc 区间图。完整正 Matsubara 频率仍在 F4，统计和逐态表使用最低正频率，Δ 单位为 meV、Z 无量纲。执行全部结束即可完成本次任务，数据有效性另列，不表示已经确定 Tc 或通过独立科学精度验收。

源码依据：superconductivity/points.rs、attempt.rs、verify.rs、foundation/program.rs、workspace_science.rs、delivery/native.rs 和 native_plot.rs；公开八步、三模式、无效数据、恢复及移动重绘证据见 [0045](../../specs/0045-epw-gap-temperatures/acceptance.md)。

## 原生数据含义

F3 交付 freq、egnv、ikmap、npool、ephmat 分片、a2f、crystal.fmt、selecq.fmt 和原生日志的角色、大小及 SHA-256，格式标识为 epw5.9-le-i4-r8。二进制适配限定小端 INTEGER(4)／REAL(8)，不猜测其他布局。

egnv 恢复费米面 k 映射、能量和物理权重；带编号还要核对同源原生日志的 ibndmin／ibndmax，恢复到细网格带索引。原始 imag_aniso 没有 k／band 列，必须按同源映射、频率和能量交叉核对，不能靠行号赋予物理身份。

保存全部正 Matsubara 频率的 Δ（meV）和 Z（无量纲）。双初值只允许统一整体符号翻转，逐元素比较 Δ／Z；D∞ 和 D₀∞ 均取最大绝对值，不用带符号平均抵消证明正常态。带统计保留权重归一化和最低正频率含义。

## Tc 任务的产物与只读检查

阶段根目录为 superconductivity，模式子目录内保存 search.json、attempt-NNNN/epw.in、epw.out、epw.err、attempt.json 及原生能隙。原始进程日志同时由 F1 process 模块保存。

阶段输出 tc.json（版本 2、分模式区间／中点／半宽／停止状态）、temperatures.csv、gaps/ 完整分布及 manifest.json。原生 manifest 使用版本 2，保留版本 1 的读取兼容；原生结果不再填旧的单一 tc 标量。superconductivity check 重验记录与产物身份，不执行科学程序。

## 显式续算与最终验证

显式 resume 沿用原科学目录与全部求解次数；有效尝试重验后复用，失败以新编号重试。中断尝试必须有原 Slurm 分配的明确终态证据；原分配仍活动、状态未知、锁所有者不明或目录含未知文件时拒绝。不会清零失败次数或删除旧现场。已有终态 attempt.json 可在重验后接续登记。

tc 任务最终阶段门重新解析原生日志及完整 Δ／Z，重放所有有效温度分类，核对 Tc 区间、固定阈值、程序／F3／求解配置、所选模式和必需能隙温度；伪造完成标志、单一模式冒充 both 或仅有包围区间均不能通过。tc 的必需能隙温度需要双初值一致的有效超导态。

## 原生结果包与图表

tc 任务的 F4 版本 2 交给原生 F5 格式 epwflow-native-results。collect 保存 summary.json、a2f.csv、coupling.csv、gap_summary.csv、gap_states.csv、tc.csv 和 report.md，以及相对路径、大小、哈希的 manifest.json。逐态表保留最低正 Matsubara 频率的 Δ、Z、真实 k／带编号、可用的晶体坐标及物理权重；完整频率数组仍在 F4 的哈希来源中。两模式分别保留区间、中点、半宽、方法与停止状态，未定位时不填写 Tc 估值。

plot 提供原生 α²F／累计 λ、能隙随温度、aniso 加权离散分布、实际 k 索引分布、Tc 区间与总览共六类图，各有 PNG／SVG／PDF。aniso 分布以温度为横轴、最低正 Matsubara 的绝对能隙为纵轴，0.05 meV 归箱后用横向宽度表示每温度归一化的费米面权重；只取已通过双初值分类的一套固定初值，不画失败／不确定温度，不做温度插值或未经验证的 σ／π 归属。k 索引图不是高对称路径；缺数据明确显示，失败或不确定温度处断线，均值范围不裁掉极值。

结果包可移动后在空 PATH 下重绘；不调用 QE、EPW、sbatch。results check 分开报告本地文件完整性与原始来源可核验性；来源不可访问时为 unavailable，不能称为已验源。native_complete 与 workflow_complete 表示当前所选运行的原生求解／三格式交付，不表示 Epic 的三来源六组合或独立科学参考已验收。比较图与原生图保留各自清单。

collect／plot 通过发布日志记录新旧文件身份，中断时可显式接续；主清单最后原子更新。发现未知目标内容时保留现场并拒绝覆盖。只读检查遇到未完成发布会拒绝，不能把半写状态当作成功。

## 历史科学门与未完成边界（2026-10-19）

以下为当时的 LDA 验收记录，保留原结论；独立抽样及阈值已由 0043 的常规比较取代，不构成当前 PBE gap 任务的前置要求。最新实现与验证以前述 0045 和模块入口为准。

- 当时的声子最大／RMS 门为 50／20 cm⁻¹。已有五点、45 模实际重验为 49.448071／8.075133，通过当时门；电子六点验证窗内四态最大／RMS 为 0.013028／0.008787 eV。阈值变化不代表插值数值改善或全费米面覆盖充分。
- F2 已修复实际 prefix、完整快照、NSCF／PH 独立副本和原生 PH 交接，三模式进程及已有 LDA 216 k／28 q 只读核验通过；新 F2 版本 2 交付由 F1／F3 重验。F3 已接通 F2 数据复制、内置 Wannier、先生成后配对的质量检查及分展宽重启交接，进程回归和既有 LDA 粗重启只读核验通过。现有安装的选窗关键字兼容、历史粗网格受控导入、F2/F3 阶段内恢复仍未收敛；真实细网格的生产函数已完成阶段验收，见下文。
- 真实 iso LAPACK 已完成 9 个有效温度及 0.35 K 宽区间；正常侧非线性收敛、固定能隙温度、独立手动参考与三来源六组合仍未验收。
- 已用生产重读／细网格函数完成既有 LDA 规范的 40³/20³ EPC，随后真实启动两模式超导；iso 10 K 和 aniso 55 K 的首次非线性尝试均未通过固定的 500 次／10⁻⁸ 相对残差门。未运行 9³ PH 或新 SCF／NSCF／PH，未覆盖默认安装。

源码依据为 superconductivity/engine.rs、verify.rs、manifest.rs 与 delivery/native.rs、native_plot.rs、publication.rs；回归见 superconductivity.rs、real_contracts.rs。设计、验证和待办见 [代码收尾验收](../../specs/0015-epwflow-mgb2-real-validation/native-closeout-validation.md) 与 [0013 任务](../../specs/0013-epwflow-superconductivity-tc/tasks.md)。

F2 当前代码事实与真实文件证据见 [原生 QE 交接验收](../../specs/0015-epwflow-mgb2-real-validation/native-qe-handoff-validation.md)。仅有版本 1 F2 清单可历史读取但不能供 F3 执行；版本 2 逐文件核对完整保存集合、原生数值与 q 响应，不以 valid 标签替代身份。

F3 当前路径将每个展宽写入 fine/smearing-NNNN，F4 使用 spectrum_index 所选目录；其余展宽文件仍带前缀角色保留并验源。质量来源在粗网格前冻结、粗网格后派生并记录身份，F1/F4 重验同一绑定。粗网格不再调用独立不完整的 Wannier 链，也不要求虚构 coarse.ephmat；细网格不回退简化文本，并用最终写入计数检查矩阵负载。代码与只读证据及未完成项见 [原生插值验收](../../specs/0015-epwflow-mgb2-real-validation/native-interpolation-validation.md)。


## LDA 细网格的真实阶段证据

生产函数重读同源粗网格时包含原 `.ukk`，核查完整带流形和旋转记录；先重新生成全部六电子点／五声子点并通过质量门，再执行 40³/20³。细网格显式使用首个固定温度 10 K，重新生成选择文件，分别核对所选 q=7971 和完整 q=8000。24 个矩阵分片、共 33 份细网格角色及 500 点 α²F 已通过真实产物重读，λ_integral=0.6853853、λ_sum=0.6853511，保留原生两种数值。

该阶段使用生产函数的显式本机测试，未发布缺失 F2 来源的自动阶段清单。公开入口的历史导入／恢复和完整 Tc 验收仍未完成；不得把日志中的经验 Tc 估计当作 Eliashberg Tc。实际故障与修复、只读恢复、原生数据及预算证据见 [细网格验收](../../specs/0015-epwflow-mgb2-real-validation/native-fine-validation.md)。

## 真实超导阶段的部分结果

同一原生细网格、μ*=0.05、截断 0.5 eV 下，iso 在 36.71 K 的最大本征值为 1.0033547，37.06 K 为 0.9974889；区间中点 36.885 K、半宽 0.175 K 仅表示线性化区间估值。必需非线性双初值尚未有效，iso `complete=false`；aniso 没有有效分类区间。未收敛小能隙、平均能隙和原生经验公式均不能补成有效正常态或正式 Tc。

原生谱路径故障已修复，显式恢复保留原失败／中断尝试和次数，最终阶段报告仍为未完成。Mori 图需要有效的逐态最低正 Matsubara 分布，当前不能用未收敛输出绘制已验收的能隙云图。来源、数值与预算证据见 [真实 Tc 验收](../../specs/0015-epwflow-mgb2-real-validation/native-tc-validation.md)。
