# 实施计划：图表、真实对照与安装交付

**Issue**：[本地 0014](issue-draft.md)；**日期**：2026-10-15；**Spec**：[spec.md](spec.md)。

## 摘要与技术上下文

Rust 2021 edition，当前工具链 rustc 1.99.0；现有 Cargo workspace，新增 epwflow 家族 crate。使用 clap、serde、serde_json、toml、anyhow、regex；哈希/进程控制按基础契约实现。科学计算调用现有 QE/EPW，绘图复用 plotters/svg2pdf。存储为受控文件与 JSON，不引入服务或数据库。目标首先 Linux x86_64 的 sugon-amd，macOS arm64 配置保持独立，未实测不写通过。规模是单用户的一次受限验收活动，可跨多个运行目录共享预算。

性能与容量遵循 Spec 的次数、并发、8 节点小时和 100 GiB；无未经测量的速度承诺。真实路径和输出仅存忽略目录。生产实现只用 Rust/Bash，测试可使用仓库既有 Python 测试习惯，但不形成 Python 运行依赖。

## 设计与数据所有权

结果收集从 F2/F3/F4 已校验 manifest 构建版本化 DTO，写 CSV/JSON 与 report.md；空值和有效零分开。表格使用往返精度，所有路径在清单中相对结果包；完整频率原数组只保存源引用，绘图需要的最低频各状态复制到结果包并计入存储。

plotters 共用绘图语义模型，分别导出 PNG/SVG，svg2pdf 生成 PDF；单位和状态由 DTO 给出。每温度、模式、展宽独立身份；图表与数据文件哈希分别保存。缺字体失败，不下载。失败温度断线，缺图面板写状态文字，不能画虚假零值。

collect/plot 采用 staging 后发布 manifest；只替换上次清单拥有的派生文件，遇未知冲突拒绝。plot 完全读取结果包，不调用调度器/科学程序；移动包仍能重绘，但不能把未重新访问上游的来源标为重新验证。

真实验收采用三个 PH 运行各选择 both，分别消费对应 PH/插值数据；六组合记录逐量对照。独立手动参考由手写 QE/EPW 输入建立，输入哈希和预算绑定，同参数参考可有据复用。小网格不是材料收敛；精度、预算、队列失败保持未完成。

安装保持 scripts/install-current.sh 的固定接口，新增第六程序与第52入口；原子替换逐文件，先在 stage 验证全部命令再覆盖。增加有关工作流记录、Slurm 与进程的活动检查，无法查清阻止更新。中途失败报告已切换清单，不自动保存旧二进制或承诺回滚。

真实验证用候选绝对路径，在无活动相关链且全部交付门满足时更新 current。记录源码、未提交差异、构建前后/兼容打包指纹及实际解析入口。Linux/macOS/musl 制品分开，缺环境平台明确未实测。只用安装 help 验收不能关闭科学 Feature。

## 源代码结构

本项新增模块 epwflow/src/delivery/，公开入口由 epwflow/src/main.rs 统一分派；共享状态、预算与路径只由 foundation 拥有，后续模块不得复制规则。测试位于 epwflow/tests/delivery.rs。具体 FR 到文件见下表；尚未存在的文件为明确计划触点，不作为已实现证据。

## Feature 文档结构

spec.md、plan.md、research.md、data-model.md、contracts/interface.md、tasks.md、quickstart.md、checklists/requirements.md、acceptance.md；数据/CLI 均适用，因此不省略条件文档。

## 质量门

| 来源 | 约束 | 满足方式 | 状态 |
| --- | --- | --- | --- |
| 用户确认 | 完整草案与边界 | spec.md 原文/确认记录，保留 FR/SC | 已核查，见共同分析 |
| F1–F5 | 共享契约一致 | 下游链接基础契约；单一账本与阶段状态 | 已核查，见共同分析 |
| AGENTS | 已授权 Rust/Bash、隔离提交 | 不纳入无关 Si 删除；文档完成后自动提交 | 适用 |
| 用户本地约定 | 不操作线上 Issue | 本地 issue-draft 双向链接，不伪造远程验证 | 远程门按用户约定不适用 |
| 真实验收 | 不放宽/不造证据 | acceptance 保持未运行，真实通过后更新 | 尚未执行 |

## 需求到设计与验证的映射

| 来源 | 任务 | 设计组件/接口 | 验证方式 |
| --- | --- | --- | --- |
| FR-001 | T003/T004 | epwflow/src/delivery/cli.rs | epwflow/tests/delivery.rs |
| FR-002 | T005/T006 | epwflow/src/delivery/collect.rs | epwflow/tests/delivery.rs |
| FR-003 | T007/T008 | epwflow/src/delivery/plot.rs | epwflow/tests/delivery.rs |
| FR-004 | T009/T010 | epwflow/src/delivery/report_store.rs | epwflow/tests/delivery.rs |
| FR-005 | T011/T012 | epwflow/src/delivery/scheduler.rs | epwflow/tests/delivery.rs |
| FR-006 | T013/T014 | epwflow/README.md | epwflow/tests/delivery.rs |
| FR-007 | T015/T016 | epwflow/src/delivery/acceptance.rs | epwflow/tests/delivery.rs |
| FR-008 | T017/T018 | epwflow/src/delivery/compare.rs | epwflow/tests/delivery.rs |
| FR-009 | T019/T020 | epwflow/src/delivery/budget.rs | epwflow/tests/delivery.rs |
| FR-010 | T021/T022 | scripts/build-native.sh、scripts/linux/package-native-runtime.sh、Cargo.toml | epwflow/tests/delivery.rs |
| FR-011 | T023/T024 | scripts/install-current.sh、scripts/install-compat.sh、scripts/verify-entries.sh | epwflow/tests/delivery.rs |
| FR-012 | T025/T026 | epwflow/src/delivery/regression.rs | epwflow/tests/delivery.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/delivery.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/delivery.rs 与真实/安装证据 |

## 阶段与依赖

准备和基础协议先完成，随后按 tasks 的测试→实现→验证顺序执行。当前 Feature 完成独立验收、quickstart、Overview 回写后才推进依赖阶段；不得用尚未实现的下游编排替代本项接口验收。全部文档门通过前禁止业务代码。

## 必要复杂度

| 复杂度 | 原因 | 拒绝的简化 |
| --- | --- | --- |
| 版本化清单和依赖指纹 | 固定路径不保证固定内容，复用有真实数据风险 | 仅检查文件存在或 JOB DONE |
| 两种状态：操作与科学结果 | 提交/绘图成功不等于科学成功 | 一个完成布尔值 |
| 独立参考与固定阈值 | 相同错误流程可产生自洽假结果 | 用自己输出生成唯一期望值 |

## 2026-10-19：四类叠图交付设计

新增 FR-014/015 是 epwflow 的交付部分，F3 按 [0012 契约](../0012-epwflow-interpolation/contracts/interface.md) 唯一生产成对数据和投影；本项不计算投影、不另建 QE 科学流程。优先复用现有 QE／Wannier 能带比较、qf-phonon 数据表示与 plotters/svg2pdf，只新增统一比较数据读取和绘制语义。

collect 从插值 comparison 清单即可构建自包含的 results/comparison，不强制读取不存在的 F4 清单；正常全链同时归集已有 α²F、能隙和 Tc。数据完整性、科学质量、图像生成、workflow_complete 分别记录。质量失败时允许明确标记的诊断结果，不能据此调度细网格 EPC／超导。未知／篡改数据拒绝，缺少已声明的投影只输出缺项状态和其余有效面板，不生成替代曲线。

同一绘图语义模型生成四图和 2×2 总览。每个元素／轨道保持两侧同色系，QE 实线先绘，EPW 虚线最后绘；直接 ph.x 使用独立标记，不与连续 matdyn 曲线混称。图例包括方法、投影与来源；共同能窗、单位和断点来自元数据，不在 plot 阶段重新确定。线段相交、完全重合及简并分支均保留原始点序，不做视觉拟合。

表格、图种和图层顺序可自动检查；PNG／SVG／PDF 的文字、虚线、图例与遮盖效果须实际查看。缺数据面板显示明确状态，科学表格与源文件在重绘前后哈希不变。输出按既有 staging、清单所有权和冲突拒绝机制发布，不递归清空用户目录。

| 来源 | 任务 | 设计组件／数据 | 验证 |
| --- | --- | --- | --- |
| FR-014、SC-009 | T039/T041/T042 | delivery 绘图语义模型、四图和总览 | 图层／线型测试与三种格式实际查看 |
| FR-015、SC-010 | T039/T040/T042 | collect、独立 comparison 包、结果检查 | 无 F4 诊断导出、移动重绘、零外部调用、哈希不变 |
| FR-014/015、SC-011 | T043 | 自动交付与质量门、0015 三来源证据 | 部分失败恢复、不误放行、六组合完成条件不变 |

## 2026-10-19：比较绘图已实施子集

comparison.rs 收集 F3 的独立数据，comparison_plot.rs 复用同一个 Plotters 绘图模型输出 PNG／SVG，SVG 经 svg2pdf 生成 PDF。每个面板先画全部 QE 曲线再画 EPW 像素虚线；线段按能窗求交，不压平窗外数据。图像与清单暂存后发布，独立结果包可移动重绘。原子投影、完整结果包和全流程调度仍按原计划补齐，本轮不新增并行模型或声称这些能力已存在。


## 2026-10-19：本轮执行范围变更

元素／轨道分面真实验收按用户最新指令暂缓，已有未完成项不勾选；恢复时仍遵守原可靠性契约。当前先完成总量叠图、声子 15/2 cm⁻¹ 质量复核、固定补充网格外点，再进入已指定细网格和原生求解。只改最大误差，RMS／虚频／覆盖／求解与资源标准不变；历史失败报告不重写。共用质量阈值与原生报告记录、边界／RMS 拒绝回归和真实续算证据见本次验收追加段。

## 2026-10-19：当前标注实现与验证计划

当前执行以 Spec 最新 50/2 cm⁻¹ 为准，先前 15/2 记录作为历史保留。F3 从现有质量成对频率计算采样／路径最大，使用倒格矢对偶基恢复晶体坐标；F5 消费带哈希标注并绘制紫点与独立诊断区，0015 复核真实模式和三格式。回归覆盖非正交转换、路径内外分类、排序模式、标注语义拒绝、50 边界及 RMS 失败。使用已有结果重绘，不新增科学作业，不越过失败质量门。

## 2026-10-19：外窗渲染与迁移

F5 只消费 F3 的外窗元数据；电子能带纵轴及 DOS 横轴采用相同 E−E_F 边界，线段在边界求交裁剪，不压平或改写科学数据；独立图及总览标题注明实际绝对窗。T046 对应 FR-014/015、SC-009/010。迁移先核验无活动任务及全部文件 SHA-256，逐运行目录改名，重新定位历史符号链接并保存旧→新映射；新来源清单递归重绑定引用及其哈希，旧清单原样保留。当前入口和手册使用新路径，原生科学子目录不任意重命名。


## 2026-10-19：底部高对称点标签

FR-014／SC-009 复用 comparison_plot::chart：仅电子能带和声子色散关闭自动横轴数值标签及网格，保留高对称点分隔线；将符号从数据坐标转换为屏幕坐标，在底部轴外的预留区域居中绘制，避免子面板偏移和绘图区裁剪。路径中的 G 显示为 Γ；其他名称沿用元数据。DOS 不进入该分支。通过实际 CLI 的 SVG 标签位置／数值刻度检查及真实 MgB₂ 三格式重绘验证，科学数据哈希保持。


## 2026-10-19：窗口边界的数据与绘制

F3 复用单一窗口解析器读取已核验 wout 的 Outer／Inner，将绝对边界写入比较元数据；frozen_window_ev 为可选字段，旧包无字段继续可读，但不会补造四条边界。加载时检查冻结窗包含关系。F5 只读元数据，用共同 E_F 将四边界定位到能量轴：能带水平、DOS 竖直；复用绘图函数服务独立图与总览。四个颜色的虚线和图例分别识别参数，相同能量在原坐标按像素相位交错绘制，避免遮盖。CLI 回归检查实际 SVG 的方向、位置、颜色、图例、重合和旧包；用真实 MgB₂ 只读重导出并查看三格式。

## 2026-10-19：费米能级的绘图参考

复用 comparison_plot::chart，为电子面板增加绿色 E_F 参考线与图例／文字；已有数据契约规定电子能量为 E−E_F，因此位置固定为 0 eV，不再减一次绝对 E_F。移除能带中不带标签的浅灰零线，独立图和总览共用相同分支；不增加输入字段或科学数据读取。扩展真实 CLI 的 SVG 坐标回归检查水平／竖直位置及声子图无此标识，用已有 MgB₂ 结果包在新编号目录只读重绘，核对表格哈希与三格式。

## 2026-10-19：50／20 门与原生代码收尾

对应 FR-001～005、SC-001/003/004。F4 版本 2 通过独立原生结果包交付，不压缩成旧单 Tc 标量。包内保存分模式搜索、最低正频率逐态 Δ／Z／k 坐标／物理权重、带统计、原生谱与累计 λ；完整 Matsubara 数组保留 F4 哈希来源。六类图均由 Rust 消费包内表，支持 PNG／SVG／PDF 和移动后空 PATH 重绘。显式 collect／plot 通过发布日志恢复半写文件；只替换已登记且哈希匹配的旧文件，未知文件拒绝覆盖。

科学求解与真实验收共用原预算；9³ PH 暂缓，不新增需求或另建线上 Issue。阈值调整不表述为插值改进，真实 Tc、独立手动参考与完整自动链仍须各自提供证据。
