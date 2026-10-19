# 实施计划：QE 三模式数据准备

**Issue**：[本地 0010](issue-draft.md)；**日期**：2026-10-15；**Spec**：[spec.md](spec.md)。

## 摘要与技术上下文

Rust 2021 edition，当前工具链 rustc 1.99.0；现有 Cargo workspace，新增 epwflow 家族 crate。使用 clap、serde、serde_json、toml、anyhow、regex；哈希/进程控制按基础契约实现。科学计算调用现有 QE/EPW，绘图复用 plotters/svg2pdf。存储为受控文件与 JSON，不引入服务或数据库。目标首先 Linux x86_64 的 sugon-amd，macOS arm64 配置保持独立，未实测不写通过。规模是单用户的一次受限验收活动，可跨多个运行目录共享预算。

性能与容量遵循 Spec 的次数、并发、8 节点小时和 100 GiB；无未经测量的速度承诺。真实路径和输出仅存忽略目录。生产实现只用 Rust/Bash，测试可使用仓库既有 Python 测试习惯，但不形成 Python 运行依赖。

## 设计与数据所有权

使用 qf-crystal 的 POSCAR 解析，QE 输入由封闭配置适配器写出，保持 17 位有效精度；必要 qf-input 修复只影响错误配置段并回归旧入口。阶段为 SCF → snapshot → (PH/init 与 NSCF)，分 q 在 init 成功后展开；qe-data 汇合依赖所有所需节点。每一 PH 和 NSCF 保存树都从不可写 SCF 快照独立复制，禁止共享可写硬链接。

SCF 由日志收敛、终态和 XML 交叉确认；保存完整 outdir 文件树，不只 prefix.save。NSCF 显式生成无重复 crystal 网格，核对实际 XML k 点、权重、nbnd、WFC；网格闭合在分数坐标模整数下验证。标记晶格和 reciprocal convention，禁止以 automatic 生成的不可约集合代替完整集合。

only_init 正常退出产生 dyn0；严格读取网格、声明数量及坐标。沿用已有 qmap 转换数学，但校验 alat/倒格矢而不沿用旧命名错误。数字目录宽度 max(2,digits(nq))，index 全局 i；coords 局部 1。复制后生成任务时显式 only_init=false。

目标格式适配器读取各任务实际 outdir/_ph0，枚举根 Γ 与 prefix.q_i 布局、串行 dvscf/并行 dvscf1，验证关联 phsave patterns XML 的 q、扰动数和维度。局部 patterns 必须与该任务 dvscf 配对；复制全局控制元数据仅来自完整 init，修改局部编号时保持扰动基底。遇到需要未知基底转换、赝势附加数据或二进制布局时拒绝而非猜测。

归集保存 prefix.dyn_q<label>、prefix.dvscf_q<label>、prefix.phsave，label 使用目标 EPW 编号规则并以真实读取检查。验收按相同原子序比较 Cartesian 力常数和有符号频率；不会要求跨模式响应势字节相同。源数据不删除。

manifest 在全部校验后发布；旧目录 import 需要显式来源清单和程序身份，只有 qeflow JOB DONE 不够。独立最小 EPW 读取可使用手写底层输入，真实消费全部 q；该验证不依赖 F3 编排。

## 源代码结构

本项新增模块 epwflow/src/qe_data/，公开入口由 epwflow/src/main.rs 统一分派；共享状态、预算与路径只由 foundation 拥有，后续模块不得复制规则。测试位于 epwflow/tests/qe_data.rs。具体 FR 到文件见下表；尚未存在的文件为明确计划触点，不作为已实现证据。

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
| FR-001 | T003/T004 | epwflow/src/qe_data/cli.rs | epwflow/tests/qe_data.rs |
| FR-002 | T005/T006 | epwflow/src/qe_data/qe_config.rs | epwflow/tests/qe_data.rs |
| FR-003 | T007/T008 | epwflow/src/qe_data/scf.rs | epwflow/tests/qe_data.rs |
| FR-004 | T009/T010 | epwflow/src/qe_data/nscf.rs | epwflow/tests/qe_data.rs |
| FR-005 | T011/T012 | epwflow/src/qe_data/phonon.rs | epwflow/tests/qe_data.rs |
| FR-006 | T013/T014 | epwflow/src/qe_data/qmap.rs | epwflow/tests/qe_data.rs |
| FR-007 | T015/T016 | epwflow/src/qe_data/collect_ph.rs | epwflow/tests/qe_data.rs |
| FR-008 | T017/T018 | epwflow/src/qe_data/manifest.rs | epwflow/tests/qe_data.rs |
| FR-009 | T019/T020 | epwflow/src/qe_data/reuse.rs | epwflow/tests/qe_data.rs |
| FR-010 | T021/T022 | epwflow/src/qe_data/scheduler.rs | epwflow/tests/qe_data.rs |
| FR-011 | T023/T024 | epwflow/src/qe_data/compare.rs | epwflow/tests/qe_data.rs |
| FR-012 | T025/T026 | epwflow/src/qe_data/qe_acceptance.rs | epwflow/tests/qe_data.rs |
| SC-001 | T027 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-002 | T028 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-003 | T029 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-004 | T030 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-005 | T031 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-006 | T032 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-007 | T033 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |
| SC-008 | T034 | acceptance.md | epwflow/tests/qe_data.rs 与真实/安装证据 |

## 阶段与依赖

准备和基础协议先完成，随后按 tasks 的测试→实现→验证顺序执行。当前 Feature 完成独立验收、quickstart、Overview 回写后才推进依赖阶段；不得用尚未实现的下游编排替代本项接口验收。全部文档门通过前禁止业务代码。

## 必要复杂度

| 复杂度 | 原因 | 拒绝的简化 |
| --- | --- | --- |
| 版本化清单和依赖指纹 | 固定路径不保证固定内容，复用有真实数据风险 | 仅检查文件存在或 JOB DONE |
| 两种状态：操作与科学结果 | 提交/绘图成功不等于科学成功 | 一个完成布尔值 |
| 独立参考与固定阈值 | 相同错误流程可产生自洽假结果 | 用自己输出生成唯一期望值 |

## 2026-10-16 复用实现方案

1. epwflow 严格配置经适配器转换为共享输入模型；直接使用 qf-crystal 的结构解析、qf-input 的 PW/PH 渲染与原子质量。显式赝势绑定由共享渲染接口接收，不退回目录自动猜选。
2. 先提取无文件副作用的 PH 渲染接口，qeflow 原写文件接口和 epwflow 同时调用。完整均匀 NSCF 网格使用共享网格生成器；EPW 5.9 的 SYSTEM.nosym/noinv 设置为 false，保留晶体对称性并检查实际完整点集（0015 真实调试修正）。旧 build_nscf_pw_params 不直接作为 EPW 适配器。
3. qeflow/src/phonon_init.rs 与 phonon_chain.rs 内的日志/保存检查、完整快照、only_init、dyn0 计数与坐标转换、分 q 物化逐步提取到共享库。优先使用 0006 当前链路；qf-phonon 的旧 generate_split_inputs 仍逐 q SCF，不作为 EPW 主路径。
4. 科学阶段与调度分离：共享函数产生输入、数据和校验结果；epwflow foundation 负责预算、作业记录和执行，不直接调用不受其账本约束的旧提交入口。
5. EPW 独有的响应势/phsave 配对、原生 EPW 保存格式、Wannier 与插值质量、超导求解继续由 epwflow 负责。普通 qeflow 声子合并成功不自动成为 EPW 交付成功。

验证先覆盖两入口共用输入、统一 prefix、真实质量、完整均匀网格和三模式 PH 语法；再覆盖快照隔离、旧结果导入和真实原生 EPW 消费。仍未接通的阶段不得记为通过。

## 保存树与真实 PH 布局收敛

沿用 FR-003～008 和 D01～D05：SCF 快照保存完整 outdir，按配置 prefix 取 XML／密度和波函数；NSCF、初始化及各 PH 阶段使用独立物理副本。统一目标版本的进程日志检查，不在同一分配内再次异步提交。实际 dyn0 使用网格、数量和三列笛卡尔坐标；从同源 SCF XML 读 alat／完整倒格矢，恢复非正交晶体坐标。PH 采用目标版本原生 XML 动力学矩阵，coords 内部 q=1 与全局 q 分离。解析并核验矩阵、扰动模式、control 和响应势尺寸后归集 EPW save 布局。

验证以目标 QE 7.4.1 已有输出的只读解析和真实格式独立进程测试为依据；合成进程不证明科学收敛。F3 主链布局、原生 Tc 和讲义图继续单独验收，不加密 PH 或放宽已确认质量门。

新发布 QeManifest 为版本 2，artifact_root 与 artifacts 唯一绑定原生角色；旧版本 1 只保留历史读取。导入按角色保留目录并重新绑定根目录；前置消费同时核对结构、赝势与 PW／PH 程序身份。阶段内失败恢复与导入自动复用仍由 T042 跟进，不以复制报告当作恢复完成。
