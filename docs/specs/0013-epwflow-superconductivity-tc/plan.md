# 实施计划：两种超导求解与受限 Tc 搜索

> **本次执行计划覆盖（2026-10-21）**：用户最新仅要求 PBE 的 20、30、40 K iso/aniso 原生执行完成，取消先做有界诊断、Tc 搜索、双初值和独立物理参考。原完整 Tc 计划保留历史设计语境，不驱动本次作业。执行与验收见 [固定温度点记录](../0015-epwflow-mgb2-real-validation/temperature-points-validation.md)；本次不改变生产 Tc 检查器的语义。

**Issue**：[本地 0013](issue-draft.md)；**日期**：2026-10-15；**Spec**：[spec.md](spec.md)。

## 摘要与技术上下文

Rust 2021 edition，当前工具链 rustc 1.99.0；现有 Cargo workspace，新增 epwflow 家族 crate。使用 clap、serde、serde_json、toml、anyhow、regex；哈希/进程控制按基础契约实现。科学计算调用现有 QE/EPW，绘图复用 plotters/svg2pdf。存储为受控文件与 JSON，不引入服务或数据库。目标首先 Linux x86_64 的 sugon-amd，macOS arm64 配置保持独立，未实测不写通过。规模是单用户的一次受限验收活动，可跨多个运行目录共享预算。

性能与容量遵循 Spec 的次数、并发、8 节点小时和 100 GiB；无未经测量的速度承诺。真实路径和输出仅存忽略目录。生产实现只用 Rust/Bash，测试可使用仓库既有 Python 测试习惯，但不形成 Python 运行依赖。

## 设计与数据所有权

温度用整数 centikelvin，避免浮点键和文件名舍入冲突。每个 mode/type/T/seed/attempt 独立目录，输入固定 nstemp=1 与一个 temps。所有求解只读 F3 数据，原始 EPW 文件保留；不跨温度 imag_read 热启动。

iso 搜索调用目标 crit_temp_iso 的 LAPACK 路径，解析温度、本征值、频率数和成功返回，记录未提供非线性 residual；常规非线性能隙另跑两初值。不得因 LAPACK 行的迭代数为 1 而杜撰残差。目标源码存在版本相关边界，真实运行探测与固定输出 fixture 必须验证输出语法和失败传播。

aniso 每分类点独立跑 1/10 meV 两初值，严格要求原生 converged 和 finite residual<1e-8。读取完整 imag_aniso 数组，按 freq→k→band 及 F3 的 fsthick mask 还原索引，核对记录数、频率和能量，不按行号猜物理身份。Δ/Z 输出来自原始精度；仅比较整体符号，不逐点取绝对值掩盖解差异。

Tc 搜索保存两模式独立 interval/attempt map，正常二分；中点分类不确定时按 1/4、3/4 有界探测，不用无效点更新端点。每次决策写入 journal 后才安排新作业，未知提交保留次数。原生失败终止受影响模式，显式 resume 才重试；反常温度分类标 inconsistent。次数包括 seed/linear/gap/retry，不等于温度数。

风险：原生相对残差在零解附近未必收敛；本项不能用小能隙补救成 normal。真实验收必须取得两个可靠侧及独立邻近温度，不满足则保持未完成，不改科学求解器。提前进行低成本正常侧验证以防在全部组合上浪费预算，但不改变容差。

结果按温度保留数值状态和分类状态；located 只有区间宽度<=0.5K且端点有效。gap_temperatures 中的必需解失败，即使 Tc located 也只记部分结果。F5 消费结构化文件，不重新推断临界温度。

## 源代码结构

本项新增模块 epwflow/src/superconductivity/，公开入口由 epwflow/src/main.rs 统一分派；共享状态、预算与路径只由 foundation 拥有，后续模块不得复制规则。测试位于 epwflow/tests/superconductivity.rs。具体 FR 到文件见下表；尚未存在的文件为明确计划触点，不作为已实现证据。

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
| FR-001 | T003/T004 | epwflow/src/superconductivity/cli.rs | epwflow/tests/superconductivity.rs |
| FR-002 | T005/T006 | epwflow/src/superconductivity/solver_config.rs | epwflow/tests/superconductivity.rs |
| FR-003 | T007/T008 | epwflow/src/superconductivity/attempt.rs | epwflow/tests/superconductivity.rs |
| FR-004 | T009/T010 | epwflow/src/superconductivity/parse.rs | epwflow/tests/superconductivity.rs |
| FR-005 | T011/T012 | epwflow/src/superconductivity/gap.rs | epwflow/tests/superconductivity.rs |
| FR-006 | T013/T014 | epwflow/src/superconductivity/iso.rs | epwflow/tests/superconductivity.rs |
| FR-007 | T015/T016 | epwflow/src/superconductivity/aniso.rs | epwflow/tests/superconductivity.rs |
| FR-008 | T017/T018 | epwflow/src/superconductivity/search.rs | epwflow/tests/superconductivity.rs |
| FR-009 | T019/T020 | epwflow/src/superconductivity/scheduler.rs | epwflow/tests/superconductivity.rs |
| FR-010 | T021/T022 | epwflow/src/superconductivity/reuse.rs | epwflow/tests/superconductivity.rs |
| FR-011 | T023/T024 | epwflow/src/superconductivity/manifest.rs | epwflow/tests/superconductivity.rs |
| FR-012 | T025/T026 | epwflow/src/superconductivity/compare.rs | epwflow/tests/superconductivity.rs |
| FR-013 | T027/T028 | epwflow/README.md | epwflow/tests/superconductivity.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |
| SC-008 | T036 | acceptance.md | epwflow/tests/superconductivity.rs 与真实/安装证据 |

## 阶段与依赖

准备和基础协议先完成，随后按 tasks 的测试→实现→验证顺序执行。当前 Feature 完成独立验收、quickstart、Overview 回写后才推进依赖阶段；不得用尚未实现的下游编排替代本项接口验收。全部文档门通过前禁止业务代码。

## 必要复杂度

| 复杂度 | 原因 | 拒绝的简化 |
| --- | --- | --- |
| 版本化清单和依赖指纹 | 固定路径不保证固定内容，复用有真实数据风险 | 仅检查文件存在或 JOB DONE |
| 两种状态：操作与科学结果 | 提交/绘图成功不等于科学成功 | 一个完成布尔值 |
| 独立参考与固定阈值 | 相同错误流程可产生自洽假结果 | 用自己输出生成唯一期望值 |

## 2026-10-19：本批实现映射

- F3 的 native_matrices.rs 负责原生二进制、费米面压缩映射和带窗口偏移；fine.rs 负责实际角色归集，不另建电子态／PH 科学执行链。
- F4 attempt.rs 负责单温度输入、独立进程和失败现场；parse.rs／gap.rs 验证日志、残差、全部正频率与状态。source.rs 只绑定 F3 原生角色及哈希。
- iso.rs／aniso.rs 只消费已验证的原生量分类，search.rs 管理整数温度区间，engine.rs 持久化尝试计数和复用。scheduler.rs 保持独立质量门并复用 F1 分配／账本。
- 原生结果清单和 tc.json 使用版本 2；既有版本 1 只作有界读取兼容。分模式结果不能压成旧的单一 Tc。
- 失败显式重试、跨目录迁移、自包含 F5 与统一入口最终验收仍未收敛，不将本批进程 fixture 当作目标程序的科学运行验证。

## 2026-10-19：50／20 门与原生代码收尾

对应 FR-008～011、SC-004/005/008。显式 resume 保留科学根目录、旧尝试、所有失败次数及固定身份，重建可继续的搜索状态；未知尝试仅在原分配明确终态后解除。新尝试使用新编号目录。最终阶段门重新解析原生日志与完整 Δ／Z、重放分类与区间，并校验固定配置、所选模式和必需能隙温度，不能只相信 complete 布尔值。

科学求解与真实验收共用原预算；9³ PH 暂缓，不新增需求或另建线上 Issue。阈值调整不表述为插值改进，真实 Tc、独立手动参考与完整自动链仍须各自提供证据。

## 原生细网格之后的单来源验收

复用已通过质量门的固定原生细网格，先探测正常侧再搜索；探测属于同一生产 Engine 和次数记录。F4 输入显式跳过 QE 初始化。阶段报告和完整 F3 共用原生角色、布局、路径及哈希校验，但阶段报告不证明公开自动链完成。本次预先固定 10～55 K、0.5 K 区间宽度目标，其余已确认求解参数不变，详见[真实 Tc 验收](../0015-epwflow-mgb2-real-validation/native-tc-validation.md)。
