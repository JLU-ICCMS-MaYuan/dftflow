# Issue 0075：声子 q 点独立并行提交

**Issue**: https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/75

**类型**: `type:feature`

**状态**: 四阶段源码已实现；双模式原生 MgB₂ 真实 Slurm 取消恢复已验收，首次提交时序确认与既有扩展回归仍未完成

---

## 原始需求记录

用户完整需求：

> 我发现分 q 点时候，不是同时提交所有 q 点，而是等前面的 q 点计算完，才会提交新 q 点。
> 我希望使用独立的 q 点任务提交。并且针对 index 和 coords 两种分 q 点模式都支持。等完成各个 q 点计算然后计算后面的 q2r, matdyn, 绘图等任务。

确认的实施细节（2026-11-18）：

1. **q 点数量上限**：不做限制
2. **状态文件位置**：`4.qe-phonon-index/phonon-qpoints.json` 或 `4.qe-phonon-coords/phonon-qpoints.json`
3. **失败恢复粒度**：不支持手动标记已完成 q 点为"需要重算"
4. **nosplit 状态文件**：不生成，保持轻量
5. **实现范围**：nosplit 模式保持现有行为
6. **依赖关系**：q2r/matdyn/绘图作为一个后续任务提交，某个 q 点失败就暂停任务，支持续算只重算失败的 q 点
7. **配置兼容性**：新的并行提交模式直接替换现有行为

后续测试授权原文：

> ### 后续工作
>
> 1. **真实 Slurm 测试**（可选）
>    - 在有 Slurm 集群的环境验证真实提交
>    - 测试 q 点失败后的续算
>    - 验证依赖关系是否正确生效
>
> 你来完成这个#75 的这个测试！！！在 test_auto目录里面完成。提交 q 点的时候，每个 q 点用 6 个核计算

本次据此完成 T006：每 q 6 路 MPI 的原生双模式真实验收，结果见 [acceptance.md](acceptance.md)。本轮不改变既有 FR／SC 或将首次无 dyn0 的实现时序差异视为已确认。

---

## 背景与问题

### 当前行为（Issue #6 实现）

Issue #6 已实现分 q 点声子计算，但仍在**单个 Slurm 分配内串行执行**：

```
┌─────────────────────────────────────┐
│  Single Slurm Job Allocation        │
│                                      │
│  init → q1 → q2 → q3 → ... → qN    │
│         ↓                            │
│      q2r → matdyn → plot            │
└─────────────────────────────────────┘

计算时间 = T(init) + T(q1) + T(q2) + ... + T(qN) + T(collect)
```

**问题**：
- 计算时间 = 所有 q 点时间总和
- 无法利用集群并行能力
- 一个 q 点失败需要重新计算所有 q 点

### 目标行为（本 Issue）

改为**独立并行提交多个 Slurm 作业**：

```
     [init job]
         |
         | dependency: afterok
         |
   ┌─────┴─────┬─────┬─────┐
   ↓           ↓     ↓     ↓
 [q1 job]  [q2 job] [q3] [qN]  (并行独立作业)
   └─────┬─────┴─────┴─────┘
         |
         | dependency: afterok:all
         |
     [collect job]
     (q2r + matdyn + plot)

计算时间 ≈ T(init) + max(T(q1), T(q2), ..., T(qN)) + T(collect)
```

**优势**：
- 计算时间降低到最慢 q 点的时间
- 充分利用集群并行能力
- 失败恢复更精细（只重算失败的 q 点）

---

## 功能需求（FR）

### FR-001：并行提交 init 作业

**需求**：`epw_phonon -c input.toml -s` 首先提交 init 作业

**输入**：
- 配置文件 `input.toml`，`phonon.mode = "index"` 或 `"coords"`
- 结构文件（POSCAR 或配置中指定）

**输出**：
- init 脚本：`4.qe-phonon-{mode}/init/ph_init.sh`
- 提交 init 作业，获得 job_id
- 创建状态文件：`4.qe-phonon-{mode}/phonon-qpoints.json`

**约束**：
- init 作业执行 `ph.x` 的 `only_init=.true., ldisp=.true., lqdir=.true.`
- 生成 `{prefix}.dyn0`，解析出所有不可约 q 点

### FR-002：解析 q 点列表

**需求**：init 完成后解析 `dyn0` 文件，获取所有 q 点信息

**输入**：
- `{prefix}.dyn0` 文件
- 几何结构信息

**输出**：
- q 点列表，包含：
  - `global_index`：全局编号（1-based）
  - `directory`：q 点目录名（如 `q0001`）
  - `crystal`：晶体坐标 `[x, y, z]`
  - `local_index`：QE 内部编号

**约束**：
- 复用现有 `qmap::parse_dyn0` 函数
- index 和 coords 模式使用相同的 q 点列表

### FR-003：并行提交所有 q 点作业

**需求**：init 作业提交后，立即并行提交所有 q 点作业

**输入**：
- q 点列表（从状态文件读取或预估）
- init 作业的 job_id

**输出**：
- 为每个 q 点生成独立脚本：`4.qe-phonon-{mode}/q/q{:04d}/ph.sh`
- 并行提交所有 q 点作业，使用 `--dependency=afterok:{init_job_id}`
- 记录每个 q 点的 job_id 到状态文件

**约束**：
- 所有 q 点作业依赖 init 成功（`afterok`）
- 每个 q 点作业独立执行 `ph.x`
- 复用 init 的 SCF 快照

### FR-004：提交 collect 作业

**需求**：提交 collect 作业，依赖所有 q 点完成

**输入**：
- 所有 q 点的 job_id 列表

**输出**：
- collect 脚本：`4.qe-phonon-{mode}/collect/collect.sh`
- 提交 collect 作业，使用 `--dependency=afterok:{q1_job}:{q2_job}:...:{qN_job}`
- 记录 collect job_id 到状态文件

**约束**：
- collect 依赖**所有** q 点成功（`afterok`）
- collect 执行：归集 → q2r → matdyn (phband + phdos) → 绘图

### FR-005：状态文件追踪

**需求**：使用 JSON 文件追踪每个 q 点和 collect 的状态

**文件位置**：
- index 模式：`4.qe-phonon-index/phonon-qpoints.json`
- coords 模式：`4.qe-phonon-coords/phonon-qpoints.json`

**格式**：
```json
{
  "schema_version": 1,
  "mode": "index",
  "init": {
    "status": "completed",
    "job_id": 12345,
    "script": "init/ph_init.sh"
  },
  "qpoints": [
    {
      "global_index": 1,
      "directory": "q0001",
      "crystal": [0.0, 0.0, 0.0],
      "status": "completed",
      "job_id": 12346,
      "script": "q/q0001/ph.sh"
    },
    {
      "global_index": 2,
      "directory": "q0002",
      "crystal": [0.5, 0.0, 0.0],
      "status": "failed",
      "job_id": 12347,
      "script": "q/q0002/ph.sh"
    }
  ],
  "collect": {
    "status": "pending",
    "job_id": null,
    "script": "collect/collect.sh",
    "dependencies": [12346, 12347]
  }
}
```

**状态枚举**：
- `pending`：待提交
- `submitted`：已提交，等待执行
- `running`：正在执行
- `completed`：成功完成
- `failed`：失败

**约束**：
- 提交后立即写入状态
- 使用锁机制防止并发写入
- 状态文件丢失或损坏时报错，不自动重建

### FR-006：失败恢复与续算

**需求**：`epw_phonon -c input.toml -s --resume` 只重算失败的 q 点

**输入**：
- 现有状态文件 `phonon-qpoints.json`
- 配置文件

**行为**：
1. 读取状态文件
2. 检查 init 状态：
   - `failed` → 重新提交 init，退出等待
   - `completed` → 跳过
3. 识别失败的 q 点（`status == "failed"`）
4. 只重新提交失败的 q 点
5. 检查 collect 状态：
   - 所有 q 点 `completed` 且 collect `failed` → 重新提交 collect
   - 否则跳过

**约束**：
- 不支持手动标记已完成 q 点为"需要重算"
- 状态文件中的 `completed` q 点不会被重新提交

### FR-007：nosplit 模式不变

**需求**：nosplit 模式保持现有单作业串行行为

**约束**：
- nosplit 不生成状态文件
- nosplit 不改变提交逻辑
- nosplit 的代码路径完全独立

---

## 场景需求（SC）

### SC-001：正常并行提交

**前置条件**：
- 配置文件 `input.toml` 设置 `phonon.mode = "index"`
- 结构文件存在
- Slurm 集群可用

**操作**：
```bash
epw_phonon -c input.toml -s
```

**预期结果**：
1. 提交 init 作业，输出：`init job: 12345`
2. 并行提交 N 个 q 点作业，输出：`q point jobs: 12346-12355`
3. 提交 collect 作业，输出：`collect job: 12356 (depends on 12346:12347:...)`
4. 状态文件创建，所有任务标记为 `submitted`
5. `squeue` 显示所有作业

### SC-002：某个 q 点失败后续算

**前置条件**：
- 已提交所有作业
- q0002 的作业失败（手动 `scancel` 模拟）
- 其他 q 点成功完成

**操作**：
```bash
scancel 12347  # 取消 q0002
# 等待作业完成
epw_phonon -c input.toml -s --resume
```

**预期结果**：
1. 读取状态文件，识别 q0002 失败
2. 只重新提交 q0002：`resubmitting q0002: job 12400`
3. 其他 q 点跳过
4. collect 作业不重新提交（等待 q0002）

### SC-003：init 失败后续算

**前置条件**：
- init 作业失败

**操作**：
```bash
epw_phonon -c input.toml -s --resume
```

**预期结果**：
1. 检测到 init 失败
2. 重新提交 init：`resubmitting init: job 12500`
3. 不提交 q 点和 collect
4. 提示用户等待 init 完成后再次执行

### SC-004：collect 失败后续算

**前置条件**：
- 所有 q 点成功完成
- collect 作业失败

**操作**：
```bash
epw_phonon -c input.toml -s --resume
```

**预期结果**：
1. 检测到所有 q 点 `completed`，collect `failed`
2. 只重新提交 collect：`resubmitting collect: job 12600`
3. 所有 q 点跳过

### SC-005：状态查询

**操作**：
```bash
epw_phonon --status -w work_dir
```

**预期结果**：
```
Phonon stage: index mode
  init: completed (job 12345)
  q points: 10 total
    - q0001: completed (job 12346)
    - q0002: failed (job 12347)
    - q0003-q0010: completed (jobs 12348-12355)
  collect: pending (waiting for q0002)
```

---

## 数据设计

### 状态文件结构（Rust）

```rust
// epwflow/src/qe_data/phonon_state.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhononState {
    pub schema_version: u32,
    pub mode: String,  // "index" | "coords"
    pub init: InitState,
    pub qpoints: Vec<QPointState>,
    pub collect: CollectState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitState {
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<u64>,
    pub script: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QPointState {
    pub global_index: usize,
    pub directory: String,
    pub crystal: [f64; 3],
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<u64>,
    pub script: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectState {
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<u64>,
    pub script: PathBuf,
    pub dependencies: Vec<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Submitted,
    Running,
    Completed,
    Failed,
}
```

---

## 实施计划（分 4 阶段）

### 阶段 1：状态追踪基础（1-2 天）

**目标**：实现状态文件的序列化、反序列化和读写

**产出**：
- `epwflow/src/qe_data/phonon_state.rs`
- 单元测试：`epwflow/tests/phonon_state.rs`

**验收**：
```bash
cargo test --package epwflow phonon_state
```

---

### 阶段 2：并行脚本生成（2-3 天）

**目标**：生成 init/q/collect 三类脚本

**产出**：
- `epwflow/src/qe_data/phonon_parallel.rs`
- 修改 `epwflow/src/qe_data/phonon.rs`
- 修改 `epwflow/src/qe_data/scheduler.rs`

**验收**：
```bash
epw_phonon -c input.toml  # 生成脚本，不提交
ls 4.qe-phonon-index/init/ph_init.sh
ls 4.qe-phonon-index/q/q0001/ph.sh
ls 4.qe-phonon-index/collect/collect.sh
cat 4.qe-phonon-index/phonon-qpoints.json
```

---

### 阶段 3：Slurm 提交与依赖（2-3 天）

**目标**：实现 init → q → collect 的依赖提交

**产出**：
- 修改 `epwflow/src/foundation/workspace.rs`
- 实现依赖提交逻辑
- 状态写入

**验收**：
```bash
epw_phonon -c input.toml -s
squeue -u $USER  # 验证作业提交
cat work/4.qe-phonon-index/phonon-qpoints.json  # 验证状态
```

---

### 阶段 4：失败恢复与续算（1-2 天）

**目标**：实现 `--resume` 逻辑

**产出**：
- 实现续算判断逻辑
- 集成测试

**验收**：
```bash
# 手动取消某个 q 点
scancel <job_id>
epw_phonon -c input.toml -s --resume
# 验证只重新提交失败的 q 点
```

---

## 验收标准

### 单元测试

- [ ] `phonon_state.rs` 序列化/反序列化测试
- [ ] 识别失败 q 点的逻辑测试
- [ ] Slurm 依赖字符串生成测试

### 集成测试

- [ ] 生成 init/q/collect 脚本（本地，不提交）
- [x] 真实 Slurm 提交与依赖验证（2026-11-18 原生双模式，见 acceptance.md；首次公开命令返回时序的 T007 仍待确认）
- [x] 续算逻辑验证（2026-11-18 各取消一个原生 q，成功点复用、失败点重提、collect 原号更新依赖）

### 真实计算验证

- [x] 使用 MgB2 测试用例（2026-11-18 原生 PBE 小网格）
- [x] index 模式并行提交（独立 q 作业 92／93，真实重叠 5 秒）
- [x] coords 模式并行提交（独立 q 作业 98／99，真实重叠 7 秒）
- [x] 手动取消某个 q 点后续算（93→95、99→101，见 acceptance.md）

---

## Documentation Impact

- [ ] 创建 `docs/specs/0075-phonon-parallel-qpoints/spec.md`（本文档）
- [ ] 创建 `docs/specs/0075-phonon-parallel-qpoints/plan.md`
- [ ] 创建 `docs/specs/0075-phonon-parallel-qpoints/tasks.md`
- [ ] 更新 `docs/overview/epwflow/`
- [ ] 更新 `epwflow/README.md`
- [ ] Issue/Spec 双向链接验证

---

## 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| q 点数量过多（>1000） | Slurm 依赖字符串过长 | 文档说明限制 |
| 状态文件与 Slurm 不同步 | 续算判断错误 | 使用 `sacct` 校准 |
| init 成功但 dyn0 解析失败 | q 点列表不可用 | init 阶段验证 dyn0 |

---

## 关联

- **Issue #6**：分 q 点声子基础设施
- **chain.rs**：Slurm 依赖提交参考实现

## 本轮实现记录（2026-11-18）

四阶段实现、文件职责与恢复边界见 [plan.md](plan.md)，逐项验证见
[acceptance.md](acceptance.md)。上文原始需求与 FR／SC 保留，不以当前实现倒改需求。

首次没有本次原生 dyn0 时，实现采用 init 成功解析列表后、在 init 作业退出前自动提交
全部 q 及 collect；首次公开命令返回时不保证所有 q 已经入队。该时序与 FR-003／SC-001
的表述存在差异，已提出澄清，尚无用户确认，不将这些时序验收项记为全部通过。

已有工作区中与本需求无关的未提交实现保持原样，本次不修改 SCF／NSCF／Electronic／
Wannier／Superconductivity 科学执行逻辑、nosplit 执行逻辑、qeflow 复用及
`collect_ph.rs` 校验。
