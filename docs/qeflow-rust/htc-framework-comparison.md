# 高通量计算（HTC）框架调研对比与 qeflow Rust 重构借鉴分析

> 调研目的：为 `qeflow` 的 Rust 全量重写选定可借鉴的架构范式。
> 约束前提：单一静态二进制（musl + strip）、弱机器可快速启动、仅通过 shell 子进程 spawn 外部 QE/VASP 二进制、复用现有 `input.toml` 与 `workflow_status.json`/`task_status.json` 可恢复语义。
> 对比对象：AiiDA、FireWorks、atomate2/jobflow、pyiron（均为 Python 生态的成熟 HTC 框架）。

---

## 1. 评估维度定义

| 维度 | 关注点 |
| --- | --- |
| 架构设计 | 核心抽象、模块边界、状态与数据的载体 |
| 任务调度 | 任务如何被排队/触发/恢复，与 HPC 队列（Slurm/PBS）的衔接 |
| 数据流处理 | 步骤间如何传递产物（字典、引用、provenance、统一存储） |
| 扩展性 | 新增 workflow / task / 计算后端（QE/VASP/…）的成本 |
| 性能优化 | 启动开销、存储依赖、缓存/增量重算、常驻进程 |
| 代码组织 | 目录/分层、可读性、与单一二进制目标的契合度 |

---

## 2. 候选框架逐一剖析

### 2.1 FireWorks（Materials Project, LBNL）

- **架构设计**：三层核心抽象解耦得极干净——
  - `LaunchPad`：工作流存储（默认 MongoDB，但抽象与存储后端解耦）；
  - `Firework`：一个任务，内含 `fw_spec`（参数字典）；
  - `Workflow`：由 `Firework` 组成的 DAG，通过 `links` 定义依赖；
  - `Firetask`：原子动作（Python 函数 + args）；`FWAction` 支持运行时动态增删/改向工作流。
- **任务调度**：内置有限状态机，状态含 `WAITING / READY / RUNNING / COMPLETED / FIZZLED / DEFUSED / PAUSED / ARCHIVED` 等；`qadapter`（Slurm/PBS/SGE/…）把工作流包装成集群作业，`qlaunch` 提交。**重跑/续算**由原生命令 `rlaunch rerun`、defuse/fizzle 处理驱动，哲学就是"任务可失败、可恢复、可重排"。
- **数据流处理**：`fw_spec` 字典在任务间透传，下游 `Firetask` 读取上游写入的 key。简单、灵活，但类型弱（纯字典）。
- **扩展性**：新增 task = 写一个 `Firetask` 类并注册；新增 workflow = 组合 `Firework`/`Workflow`。`qadapter` 模板化适配任意调度器。
- **性能优化**：无常驻 daemon（runner 即起即跑），MongoDB 可替换为极轻量实例；**核心抽象不强制重存储**——这是它能平移到"文件系统 JSON 状态"的关键。
- **代码组织**：`fireworks/core`（状态机/存储）、`fireworks/user_objects`（task/queue）、`fireworks/utilities`，按角色清晰拆分。
- **借鉴价值**：★★★★★ 最契合。轻量、状态机优先、队列适配器、resume 哲学，几乎是为"弱机器 + 单二进制 + spawn 子进程"量身。

### 2.2 atomate2 / jobflow（Materials Project, 新一代）

- **架构设计**：`jobflow` 以 **`Job`（装饰函数，返回带 pydantic 类型化 `outputs` 的 `JobOutput`）+ `Flow`（Job 组成的图）** 为核心；`Maker` 模式（如 `RelaxMaker`、`StaticMaker`）把可复用工作流封装成类，`make()` 返回 `Flow`。`atomate2` 在其上提供 Materials Project 全套工作流（VASP/QE/CP2K/LAMMPS）。
- **任务调度**：由图的依赖关系驱动执行；存储用 `maggma` 的 `Store` 抽象（默认 MongoDB，但**可以是任意后端，包括文件系统 JSON Store**）——数据库无关性是其最大亮点。
- **数据流处理**：**`OutputReference` 实现步骤间惰性数据引用**——上游输出作为下游输入，未求值前只是引用，执行时自动解析。配合类型化 outputs，数据传递安全且可组合。支持**增量重算/缓存**：产物已存在则跳过。
- **扩展性**：装饰器 `@job` 注册函数即 task；`Maker` 组合出复杂 workflow；计算后端通过 `Job` 内部调用的 runner 解耦。
- **性能优化**：增量重算 + 缓存显著降低重复计算；无强制 daemon。数据库无关使其可降级为文件存储。
- **代码组织**：`jobflow/{core,jobs}` 与 `atomate2/{vasp,qe,…}` 分层清晰，类型化输出贯穿全局。
- **借鉴价值**：★★★★ 类型化 `Job`/`Flow` 数据流范式与 `OutputReference` 非常值得吸收，用于 qeflow 各步骤（scf→nscf→phonon→postprocess）之间的强类型产物传递与"已存在则跳过"。

### 2.3 AiiDA（EPFL, MARVEL）

- **架构设计**：一切皆节点，构成有向无环 **provenance 图**——`Data` 节点与 `Process` 节点（Calculation / Work），通过 `CREATE / INPUT / RETURN / CALL` 等显式链接串联，形成完整可追溯谱系。
- **任务调度**：**`verdi daemon` 常驻 worker**，从 RabbitMQ 队列取任务，状态持久化在 **PostgreSQL**，Redis 做缓存。进程状态：`CREATED / RUNNING / WAITING / FINISHED / FAILED / EXCEPTED / KILLED`。**Checkpoint 哲学极强**：daemon 崩溃后可从上次检查点恢复进程。
- **数据流处理**：provenance 自动追踪，强类型 `Data` 节点；`QueryBuilder` 可做任意谱系查询。最严谨，但重量级。
- **扩展性**：通过 `entry points` 注册插件（`aiida-quantumespresso` 等），生态成熟。
- **性能优化**：**强依赖 PostgreSQL + RabbitMQ + Redis + 常驻 daemon**，启动与运维成本高，与"弱机器单二进制"目标冲突。
- **代码组织**：`aiida_core/{engine,orm,cmdcl,plugins,storage}` 模块边界清晰，但整体重。
- **借鉴价值**：★★ provenance/checkpoint 哲学（显式节点状态 + 可恢复）值得保留，但 daemon + 数据库过重，须用文件系统状态文件替代，不能直接搬。

### 2.4 pyiron（Max Planck, 计算材料）

- **架构设计**：集成式环境，`Project` 持有 Job 表，Job 对象把输入/输出统一存入 **HDF5**（`pyiron` 的 project HDF5）。Job 类（如 `VaspBase`、`QuantumespressoBase`）提供通用 `run()` / `collect()`。
- **任务调度**：内置 queuing system 接口提交集群作业；调度在 Python 层、交互性强（Jupyter 探索式工作流）。
- **数据流处理**：HDF5 作为统一对象存储，读写便捷；偏向"对象即数据"。
- **扩展性**：Job 类继承扩展；与 ASE / VASP / LAMMPS 紧耦合。
- **性能优化**：Python 层偏重，交互强但非"即启即跑"编排二进制。
- **代码组织**：整体单体较大， exploratory 导向。
- **借鉴价值**：★★ HDF5 统一存储思路可参考（我们用 `workflow_status.json` 做轻量类比），但整体偏重，不适合作为单二进制骨架。

---

## 3. 六维度横向对比

| 框架 | 架构设计 | 任务调度 | 数据流处理 | 扩展性 | 性能优化 | 代码组织 | 借鉴价值 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **FireWorks** | LaunchPad(存储)+Firework(spec)+Workflow(DAG) | 状态机 + qadapter(Slurm/PBS)，resume 哲学 | `fw_spec` 字典透传 | 高（task/workflow 易注册） | 无常驻 daemon，存储可降级 | 按角色拆分 | ★★★★★ |
| **atomate2/jobflow** | Job(类型化 outputs)+Flow(图)+Maker | 图依赖驱动，数据库无关(maggma Store) | `OutputReference` 惰性引用 + 增量重算 | 高（装饰器+组合） | 缓存/增量重算 | jobflow/atomate2 分层 | ★★★★ |
| **AiiDA** | Data/Calculation 节点 provenance 图 | daemon 常驻 + ORM 查询 | 自动 provenance + 强类型节点 | 中（插件生态） | 重（PG+MQ+Redis+daemon） | aiida_core 模块清晰 | ★★ |
| **pyiron** | Project+Job 表 + HDF5 存储 | 内部 job 表 + queuing 接口 | HDF5 统一对象存储 | 中（紧耦合 ASE/VASP） | Python 层偏重 | 单体较大 | ★★ |

---

## 4. 对 qeflow Rust 重构的借鉴映射

**推荐主骨架：FireWorks 的状态机 + 队列适配器 + Workflow DAG。**
**补充吸收：jobflow 的类型化 `Job`/`Flow` 数据流与 `OutputReference`；AiiDA 的显式节点状态 + checkpoint 恢复哲学。**
**统一落地：用文件系统 JSON 状态文件（`workflow_status.json` / `task_status.json`）替代 MongoDB/PostgreSQL/daemon，契合"弱机器 + 单静态二进制 + spawn 子进程"硬约束。**

| FireWorks / jobflow / AiiDA 概念 | qeflow Rust 映射 |
| --- | --- |
| `Firework` / `Job` | `qf-state` 中的一个**步骤（step）**，如 `scf` / `ph` / `q2r` / `matdyn_phband` |
| `Workflow` DAG / `Flow` | `qf-phonon` 的编排图（split→merge→q2r→matdyn→collect→plot） |
| 状态机 `READY/RUNNING/COMPLETED/FAILED` | `TaskState::{Unstarted, Finished, Error}`（对齐现状三态） |
| `qadapter` (Slurm) | `qf-runner` 生成 Slurm 两段提交脚本（pre: scf→dyn0→split；post: merge→q2r→matdyn→plot） |
| `fw_spec` 字典 / `OutputReference` | 步骤间**强类型产物**（Rust struct/typed value）传递，已 finished 则跳过（增量重算） |
| `LaunchPad` 存储 | `workflow_status.json` / `task_status.json` 文件系统后端 |
| AiiDA checkpoint | 每次步骤完成即落盘状态，崩溃后重跑仅跳过 `Finished` |

**明确不借鉴**：AiiDA 的 daemon + PostgreSQL + RabbitMQ + Redis 常驻体系；pyiron 的 HDF5 单体集成环境。二者与单二进制/弱机器目标相悖。

---

## 5. 推荐 Rust 架构骨架（与现状契约对齐）

```
qeflow 单二进制 (clap subcommands: qe_scf / qe_phonon* / scflow ...)
 ├─ qf-config    TOML 递归深合并（复刻 public/config_templates.py 的 _merge_dict_in_place）
 ├─ qf-crystal   POSCAR 解析 / 化学式 / kmesh / ibrav 自动判断（三模式晶格写出）
 ├─ qf-input     pw/ph/q2r/matdyn 输入 deck 写出
 ├─ qf-io        scf.out/ph.out/dyn*/dos/elph 集中读取
 ├─ qf-state     状态机 + split_resume_step(1–11 阶段) + 三态判定 + 续算跳过 finished
 ├─ qf-runner    Command spawn + Slurm 脚本生成 + 两段提交 + 并行读取(ThreadPool)
 ├─ qf-phonon    split/merge 工作流编排
 └─ qf-scflow    a2f / Allen-Dynes / Eliashberg 求解 / Tc / plotters（替代 numpy/scipy/matplotlib）
```

- **单二进制 + 文件系统状态**：彻底消除 Python 解释器/动态库导入开销。
- **static musl 二进制**：`cargo build --release --target x86_64-unknown-linux-musl` + `strip`，交付 Linux 静态二进制；Darwin 仅作开发构建。
- **复用现状契约**：`executable_path` 为"启动命令 + bin 目录"并自动拼接 `pw.x/ph.x/q2r.x/matdyn.x`；`[qe]/[slurm]/[qe_phonon]` 配置块与递归覆盖保持兼容；Slurm 两段提交逻辑 1:1 对齐现有 `qe_phonon_split_*`。

---

## 6. 风险与下一步

- **最高风险**：`qf-scflow` 的超导数值计算（复刻 scipy `simpson`、Allen-Dynes / Eliashberg 求解）与 `plotters` 绘图，须作为**垂直切片优先验证**（scflow 单点端到端跑通 + 与现有 Python scflow 结果数值对照）。
- **次高风险**：`qf-state` 的状态机与 11 阶段续算判定须 1:1 对齐现有 `split_resume_step`，否则破坏高通量可恢复语义。
- **下一步**：① 用 `big-project-planer` 产出 qeflow-rust 架构大纲与阶段路线；② 在 `qeflow-rust` 分支搭建 cargo workspace 骨架；③ 实现 config/crystal/input/io/state/runner 并跑通垂直切片；④ 用 `big-project-issue-manager` 建立 Issue 与 Spec 跟踪。

---
*本调研为 qeflow Rust 重构的决策依据；后续架构大纲与代码骨架将落地于 `qeflow-rust` 分支。*
