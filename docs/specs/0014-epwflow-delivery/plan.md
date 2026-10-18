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
