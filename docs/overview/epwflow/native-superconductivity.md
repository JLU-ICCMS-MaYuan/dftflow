# 原生超导执行适配

截至 2026-10-19，原生 EPW 5.9 的单温度执行、完整虚频轴检查、持久化区间搜索、显式失败续算与原生结果包已有代码及独立进程回归。当前没有真实 MgB₂ 原生 Δ／Z 或 Tc 验收结果；本页不表示公开自动链已经完成。

## 已实现的执行边界

- F4 先核对 F3 清单、原独立质量来源及当前粗网格身份，再绑定原生细网格角色。旧简化 fine 文本不能冒充原生二进制。
- 在 F1 已登记的同一 Slurm 分配内顺序调用 EPW，不另行提交作业。每次新调用前检查分配登记、F3 清单和质量来源；本适配不另建资源账本。
- 一次尝试只含一个模式、一个 centikelvin 温度和一个固定初值；使用独立编号目录，固定 nstemp=1、temps(1)、imag_read=false、μ*、截断、混合与低频规则。
- iso 使用原生 LAPACK 最大本征值分类；另做必需温度和区间超导侧端点的非线性双初值。aniso 使用非线性双初值逐元素分类。Allen–Dynes、BCS 和归一化形状不再作为正式求解路径。
- stdout、stderr、退出状态、执行时间、输入、程序及来源身份随尝试持久化。未收敛、非有限残差、错温度、缺频率、模式冲突或来源变化均不能分类。
- 每模式独立保存搜索区间、方法、阈值、全部尝试和停止原因；使用中点、四分之一、四分之三温度，不用不确定点缩小区间。有效温度复核后复用；失败也消耗计数，已有活动或未知尝试不重提。

## 原生数据含义

F3 交付 freq、egnv、ikmap、npool、ephmat 分片、a2f、crystal.fmt、selecq.fmt 和原生日志的角色、大小及 SHA-256，格式标识为 epw5.9-le-i4-r8。二进制适配限定小端 INTEGER(4)／REAL(8)，不猜测其他布局。

egnv 恢复费米面 k 映射、能量和物理权重；带编号还要核对同源原生日志的 ibndmin／ibndmax，恢复到细网格带索引。原始 imag_aniso 没有 k／band 列，必须按同源映射、频率和能量交叉核对，不能靠行号赋予物理身份。

保存全部正 Matsubara 频率的 Δ（meV）和 Z（无量纲）。双初值只允许统一整体符号翻转，逐元素比较 Δ／Z；D∞ 和 D₀∞ 均取最大绝对值，不用带符号平均抵消证明正常态。带统计保留权重归一化和最低正频率含义。

## 产物与只读检查

阶段根目录为 superconductivity，模式子目录内保存 search.json、attempt-NNNN/epw.in、epw.out、epw.err、attempt.json 及原生能隙。原始进程日志同时由 F1 process 模块保存。

阶段输出 tc.json（版本 2、分模式区间／中点／半宽／停止状态）、temperatures.csv、gaps/ 完整分布及 manifest.json。原生 manifest 使用版本 2，保留版本 1 的读取兼容；原生结果不再填旧的单一 tc 标量。superconductivity check 重验记录与产物身份，不执行科学程序。

## 显式续算与最终验证

显式 resume 沿用原科学目录与全部求解次数；有效尝试重验后复用，失败以新编号重试。中断尝试必须有原 Slurm 分配的明确终态证据；原分配仍活动、状态未知、锁所有者不明或目录含未知文件时拒绝。不会清零失败次数或删除旧现场。已有终态 attempt.json 可在重验后接续登记。

最终阶段门重新解析原生日志及完整 Δ／Z，重放所有有效温度分类，核对 Tc 区间、固定阈值、程序／F3／求解配置、所选模式和必需能隙温度；伪造完成标志、单一模式冒充 both 或仅有包围区间均不能通过。必需能隙温度需要双初值一致的有效超导态。

## 原生结果包与图表

F4 版本 2 交给原生 F5 格式 epwflow-native-results。collect 保存 summary.json、a2f.csv、coupling.csv、gap_summary.csv、gap_states.csv、tc.csv 和 report.md，以及相对路径、大小、哈希的 manifest.json。逐态表保留最低正 Matsubara 频率的 Δ、Z、真实 k／带编号、可用的晶体坐标及物理权重；完整频率数组仍在 F4 的哈希来源中。两模式分别保留区间、中点、半宽、方法与停止状态，未定位时不填写 Tc 估值。

plot 提供原生 α²F／累计 λ、能隙随温度、aniso 加权离散分布、实际 k 索引分布、Tc 区间与总览共六类图，各有 PNG／SVG／PDF。k 索引图不是高对称路径；缺数据明确显示，失败或不确定温度处断线，均值范围不裁掉极值。

结果包可移动后在空 PATH 下重绘；不调用 QE、EPW、sbatch。results check 分开报告本地文件完整性与原始来源可核验性；来源不可访问时为 unavailable，不能称为已验源。native_complete 与 workflow_complete 表示当前所选运行的原生求解／三格式交付，不表示 Epic 的三来源六组合或独立科学参考已验收。比较图与原生图保留各自清单。

collect／plot 通过发布日志记录新旧文件身份，中断时可显式接续；主清单最后原子更新。发现未知目标内容时保留现场并拒绝覆盖。只读检查遇到未完成发布会拒绝，不能把半写状态当作成功。

## 当前科学门与未完成边界

- 用户最新指定的声子最大／RMS 门为 50／20 cm⁻¹。已有五点、45 模实际重验为 49.448071／8.075133，通过新门；电子六点验证窗内四态最大／RMS 为 0.013028／0.008787 eV。阈值变化不代表插值数值改善或全费米面覆盖充分。
- F2 保存树硬编码及 SCF→NSCF／PH 交接、F3 粗网格生成前的质量输出依赖和粗／细重读布局仍有代码阻断；已有原生归集器不表示从全新输入的生产链已接通。
- 真实 LAPACK、正常侧及临界附近收敛、固定能隙温度、独立手动参考与三来源六组合均未验收。
- 本批未运行 9³ PH、新 NSCF、细网格或超导，未覆盖默认安装。真实 MgB₂ 原生 Δ／Z 与 Tc 尚未得到。

源码依据为 superconductivity/engine.rs、verify.rs、manifest.rs 与 delivery/native.rs、native_plot.rs、publication.rs；回归见 superconductivity.rs、real_contracts.rs。设计、验证和待办见 [代码收尾验收](../../specs/0015-epwflow-mgb2-real-validation/native-closeout-validation.md) 与 [0013 任务](../../specs/0013-epwflow-superconductivity-tc/tasks.md)。
