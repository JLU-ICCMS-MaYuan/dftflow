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

## 补充实施：独立 dyn0 工具与初始化入口

1. tools/qf-gen-dyn0 使用独立 Cargo 工程/锁文件，迁入 qgrid、dyn0 和完整 XML 对称解析及测试；不引用内部 crate。参数 --xml、--nq1/2/3、--prefix、--out-dir，XML 必填，网格严格正整数且检查乘积溢出。
2. 主项目删除生成子命令、argv[0] 别名、生成函数及仅服务生成器的全网格写出；XML 仅保留坐标变换需要的 bg，旧 q_points.json 读取兼容保留。
3. 提取共享 PH 初始化参数、日志/dyn0 检查。qe_ph_nosplit --dyn0 使用绝对配置/工作目录与独立 ph_init.in/out、run_ph_init.sh；-r 与 -s 互斥，不带执行参数仅准备。执行前检查 outdir 内 XML、电荷密度、波函数；不运行 SCF。提交作业执行同一经过验证的初始化路径。
4. 主安装器和验证改为 51 入口，旧 dyn0 链接识别为同目录 qeflow 的安装器链接后备份移除；独立程序保留。独立工具手动构建使用。
5. 回归工作流与坐标测试，隔离复制已有 SCF 在 sugon-amd 运行初始化；记录输入/制品哈希、作业和输出。通过后升级五二进制与入口，提交。

| 需求/成功标准 | 组件/事实来源 | 任务 | 验证 |
| --- | --- | --- | --- |
| FR-007 / SC-005 | 独立工具、原生成器基线 | T008/T009 | 仓库外构建、输出一致 |
| FR-008 / SC-006 | 共享初始化、QE 日志/dyn0 | T010/T011/T013 | 模拟失败边界、实际初始化 |
| FR-009 / SC-004/006 | 安装器、CLI、坐标读取 | T009/T012/T014 | 51 入口、旧链接迁移、源码哈希 |

## 全量重编译与默认版本切换

T015：保留现有制品，另建空的 target 目录，使用本机 Linux x86_64 配置与 locked/offline Cargo 依赖重编译五份主程序及测试；独立工具用自己的 Cargo 工程和空 target。完成后使用用户级 glibc 加载器打包，校验源码/源二进制/打包二进制哈希，安装器备份旧链接并切换 51 个主入口；独立工具单独打包和安装 qf-gen-dyn0 链接，不由主安装器管理。保留旧版本与已生成脚本指向，避免改变已有计算。

## US4：统一默认 outdir

配置默认值和 PH 输入、初始化保存文件检查、自动链快照/复制、独立合并及旧 coords 拆分的缺省目录统一改为 ./tmp。保留显式配置优先级；只改未来生成默认值，不重写原有运行输入或搬迁保存数据。examples/MgB2/prepare.sh 的新配置使用 ./tmp。

| 需求/标准 | 组件与事实来源 | 任务 | 验证 |
| --- | --- | --- | --- |
| FR-010 / SC-007 | qf-config、qf-input、chain/init、merge/split；配置与生成的 QE 输入 | T016/T017 | 模拟 QE 从实际输入读取 outdir，两种模式完整链与仅初始化，旧显式目录回归 |
| FR-010 / SC-004/007 | 算例、Overview、安装制品 | T018 | 文档/脚本、构建来源与已安装入口核对 |

完成定向测试后编译主程序，使用新版本目录保留旧制品并切换已安装入口。独立 dyn0 工具没有 outdir 配置语义，本次不修改。既有数值验收不重复执行或冒充新验证。

## US5：nosplit 结构参数

在 QePhononNosplit 增加与 split 同定义的 input 参数；分派到 run_qe_phonon_nosplit 后传入 create_phonon_context，复用显式优先与自动查找逻辑。

FR-011/012 → CLI 参数与上下文结构选择 → T019/T020 → SC-008 真实进程准备输入验证；T021 回写文档。无新实体、配置或数据迁移。

## US5 安装补充

用户后续授权更新命令：仅替换 qeflow 为本次已测试 release 制品，使用既有 glibc 加载器打包；其他四程序复用上一版本。新版本目录及旧入口备份保留，安装后验证全部入口与 nosplit 实际 -i 输入准备。
