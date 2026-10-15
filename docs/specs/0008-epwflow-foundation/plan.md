# 实施计划：配置、环境检查与受控执行基础

**Issue**：[本地 0008](issue-draft.md)；**日期**：2026-10-15；**Spec**：[spec.md](spec.md)。

## 摘要与技术上下文

Rust 2021 edition，当前工具链 rustc 1.99.0；现有 Cargo workspace，新增 epwflow 家族 crate。使用 clap、serde、serde_json、toml、anyhow、regex；哈希/进程控制按基础契约实现。科学计算调用现有 QE/EPW，绘图复用 plotters/svg2pdf。存储为受控文件与 JSON，不引入服务或数据库。目标首先 Linux x86_64 的 sugon-amd，macOS arm64 配置保持独立，未实测不写通过。规模是单用户的一次受限验收活动，可跨多个运行目录共享预算。

性能与容量遵循 Spec 的次数、并发、8 节点小时和 100 GiB；无未经测量的速度承诺。真实路径和输出仅存忽略目录。生产实现只用 Rust/Bash，测试可使用仓库既有 Python 测试习惯，但不形成 Python 运行依赖。

## 设计与数据所有权

配置使用 serde 的封闭结构与 schema_version=1；共同路径在配置文件父目录解析，结构/PSEUDO/程序/显式环境脚本均 SHA-256 绑定。科学字段由后续契约定义后纳入同一解析器，未知字段报出字段路径。check 只读；prepare 使用同目录临时目录完整写入后发布，input-init.toml 写绝对绑定路径，并保存原结构名加 -init 的副本。执行前重新哈希，软链接目标改变使绑定失效。

使用单一 epwflow crate 的 foundation 模块，不修改已有 qf-config 的宽松语义。命令用 std::process::Command 和参数数组；环境脚本仅作为用户显式绑定的可信输入，用固定 Bash 包装并通过位置参数传递命令，不能 eval 配置文本。每个外部命令设置工作目录、输入、独立 stdout/stderr、超时和进程组，取消时终止所属进程组。

状态是版本化 JSON；状态写入为同目录临时文件、fsync、rename、父目录 fsync。协调锁采用排他锁目录，保存 owner/host/pid/boot/attempt；自动恢复不能删除未知锁，进程中断遗留锁须先核实无活动所有者与提交记录，再显式恢复。预算更新先取得共享账本锁，再取得运行锁，禁止反向锁序。记录修订号，读取损坏或新版本即失败。

预算使用整数节点秒与字节：总上限 28800 节点秒、107374182400 字节；每任务请求最多 7200 秒、1 节点、64 MPI。spent + unresolved_reserved <= limit。每个 allocation 只结算一次，sacct 的 step 行不重复计费，排队时间不计占用。Slurm ID 含 cluster，记录 AllocNodes/ElapsedRaw/State/ExitCode/Start/End；查询失败或缺终态保留预留。资源内运行须关联登记的 SLURM_JOB_ID，不能把同一 allocation 多次计费。

共享账本保存两条执行通道的末尾作业；锁内给新作业追加 afterany 通道依赖，科学依赖另加 afterok。只向已经提交的先行作业加边，拒绝循环。手动参考 reserve 返回同样的通道依赖与资源约束，attach 校验调度器实际记录；未遵守的外部手动任务标记预算协议违规，不冒充并发受控。该协议仅约束登记的本次任务，不管理用户无关作业。

原生 sbatch --parsable：提交前持久化 attempt token、脚本哈希与预留，再执行一次；严格接受一个正整数 ID 和可选集群名。将 token 写入 job name/comment 用于 squeue/sacct 双源核对。非零退出若不能证明未收单也保留 unknown；成功响应落盘失败后凭 token 查找，零条或多条均不得自动重提。无无限重试。

每个作业末尾的内部 advance 只在完成数值验证后安排已经就绪的后继；执行通道与 afterok 保证先行真正退出后再启动。每次推进有界查询并核对账本，不驻留等其他作业。无可提交额度保留 blocked 原因。存储监测为作业包装内受控子线程，周期 5 秒扫描登记根目录、统计实际占用/预留/文件系统空闲；达到上限停止追加、请求停止本流程写入进程，记录峰值；突发超限不计通过，不承诺硬配额。

F1 科学阶段注册表暂为空；生产 run/submit/resume 明确失败为阶段未实现。测试用独立可执行 fixture、独立进程和真实文件跨越提交、状态、预算边界。probe 是独立有界阶段，只读取版本，不算科学成功。

## 源代码结构

本项新增模块 epwflow/src/foundation/，公开入口由 epwflow/src/main.rs 统一分派；共享状态、预算与路径只由 foundation 拥有，后续模块不得复制规则。测试位于 epwflow/tests/foundation.rs。具体 FR 到文件见下表；尚未存在的文件为明确计划触点，不作为已实现证据。

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
| FR-001 | T003/T004 | epwflow/src/foundation/cli.rs | epwflow/tests/foundation.rs |
| FR-002 | T005/T006 | epwflow/src/foundation/config.rs | epwflow/tests/foundation.rs |
| FR-003 | T007/T008 | epwflow/src/foundation/paths.rs | epwflow/tests/foundation.rs |
| FR-004 | T009/T010 | epwflow/src/foundation/snapshot.rs | epwflow/tests/foundation.rs |
| FR-005 | T011/T012 | epwflow/src/foundation/environment.rs | epwflow/tests/foundation.rs |
| FR-006 | T013/T014 | epwflow/src/foundation/state.rs | epwflow/tests/foundation.rs |
| FR-007 | T015/T016 | epwflow/src/foundation/slurm.rs | epwflow/tests/foundation.rs |
| FR-008 | T017/T018 | epwflow/src/foundation/store.rs | epwflow/tests/foundation.rs |
| FR-009 | T019/T020 | epwflow/src/foundation/budget.rs | epwflow/tests/foundation.rs |
| FR-010 | T021/T022 | epwflow/src/foundation/storage.rs | epwflow/tests/foundation.rs |
| FR-011 | T023/T024 | epwflow/src/foundation/process.rs | epwflow/tests/foundation.rs |
| FR-012 | T025/T026 | epwflow/src/foundation/status.rs | epwflow/tests/foundation.rs |
| FR-013 | T027/T028 | epwflow/src/foundation/stage.rs | epwflow/tests/foundation.rs |
| SC-001 | T029 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-002 | T030 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-003 | T031 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-004 | T032 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-005 | T033 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-006 | T034 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |
| SC-007 | T035 | acceptance.md | epwflow/tests/foundation.rs 与真实/安装证据 |

## 阶段与依赖

准备和基础协议先完成，随后按 tasks 的测试→实现→验证顺序执行。当前 Feature 完成独立验收、quickstart、Overview 回写后才推进依赖阶段；不得用尚未实现的下游编排替代本项接口验收。全部文档门通过前禁止业务代码。

## 必要复杂度

| 复杂度 | 原因 | 拒绝的简化 |
| --- | --- | --- |
| 版本化清单和依赖指纹 | 固定路径不保证固定内容，复用有真实数据风险 | 仅检查文件存在或 JOB DONE |
| 两种状态：操作与科学结果 | 提交/绘图成功不等于科学成功 | 一个完成布尔值 |
| 独立参考与固定阈值 | 相同错误流程可产生自洽假结果 | 用自己输出生成唯一期望值 |
