# 0075 实施方案

关联：[Issue #75](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/75)、[需求](spec.md)。

## 范围与入口

只改变编号工作区 `epw_phonon -s` 的 index／coords 提交路径。在 `foundation/workspace.rs` 分流；nosplit、已有声子导入、其他科学阶段及 qeflow 复用实现保持原路径。`-r` 继续表示在调用方已有分配内同步执行。

## 四阶段对应

| 阶段 | 文件 | 职责与依据 |
| --- | --- | --- |
| 1 | `qe_data/phonon_state.rs` | FR-005：五种状态、元数据校验、锁保护的原子读改写；只筛选 failed q 点 |
| 2 | `qe_data/phonon_parallel.rs`、`phonon.rs`、`scheduler.rs` | FR-001～004：生成三类脚本；由原生 dyn0 重建 QPoint；归集只验证现有响应，不执行 PH |
| 3 | `foundation/workspace.rs`、`program.rs`、`workflow.rs` | FR-003～005、007：公开入口分流、隐藏工作进程入口；复用脚本头、环境加载、严格 sbatch 回执和 afterok 依赖模式 |
| 4 | `qe_data/phonon_parallel.rs` | FR-006、SC-002～005：Slurm 终态校准；只重提失败 q；保持已完成 q 的 job ID；更新等待中 collect 的依赖 |

## 初始化列表与提交时序

QE 的不可约点数、顺序、晶体坐标及 coords 局部编号只能由本次原生 dyn0 确定。首次只生成 init、collect 和确定为 Γ 的 q0001 驱动模板；状态中的 qpoints 保持空，不伪造未知点。

init 执行现有 `phonon::run_init`，解析并校验 dyn0/control，生成完整 q 脚本，在 init 作业退出前一次性提交所有 q 作业及 collect。所有 q 依赖 `afterok:init`，collect 依赖全部 q 的 `afterok`。提交循环只等待 sbatch 回执，不等待任何 q 计算结束。

这与 FR-002 的原生列表来源一致，但不同于 FR-003／SC-001 中“首次公开命令返回前全体作业已提交”的描述。此时序差异是实现取舍，不记为用户已确认；首次命令只返回 init 作业号，其他作业号由 init 日志、状态文件和 `--status` 查看。若必须在 init 科学执行前预提交所有作业，需要另外解决不可约列表的可靠来源。

## 状态与失败边界

- 唯一文件在 `4.qe-phonon-{index,coords}/phonon-qpoints.json`；nosplit 不创建。
- `store::Lock::acquire` 只保护短事务，竞争时有界重试；`atomic_bytes` 原子替换。科学执行不持有全工作区锁。
- `scancel` 若打断短状态事务，`--resume` 只对状态中已登记且经 Slurm 确认结束的锁所属作业调用 `store::recover_finished_lock`；活动、无作业号或未知所有者的锁不移除。
- 提交前写 `submitted` 且 `job_id=null`，获得严格回执后立即绑定 ID。未知回执拒绝自动重提。
- 所有叶子复用已有原生分配上下文、任务记录和合并 `.out` 日志；退出前恢复同名公开驱动，避免 `ph.sh`／`ph_init.sh` 被原生脚本永久覆盖。失败 init／q 使用新分配的原生尝试，collect 继续复用已完成后处理。每次新 PH 分配只清理自身工作数据和对应 dyn 文件，不动成功 q、SCF 或其他阶段。
- collect 验证每个原生响应并调用现有 `collect_ph::collect`，随后复用第 4 步后处理、绘图和完成核验。后处理继续使用既有原生任务回执。
- `sacct` 明确取消、超时等失败可变成 failed；缺失／未知记录报错。仅 Slurm COMPLETED 而没有科学完成回执，不能标为成功。
- q 重提获得新 ID 后，用 `scontrol update Dependency=afterok:...` 更新仍在等待的 collect，保留其 job ID。使用 `--kill-on-invalid-dep=no` 保留等待任务。
- 初次展开仍依赖正在运行的 init 与全部 q；续算时已完成前驱的依赖已满足，不再把可能已被 Slurm 清除的旧作业号加入新依赖。collect 只等待未完成 q，全部已完成时可清空过时依赖；原生全量归集核验保留。
- collect 自身失败仅在全部 q 完成后重提。init 失败时公开续算只先重提 init；其完成后重新绑定尚在等待旧 init 的 q 依赖。
- 批次绑定冻结配置，禁止在原批次悄悄改变科学参数或启动设置。状态丢失／损坏报错，不重建。

## 验证与限制

状态与脚本单元测试验证格式、并发回执、依赖和 shell 语法；进程测试通过公开 CLI、实际 Bash 脚本与隔离 Slurm／科学程序替身验证协议。替身测试不等于真实 Slurm 或 MgB₂ 科学验收。具体命令和未验收内容见 [验收记录](acceptance.md)。

q 点数量不设置程序上限；超长依赖仍受操作系统与 Slurm 限制，提交拒绝时保留现场和未知回执，不自动拆批或重复提交。
