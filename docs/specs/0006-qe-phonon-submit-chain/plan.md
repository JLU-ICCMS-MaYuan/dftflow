# 实施计划：QE 分 q 点声子一次提交完成

**Issue**：[0006](issue-draft.md)；**Spec**：[spec.md](spec.md)；日期 2026-10-14。

## 摘要与用户批准方案

使用 Rust 工作流编排和 Bash 入口，SCF→only_init→逐 q PH→merge→q2r→matdyn 两项→collect→plot。index 使用 start_q/last_q；coords 使用 QE 笛卡尔 q（2π/a₀）。不进行逐 q SCF。

## 技术上下文

Rust 1.99、现有 Cargo workspace、serde_json、文件系统状态，Linux x86_64 Slurm/Intel MPI/QE 7.4.1；无数据库。现有 Rust/pytest 测试体系，模拟命令集成测试加真实集群验证。工作目录在 examples 的机器/模式/run 中；已有目录保留归档。

## 质量门

| 来源 | 要求 | 实施 | 状态 |
| --- | --- | --- | --- |
| 用户批准方案 | SCF 一次、两种模式、完整自动化 | FR-001～005 | 已确认 |
| AGENTS.md | 中文、真实路径本地化、立即提交 | docs/local、中文手册、最终提交 | 执行中 |
| 技能默认规则 | Issue/Spec 与确认 | 用户指定本地 0006 且完整实施授权；不操作线上 | 明确例外 |

## 源代码结构与职责

qeflow/src/phonon_chain.rs 负责阶段编排、快照、脚本及作业记录；qf-phonon 负责坐标、映射与合并；qf-runner 负责原生 Slurm 提交与查询；qf-state 支持复用 SCF 的状态。现有 CLI 名称保留，新隐藏阶段入口执行 pre/post，qe_split_submit 增加 --chain。配置路径与 workdir 规范化为绝对路径。

## 数据与依赖链

前置作业成功后才允许 q 点开始。前置阶段运行一次 SCF，检查正常终止、收敛及保存文件，复制完整 outdir（含波函数）为快照，再以 only_init=true/lqdir=true 运行 PH。以真实 dyn0 顺序建立两种坐标的索引。各 q 点从快照复制，only_init=false/recover=false，运行 ph.x。前置阶段动态提交 q 点作业及 afterok 依赖全部作业的 post；不占资源轮询。

原生 sbatch --parsable 严格解析退出码/作业号，--kill-on-invalid-dep=yes；记录写入需原子替换，提交加互斥防止并发重复。重复调用查询 squeue/sacct，活动作业不重复提交，完成结果跳过；查询不可用时报错而不是猜测。部分提交记录保留，可重新推进；旧依赖已失效的等待作业取消后可重建。

合并按实际 outdir/_ph0，预检查全部 q 点再复制；coords dyn 编号规范化，patterns/目录逐文件合并，不嵌套 cp 目录。后处理修复 prefix/workdir/config，全部输出验证后写成功标记。

## 需求到设计与验证

| 来源 | 组件 | 任务 | 验证 |
| --- | --- | --- | --- |
| FR-001、004 / SC-002 | chain、runner | T002、T004、T005 | 模拟调度及真实依赖 |
| FR-002、003 / SC-001 | chain、split、input | T003、T004 | 非正交坐标、隔离、pw 次数 |
| FR-005 / SC-003 | merge、postprocess | T003、T004、T006 | 两模式与 nosplit |
| FR-006 / SC-004 | examples、docs、安装 | T005～T007 | 手册、忽略规则、制品哈希 |

## 阶段与依赖

文档门→测试→runner/数据基础→chain→示例→模拟与回归→实际验收→安装/文档/提交。共享源码串行修改。不新增第六个二进制，不使用 Python 运行核心工作流。

## 复杂度说明

多个关联作业用于 q 点并行和自动后处理；快照复制避免并发写同一 QE 保存目录；持久记录用于失败与重复提交。单 Slurm 作业内循环不能满足用户确认的并行作业组织。
