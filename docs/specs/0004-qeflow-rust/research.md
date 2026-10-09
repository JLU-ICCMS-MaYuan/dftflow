# 调研：主流高通量计算（HTC）框架六维度对比

为 qeflow Rust 重写确定可借鉴的架构范式，对比 AiiDA / FireWorks / atomate2·jobflow / pyiron 六个维度。

| 框架 | 架构设计 | 任务调度 | 数据流处理 | 扩展性 | 性能优化 | 代码组织 | 借鉴价值 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **FireWorks** | LaunchPad(存储)+Firework(任务spec)+Workflow(DAG) | 状态机 READY/RUNNING/COMPLETED/FAILED/WAITING，队列适配器(Slurm/PBS) | fw_spec 字典在任务间传递 | 任务=函数指针+args，易注册 | 文件系统 LaunchPad 可免 MongoDB | fireworks/* 按角色拆分 | ★★★★★ 最契合：轻量、状态机、队列适配器、resume 哲学 |
| **atomate2/jobflow** | Job(装饰函数,含typed outputs)+Flow(图) | 由图依赖驱动，数据库无关 | 输出引用（OutputReference）实现步骤间数据传递 | 装饰器注册，Flow 可组合 | 增量重算/缓存 | jobflow/atomate2 分层 | ★★★★ Job/Flow 类型化数据流范式值得借鉴 |
| **AiiDA** | Data/Calculation 节点构成有向无环 provenance 图 | Daemon 常驻调度，ORM 查询 | 自动 provenance 追踪，强类型节点 | 插件(aiida-*)生态 | 需 PostgreSQL/Redis，重 | aiida_core 模块清晰 | ★★ provenance/checkpoint 哲学好，但 daemon+DB 过重，不适合单二进制 |
| **pyiron** | 集成环境，Job 表 + HDF5 对象存储 | 内部 job 表调度 | HDF5 统一存储对象 | 紧耦合 ASE/VASP | Python 层，交互强但重 | pyiron 单体较大 | ★★ HDF5 统一存储思路可参考，整体偏重不适合 |

## 推荐借鉴方向

以 **FireWorks 的状态机 + 队列适配器 + Workflow DAG** 为主骨架；吸收 **jobflow 的 Job/Flow 类型化输出数据流** 用于步骤依赖；保留 **AiiDA 的显式节点状态 + 可恢复(checkpoint)** 哲学，但用文件系统状态文件替代其 daemon/数据库，契合"弱机器 + 单静态二进制 + spawn 子进程"的硬约束。

## 关键决策

- **单二进制 + 文件系统状态**：避免 FireWorks/AiiDA 的外部存储依赖，用 `workflow_status.json` 复刻状态机，最贴合弱机器与可恢复语义。
- **static musl 二进制**：彻底消除 Python 解释器/动态库导入开销，满足"每次调用即启即跑"。
- **scflow 数学自实现**：用 `ndarray`+静态 OpenBLAS 复刻 scipy 数值方法，绘图用 `plotters`，是风险最高处，须作为垂直切片优先验证。
- **复用现状契约**：`executable_path` 为"启动命令+bin 目录"并自动拼接 `pw.x/ph.x/q2r.x/matdyn.x`；`[qe]/[slurm]/[qe_phonon]` 配置块与递归覆盖保持兼容。
