# qeflow-rust 架构大纲与阶段路线

> 本文件基于 `htc-framework-comparison.md` 的调研结论，定义 qeflow Rust 全量重写的模块边界、核心抽象、调度范式与迁移阶段。
> 硬约束：单一静态二进制（musl + strip）、仅 spawn 外部 QE/VASP、兼容现有 `input.toml` 与 `workflow_status.json`/`task_status.json`、Slurm 两段提交与续算跳过 finished。

---

## 1. 设计目标

| 目标 | 说明 |
| --- | --- |
| 单二进制 | 一个 `qeflow` 可执行文件，包含所有子命令；消除 Python 解释器与动态库导入开销 |
| 弱机器友好 | 启动即跑、低内存；无常驻 daemon、无 PostgreSQL/RabbitMQ/MongoDB |
| 外部计算隔离 | 仅通过 `std::process::Command` spawn 外部 `pw.x/ph.x/q2r.x/matdyn.x/vasp_std` |
| 契约兼容 | `input.toml`（`[qe]/[slurm]/[qe_phonon]` 等段 + 递归覆盖）、状态机三态、Slurm 两段提交语义 1:1 对齐 Python 版 |
| 可恢复 | 每次步骤完成即落盘状态；崩溃重跑仅跳过 `Finished`，与现状 `split_resume_step`(1–11 阶段) 一致 |

---

## 2. 借鉴来源（来自调研）

- **主骨架 — FireWorks**：`Firework`/`Workflow` DAG + 状态机（`READY/RUNNING/COMPLETED/FAILED`）+ `qadapter`（Slurm/PBS）+ resume 哲学。
- **补充 — jobflow**：`Job`/`Flow` 类型化 outputs + `OutputReference` 惰性数据引用；步骤间强类型产物传递、已存在则跳过（增量重算）。
- **哲学 — AiiDA**：显式节点状态 + checkpoint 恢复；但**不引入 daemon/数据库**，用文件系统 JSON 状态文件替代。
- **不借鉴**：AiiDA 的 PostgreSQL/RabbitMQ/Redis 常驻体系、pyiron 的 HDF5 单体集成环境。

---

## 3. 模块划分（cargo workspace crates）

```
qeflow-rust/                          # workspace 根
├── Cargo.toml                        # 共享依赖: clap/serde/toml/ndarray/openblas-static/plotters/rayon
├── src/bin/qeflow.rs                 # 单一二进制入口, clap 子命令映射 qe_*
└── crates/
    ├── qf-config/                    # TOML 加载 + 递归深合并(复刻 _merge_dict_in_place)
    │     pub fn load(path) -> Config
    │     pub fn merge_recursive(base: &mut Value, overrides: &Value)
    │     [qe] / [slurm] / [qe_phonon] / [qe_phonon.scf|ph|q2r|phband|phdos] 段
    ├── qf-crystal/                   # POSCAR 解析 / 化学式 / kmesh / ibrav 自动判断
    │     pub fn parse_poscar(path) -> Structure
    │     pub fn cell2ibrav(...) -> Ibrav            # 优先调用 QE cell2ibrav.x, 失败回退 Python 等价 fallback
    │     pub fn write_lattice(mode, cell) -> String # ibrav_angstrom / ibrav_celldm / cell_parameters
    ├── qf-input/                     # pw/ph/q2r/matdyn 输入 deck 写出
    │     pub fn write_pw(...)  pub fn write_ph(...)  pub fn write_q2r(...)  pub fn write_matdyn(...)
    ├── qf-io/                        # QE 输出集中读取(对齐 io_reader)
    │     pub fn read_scf_out(path) -> ScfOut
    │     pub fn read_ph_out(path) -> PhOut
    │     pub fn read_dyn0(path) -> Dyn0
    │     pub fn read_phdos(path) -> PhDos
    ├── qf-state/                     # 状态机 + 续算
    │     #[derive] pub enum TaskState { Unstarted, Finished, Error }
    │     pub struct WorkflowStatus { scf, ph, merge, q2r, matdyn_band, matdyn_dos, ... }
    │     pub fn split_resume_step(status) -> usize      # 返回下一个待执行阶段 1..=11
    │     pub fn save(path, &status) / pub fn load(path) -> WorkflowStatus
    ├── qf-runner/                    # spawn + Slurm 脚本 + 两段提交 + 并行读取
    │     pub fn resolve_qe_command(exec_path, bin) -> String
    │     pub fn spawn(cmd, stdin_file, stdout_file) -> ExitStatus
    │     pub fn write_slurm_two_stage(...)              # pre: scf->dyn0->split; post: merge->q2r->matdyn->plot
    │     pub fn parallel_read(paths, f) -> Vec<_>       # ThreadPool(rayon)
    ├── qf-phonon/                    # split/merge 工作流编排
    │     pub fn split_workflow(mode) -> Vec<Step>       # index | coords
    │     pub fn merge(...)                            # 对齐 split_merge.py 规则
    └── qf-scflow/                    # a2f / Allen-Dynes / Eliashberg / Tc / plotters
          pub fn compute_a2f(...)  pub fn allen_dynes(...)  pub fn eliashberg_solve(...)
          pub fn plot_band_dos(...)                    # plotters 替代 matplotlib
```

每个 crate 为独立 lib，`qeflow` 二进制仅做 clap 子命令 → crate 调用的薄适配层（对齐现状 CLI Adapter 哲学）。

---

## 4. 核心数据模型

```rust
// 状态机(复刻 FireWorks + 现状三态)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState { Unstarted, Finished, Error }

// 与 qf-state 的 WorkflowStatus 对应, 字段对齐 workflow_status.json
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WorkflowStatus {
    pub scf: TaskState,
    pub dyn0: TaskState,
    pub split: HashMap<String, TaskState>,   // 01/02/.. 每个 q 点任务
    pub merge: TaskState,
    pub q2r: TaskState,
    pub matdyn_band: TaskState,
    pub matdyn_dos: TaskState,
    pub plot: TaskState,
}

// 配置递归覆盖(复刻 _merge_dict_in_place: dict×dict 递归, 否则覆盖, None 跳过)
pub fn merge_recursive(base: &mut Value, overrides: &Value) { ... }

// 外部命令解析(复刻 resolve_qe_command: "mpirun -np 8 /qe/bin" + "pw.x")
pub fn resolve_qe_command(exec_path: &str, bin: &str) -> String { ... }
```

---

## 5. 任务调度与状态机

- **阶段模型**：phonon 工作流固定 11 阶段（`split_resume_step` 返回下一个未 `Finished` 阶段），对齐现状：
  1. scf → 2. dyn0 → 3. split 提交 → 4. 各 q 点 ph（并行）→ 5. merge → 6. q2r → 7. matdyn_band → 8. 归档 phband → 9. matdyn_dos → 10. 归档 phdos → 11. plot。
- **续算**：每次步骤完成即 `save(status)`；重跑时 `split_resume_step` 跳过 `Finished`，只执行 `Unstarted`/`Error`。
- **Slurm 两段提交**（1:1 对齐 `qe_phonon_split_*`）：
  - pre 段：`submit_pre_and_split_stage.sh` 在 Slurm 内 `scf → dyn0 → qe_split_submit`；
  - post 段：`submit_merge_and_process_phonon.sh` 在 Slurm 内 `merge → q2r → matdyn_phband → collect → matdyn_phdos → collect → plot`。
- **本地顺序执行**：`-r` 直接在登录节点按阶段顺序 spawn，等价于现状 `-r` 行为。

---

## 6. 配置兼容

- 读取 `input.toml`，`[qe].executable_path` 为"启动命令 + bin 目录"；`[qe].lattice_mode` 默认 `ibrav_angstrom`。
- 递归覆盖：仅当 base[key] 与 override[key] 均为表时递归合并，否则覆盖（None/缺失不覆盖），保证与 `public/config_templates.py` 行为一致。
- `[qe_phonon]` 共享配置块不被拆分成多个 TOML 节；`work_dir` 规则（显式/默认目录、当前目录 basename 判定）1:1 对齐现状。

---

## 7. 构建与打包

- 开发构建（Darwin）：`cargo build --release`。
- 发布目标（Linux 静态二进制）：`cargo build --release --target x86_64-unknown-linux-musl` + `strip`；`ndarray-linalg` 链接 `openblas-static` 静态库，避免运行时动态库依赖。
- 交付物：单个 `qeflow` 可执行文件；可用 `upx` 进一步压缩（可选）。

---

## 8. 阶段路线 / 迁移顺序

| 里程碑 | 内容 | 验收 |
| --- | --- | --- |
| M0 骨架 | qeflow-rust 分支 + cargo workspace + 各 crate 空壳 + clap 入口 | `cargo build` 通过, `qeflow --help` 列出子命令 |
| M1 配置/晶体 | qf-config 递归合并 + qf-crystal POSCAR/ibrav | 与 Python 版同输入产出一致 lattice 段 |
| M2 IO/输入 | qf-input 写出 pw/ph/q2r/matdyn + qf-io 读取 scf.out/ph.out/dyn0 | 生成的 `.in` 与 Python 版逐字节可比 |
| M3 状态/调度 | qf-state 状态机 + qf-runner spawn + Slurm 两段提交 | **已落地**：`qf-state` 复刻 `workflow_status.json` 契约 + 文件系统状态探测 + `split_resume_step`；`qf-runner` 实现 Slurm 两段提交脚本（pre/post）、`run_split.sh`、`submit_split_stage_tasks`、`parallel_read`；二进制新增 `qeflow status` 演示续算。验收：`cargo test -p qf-state -p qf-runner` + `cargo build`（需本地工具链） |
| M4 垂直切片 | qe_scf→qe_nscf 端到端 + scflow 单点(Eliashberg)端到端 | **进行中（代码已落地，待本地编译/数值对照）**：`qf-scflow` 已落地 α²F 读取、λ/wlog/w2（`integ` 复刻 scipy `simpson`）、McMillan/Allen-Dynes 闭式、Eliashberg 矩阵本征值法求解（`eig` 纯 Rust Jacobi，避免 OpenBLAS 重依赖）；二进制新增 `qeflow scflow -c examples/input.toml`（读取 `a2F.dat` 三列 THz 格式）→ 输出 λ/wlog/w2 与三种 Tc + `result.json`；`qe_nscf` 现已真实写出 `nscf.in`。验收：`cargo test -p qf-scflow` + `qeflow scflow -c examples/input.toml` 与 Python scflow 数值对照 |
| M5 全量迁移 | 全量 qe_* 子命令 + phonon split/merge + plotters 绘图 | 全量 CLI 对照现状行为 |

---

## 9. 风险

- **R1（最高）**：`qf-scflow` 复刻 scipy `simpson`、Allen-Dynes / Eliashberg 求解与 `plotters` 绘图，须在 M4 垂直切片优先验证并与 Python scflow 数值对照。
- **R2**：`qf-state` 的 11 阶段续算须 1:1 对齐 `split_resume_step`，否则破坏高通量可恢复语义。
- **R3**：`cell2ibrav` fallback 须与 QE 官方 `latgen.f90` 行为一致，建议加对照测试。
- **R4**：静态 musl + openblas 链接在交叉编译环境可能需预装 `musl-tools` 与静态 OpenBLAS。
