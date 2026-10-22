# epw-state 功能清单与当前修复范围

核查／范围修订日期：2026-10-22。功能与路径来自已核查的生产源码；下列运行目录观察保留原检查时间，不作为此刻作业状态。

**当前范围以本轮收窄指令为准**：仅移除 `epw-state/control.lock`、`epw-state/execution.lock` 及阶段同类 `execution.lock`，保留五项脚本优化。此前“全部锁，包括预算账本锁”的选择已被替代；预算、注册表和其他种类锁及状态机制暂不调整。本清单是背景资料，不是全面去锁或状态重构的任务表。直接删除文件的影响与修改代码移除机制是两回事。

## 当前观察与锁故障

当前目录在检查期间持续变化，不作为静止快照使用。10:48:27 的读取结果是：

- `control.lock` 和 `control.lock/owner.json` 均不存在。
- 存在 `current-config.toml`、`layout.json`、`native-active.json`、`native-jobs/`、`native-progress/`、`requests/`、`history/`。
- `native-active.json` 的目录为 `3.qe-electronic/qe_eband`，状态为 `submitted`，作业号为 1327914。这里只读取了本地记录，未实时查询该作业是否仍活动。
- `1.qe-scf/`、`2.qe-nscf/` 存在 `completion.json`、输入／输出、带 token 的正式脚本和 `submit.sh`。

本轮未删除锁或状态、未提交任务、未修改计算输入。此前读取曾看到残留空 `control.lock`；不能据此推断它的具体产生原因或是谁清理了它。

源码能证明的锁故障机制：

1. [公开推进入口](../../../epwflow/src/foundation/native_workspace.rs)先调用 `workspace::control`，然后才检查、结算原生任务。
2. [控制锁调用方](../../../epwflow/src/foundation/workspace.rs)只在旧 `active.json` 存在时尝试依赖旧作业的恢复；它不等同于当前 `native-active.json`。
3. [锁实现](../../../epwflow/src/foundation/store.rs)先建目录，再写 `owner.json`；目录已存在便拒绝。中断可能留下目录，但此次没有复现确切中断窗口。
4. 旧恢复要求 owner 属于指定 Slurm 作业，并证明该作业终结。登录节点命令往往没有 `SLURM_JOB_ID`，空锁更缺少 owner，均不能依靠此恢复途径普遍解决。
5. 删锁只会越过锁检查。接下来仍有旧活动记录检查、原生作业结算、脚本／输入／输出身份检查、预算、阶段记录等步骤。因此“删锁仍不能提交”不能只凭一段报错归因；需取得删锁后的实际错误。本轮没有执行该操作序列。

最新要求已将全面去锁收窄为工作区控制／执行锁修复。移除对应获取、释放、恢复和残留拦截，不建立替代工作区锁；不删除作业与科学记录，不改预算账本／注册表及其锁。工作区同时写入的互斥不再由这些已移除锁保证。

## 当前原生提交和阶段推进

这些记录是现在逐次执行公开命令推进流程时使用的，不能一概称为旧缓存。

| 路径，相对工作区根目录 | 当前功能及使用者 | 直接删除的影响 | 本次处理 |
| --- | --- | --- | --- |
| `epw-state/control.lock/owner.json` | 标记谁占用控制入口，含主机、PID、token、时间、可选 Slurm 作业号；`workspace`、导入入口使用，`store` 创建目录锁 | 在存在活动写操作时删锁会解除原有互斥；单删文件也不会删除整套拦截机制 | 本次移除工作区控制／执行锁机制；不以手动清理代替代码修复 |
| `epw-state/layout.json` | 记录布局版本与声子模式；`layout`、配置加载据其存在与否选择编号目录映射 | 同一工作区可能被当作旧布局，读取错误路径；并非无关展示信息 | 本轮不调整，保留原有记录及机制 |
| `epw-state/current-config.toml` | 保存最近推进时解析后的配置，旧工作区内部 worker 也读取它；不会代替用户此次 `-c` 指定的配置成为新入口唯一输入 | 新 native 入口可再次写入；仍依赖该副本的旧 worker 会失败 | 本轮不调整，保留原有记录及机制 |
| `epw-state/requests/<阶段>.json` | 保存阶段科学参数摘要；供阶段交接、比较及电子结果接续检查读取 | 相关阶段无法核对当前请求，可能拒绝续跑；不是完整 TOML 备份 | 本轮不调整，保留原有记录及机制 |
| `epw-state/native-progress/<阶段>.json` | 保存参数摘要、generation 和推进状态；`native_workspace` 据此连接多次命令、处理失败重试 | 失去原推进代次；入口可能进入新的归档／重算分支，无法仅凭它恢复当前进度 | 本轮不调整，保留原有记录及机制 |
| `epw-state/native-active.json` | 保存当前原生任务的完整记录，含目录、命令、作业号、状态、预留及输入／脚本身份；`native_jobs` 优先结算它 | 当前作业可能失去入口跟踪，不能保证避免重复提交及结算 | 本轮不调整，保留原有记录及机制 |
| `epw-state/native-jobs/<任务键>.json` | 按任务键保存记录，供同一科学调用再次进入时复用或重试；任务键包括调用内容和推进代次 | 失去该任务的复用／失败判断，可能再次准备计算 | 本轮不调整，保留原有记录及机制 |
| `epw-state/native-jobs/history/<任务ID>.json` | 同次状态变化也写入该记录；与前两处保存相同 Task，供历史检查及失败恢复读取，并非只追加的每次状态事件日志 | 丢失部分历史查询／恢复依据；现有读取者需要调整 | 本轮不调整，保留原有记录及机制 |
| `epw-state/history/<归档名>/` | 参数变更、失败重试、比较输入变化时移入旧阶段目录、回执或后处理；可能包含原始科学数据和活动 Slurm 日志的保留链接 | 失去旧科学结果和失败现场，部分旧引用可能失效；可能实际删除大量数据 | 本轮不调整，保留原有记录及机制 |

主要读写者：[native_workspace.rs](../../../epwflow/src/foundation/native_workspace.rs)、[native_jobs.rs](../../../epwflow/src/foundation/native_jobs.rs)、[workspace.rs](../../../epwflow/src/foundation/workspace.rs)、[workspace_electronic.rs](../../../epwflow/src/foundation/workspace_electronic.rs)、[workspace_science.rs](../../../epwflow/src/foundation/workspace_science.rs)、[layout.rs](../../../epwflow/src/foundation/layout.rs)。

`native_jobs::persist` 每次会更新任务键记录、history 记录和 active 记录三处；本次不对这三份记录去重。固定脚本／日志名称只对 `verify`、`reconcile` 和输出读取做必要适配，避免合法覆盖被旧身份检查拒绝。

## 科学数据交接与后处理

这些清单帮助下一阶段找到真正的数据，也包含若干重试机制。状态 JSON 本身通常不大，部分历史目录实际存放科学文件。

| 路径，相对工作区根目录 | 当前功能及使用者 | 直接删除的影响 | 本次处理 |
| --- | --- | --- | --- |
| `epw-state/qe-data/manifest.json` | QE→EPW 交接，登记结构、SCF／完整 NSCF、声子响应等来源及身份；QE 数据、Wannier 与插值阶段读取 | 后续 EPW 无法按现有接口取得并核对上游数据，即使 `.save` 仍在也不等同于完成交接 | 本轮不调整，保留原有记录及机制 |
| `epw-state/qe-data/q_index_map.json` | 对应不可约 q 点和响应势文件，解决来源编号差异 | 无法按该映射准确选择响应势 | 本轮不调整，保留原有记录及机制 |
| `epw-state/qe-data/acceptance.json` | 条件生成的 QE 真实验收记录，由旧 scheduler 写入 | 丢失已记录的验收信息；不等同于删原始计算结果 | 本轮不调整，保留原有记录及机制 |
| `epw-state/coarse/manifest.json` | 编号布局把逻辑 `wannier/manifest.json` 放在此处；登记 Wannier／粗网格数据及来源，细网格读取 | 细网格阶段失去上游交接清单 | 本轮不调整，保留原有记录及机制 |
| `epw-state/interpolation/manifest.json` | 交付细网格 EPC、展宽／谱等数据及粗网格绑定；超导与结果收集读取 | iso／aniso 求解与结果收集失去上游交接依据 | 本轮不调整，保留原有记录及机制 |
| `epw-state/native-attempts/<用途>.json` | 原生 Wannier／粗细网格及比较调用的尝试记录，保存输入、输出、状态、退出码等；`interpolation::attempt` 使用 | 失去这层输入冻结、失败证据和重试判断 | 本轮不调整，保留原有记录及机制 |
| `epw-state/native-attempts/history/<旧尝试>/` | 保存旧尝试的回执和移走的失败科学目录，可能有原始 EPW 输出，不是全为 JSON | 失败输出、输入及其他科学文件可能永久丢失 | 本轮不调整，保留原有记录及机制 |
| `epw-state/post/coarse-path.json` | 保存粗网格后处理使用的路径摘要，判断 KPATH 改动是否只需重建后处理 | 失去路径变更判断，按当前逻辑可能重建后处理 | 本轮不调整，保留原有记录及机制 |
| `epw-state/post/coarse-sources.json` | 保存粗网格比较来源／后处理参数关系，判断来源变化和重用 | 比较后处理的复用或重建判断改变 | 本轮不调整，保留原有记录及机制 |

主要读写者：[QE 数据调度](../../../epwflow/src/qe_data/scheduler.rs)、[qeflow 数据适配](../../../epwflow/src/qe_data/qeflow.rs)、[插值调度](../../../epwflow/src/interpolation/scheduler.rs)、[科学尝试记录](../../../epwflow/src/interpolation/attempt.rs)、[后处理编排](../../../epwflow/src/foundation/workspace_science.rs)。

作为条件生成的实证，已完成的 `docs/local/0045-gap-temperatures/clean/epw-state/` 存在上述多数清单：`native-attempts/` 有 114 个文件，`native-jobs/` 有 49 个文件。计数仅对应该历史示例，不是每次计算固定要产生的数量，也不证明每一项都有必要保留。

## 旧流程、导入、迁移与通用执行后端

以下路径有生产代码支持，但不表示当前目录全部存在，也不表示当前 native 正常路径都会写入。本次仅修改工作区控制／执行锁调用；旧记录、旧入口及其其他锁不作清退。

| 路径，相对所在工作区 | 当前功能及使用者 | 直接删除的影响 | 本次处理 |
| --- | --- | --- | --- |
| `epw-state/active.json` | 旧包装 worker 的当前尝试；当前 native 入口仍检查它，以免与旧作业同时运行 | 旧作业可能失去活动判断与控制锁恢复依据 | 本轮不调整，保留原有记录及机制 |
| `epw-state/attempts/<ID>/` | 旧尝试 `attempt.json`、固定配置和运行快照，可能包含执行文件及计算内容；部分电子旧结果接续也会读取 | 旧 worker／旧电子接续或追溯失败，可能丢失原始数据 | 本轮不调整，保留原有记录及机制 |
| `epw-state/execution.lock/` | 旧工作区 worker／本地执行的互斥，避免同时改写工作区；是另一把锁 | 解除该路径原有互斥；与删除 control 不是同一个动作 | 本次移除工作区控制／执行锁机制；不以手动清理代替代码修复 |
| `epw-state/workflow.json` 及其 `.lock/` | 更早的整体 workflow 记录、状态和阶段指向；导入／恢复仍有读取 | 旧工作流状态查询、恢复、导入可能失败 | 本轮不调整，保留原有记录及机制 |
| `epw-state/generated/<阶段>.json` 及其 `.lock/` | 旧阶段生成记录，保存固定生成／提交关系 | 旧生成／提交幂等判断失去依据 | 本轮不调整，保留原有记录及机制 |
| `epw-state/generated/chain.json`、`chain-submission.json` 及各自 `.lock/` | 旧三程序自动链生成与提交回执，跟踪依赖提交；当前逐次 native 推进不依靠此链 | 旧链恢复及重复提交判断失效 | 本轮不调整，保留原有记录及机制 |
| `epw-state/stages/<阶段>.json` | 旧通用 StageState，记录阶段依赖、作业、验证器、产物与错误 | 旧阶段查询、依赖判断丢失 | 本轮不调整，保留原有记录及机制 |
| `epw-state/slurm/` | 通用 Slurm 后端或旧 worker 的提交脚本及日志 | 活动作业脚本通常已由 Slurm 接收，但工具可能无法读取旧日志和结算 | 本轮不调整，保留原有记录及机制 |
| `epw-state/logs/` | 通用本地 `process` 后端生成的 stdout／stderr；该路径相对实际调用的 `work`，可能在编号阶段内另有一份 `epw-state` | 失去对应执行报告的实际日志 | 本轮不调整，保留原有记录及机制 |
| `epw-state/migration.json` | 旧隐藏 `.epwflow`→可见状态迁移的完成报告；入口会检查是否完成 | 旧隐藏状态仍存在时，入口可能再次要求显式迁移 | 本轮不调整，保留原有记录及机制 |
| `epw-state/history/legacy-state/` | 迁移保存的原隐藏状态完整副本；按来源可能含更多旧目录 | 失去迁移前历史原件 | 本轮不调整，保留原有记录及机制 |
| `epw-state/importing.json`、`import-<token>/`、`legacy-import.json` | 导入中的事务记录／临时副本，以及导入完成报告；允许按同一来源完成发布 | 中途导入失去续接依据；临时副本不一定已有正式副本；完成报告丢失 | 本轮不调整，保留原有记录及机制 |
| `.epw-<token>.tmp`、`epw-released-<token>/` | 前者为原子写入临时文件，后者为锁释放时移走的待清理目录；不是阶段进度记录 | 原子写入未完成或锁现场需分辨；不能混同长期科学数据 | 本轮不调整，保留原有记录及机制 |

主要读写者：[workspace.rs](../../../epwflow/src/foundation/workspace.rs)、[workflow.rs](../../../epwflow/src/foundation/workflow.rs)、[chain.rs](../../../epwflow/src/foundation/chain.rs)、[state.rs](../../../epwflow/src/foundation/state.rs)、[slurm.rs](../../../epwflow/src/foundation/slurm.rs)、[process.rs](../../../epwflow/src/foundation/process.rs)、[迁移](../../../epwflow/src/foundation/run_state.rs)、[导入](../../../epwflow/src/foundation/workspace_import.rs)、[快照](../../../epwflow/src/foundation/snapshot.rs)。

## 不在工作区 epw-state 内，但容易混淆的内容

| 路径／类别 | 功能与删除影响 | 与本次范围的关系 |
| --- | --- | --- |
| 编号阶段的 `completion.json` | 阶段完成和实际科学产物的可复用证据；删掉可能触发重建 | 在 `1.qe-scf/` 等阶段内，不是 `epw-state` 中的锁 |
| `native-plan.json` | 根目录展示当前推进／等待计划，可以重新生成 | 不是当前作业的唯一控制记录 |
| `1.qe-scf/`～`8.epw-superconductivity/` | 实际输入、输出、波函数、响应势、EPW 数据、能隙和图表 | 删除 `epw-state` 不等于清理了这些科学数据；部分归档则反过来存于 state 内 |
| `8.epw-superconductivity/execution.lock/` | 单点和 Tc 求解共用的执行锁；残留也可能阻止这一步 | 属于阶段同类 execution 锁，本次移除 |
| 比较、交付及绘图目录旁的 `comparison.lock/`、`native-results.lock/`、`native-figures.lock/`、`comparison-figures.lock/` | 分别保护比较导出、结果收集和绘图发布，防止同时写同一结果 | 本轮不调整这些结果发布／绘图锁 |
| `epw-state-migration.lock/`、`epw-state-migrating/` | 根目录中的迁移锁与临时目录 | 本轮不调整迁移锁与迁移临时目录 |
| 快照目录旁的 `<目录名>.lock/` | `snapshot::prepare` 对目标目录的准备互斥；旧尝试目录中的快照也可能有 | 本轮不调整快照锁或快照数据 |
| 配置的预算账本及 `<账本文件>.lock/` | 记录累计消费、预留、作业结算；锁保护账本修改 | 暂不改动账本、对应锁、累计消费及预留 |
| `~/epwflow-state/campaigns.json` 及 `.lock/` | 默认用户级 campaign 与账本路径绑定；可由 `EPWFLOW_LOCAL_STATE` 或 `XDG_STATE_HOME` 改位置，旧默认有迁移读取 | 暂不改动注册表、对应锁或绑定记录 |
| 声子目录下 `.epw-save.lock/` | 共享 `qf-phonon` 导出 EPW save 时的互斥，不在 epw-state 内 | 本轮不调整共享 QE 归档锁 |
| QE 链目录下 `.chain-submit.lock/` | `qeflow::phonon_chain` 保护依赖链提交；可能因中断残留 | 本轮不调整 QE 提交链及其锁 |
| 当前脚本和 `slurm-*.log` | 运行说明及环境／调度日志，按本次要求可覆盖 | 五项脚本优化范围；仅作支持覆盖与直接科学输出所需的调整 |

源码：[阶段回执](../../../epwflow/src/foundation/stages.rs)、[超导单点](../../../epwflow/src/superconductivity/points.rs)、[Tc 调度](../../../epwflow/src/superconductivity/scheduler.rs)、[比较导出](../../../epwflow/src/interpolation/comparison.rs)、[结果交付](../../../epwflow/src/delivery/native.rs)、[比较交付](../../../epwflow/src/delivery/comparison.rs)、[原生绘图](../../../epwflow/src/delivery/native_plot.rs)、[比较绘图](../../../epwflow/src/delivery/comparison_plot.rs)、[预算](../../../epwflow/src/foundation/budget.rs)、[状态根目录](../../../epwflow/src/foundation/store.rs)、[共享 QE 归档](../../../crates/qf-phonon/src/epw.rs)、[QE 链提交](../../../qeflow/src/phonon_chain.rs)。

## 拟生成脚本的可读形式

以下是待实现的样式示例，不是现有目录已生成或已经执行的新脚本。用途为 NSCF，脚本名 `nscf.sh`。

```bash
#!/bin/bash
#SBATCH --job-name=epw-nscf
#SBATCH --partition=实际分区
#SBATCH --nodes=1
#SBATCH --ntasks=32
#SBATCH --time=实际分钟数
#SBATCH --output=slurm-nscf.log
#SBATCH --error=slurm-nscf.log

if ! cd -- "/实际工作目录/2.qe-nscf"; then exit 1; fi
source "/data/home/mayuan/intel-2024/oneapi/intel2024_setvars.sh" --force
mpirun -np 32 /data/home/mayuan/soft/qe-7.4.1-amd-oneAPI2024/bin/pw.x < nscf.in > nscf.out
```

示例中的 `-np` 后面需要进程数，再接科学程序；用户原话保留于 Issue／Spec 不作修改。普通参数直接显示，路径需要引用时再引用。计算标准错误是否单独写 `nscf.err` 在设计时与现有解析契约一并落实；科学标准输出确定直接写 `.out`。

工具在脚本目录调用 `sbatch` 即可，不再加 `--chdir`；手工执行则先进入脚本目录：

```bash
cd "/实际工作目录/2.qe-nscf"
sbatch nscf.sh
```

Slurm 的提交目录默认是调用进程的当前目录；脚本正文 `cd` 不会回溯改变 Slurm 打开相对日志路径的位置。相对日志、`--chdir`、`--open-mode` 的依据是 [Slurm 官方 sbatch 文档](https://slurm.schedmd.com/sbatch.html)，2026-10-22 已读取。默认追加或截断由系统 `JobFileAppend` 控制；实施若要求每次截断，需显式选择 `--open-mode=truncate`，不能只改文件名就声称实现覆盖。

## 当前实施顺序与边界

1. 工作区控制／执行锁修复优先，可单独实现并验收。验证中断后再次执行原命令无需手动删锁，不依赖脚本优化是否完成。
2. 五项脚本优化按 Spec 独立验收；仅调整固定命名、科学输入输出、目录切换、环境加载、必要引用及其必需的回执读取。
3. 预算账本／注册表及其锁保持原样；其他锁、其他状态去重、旧流程清退和历史科学数据清理全部暂缓。

本轮只修改 0046 文档，没有执行上述代码修复，也没有清理现场。
