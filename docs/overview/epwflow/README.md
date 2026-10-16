# epwflow 家族二进制 — 已实现行为总览

> 本文档只描述已经落地的代码行为（截至 2026-10-16）。未落地的设计、计划或 Issue 不写作现状。

## 定位

`epwflow` 是工作区第六个家族二进制，用 Rust 实现从结构/赝势/配置一次性启动
`pw.x` / `ph.x` / `wannier90.x` / `pw2wannier90.x` / `epw.x` 的全自动编排层，覆盖
QE 数据准备 → Wannier 化与 EPW 插值 → 超导求解（iso/aniso）→ 结果交付。它只做编排，
不重写求解器；受预算约束、阶段可复用、可续算，并产出结构化的清单与图表。

## 模块族与 Spec 对应

| 阶段 | 模块 | Spec | 源文件数 |
| --- | --- | --- | --- |
| F1 执行基础 | `foundation` | 0008 | 13（含 cli/config/store/snapshot/state/budget/storage/process/slurm/environment/status/stage） |
| F2 QE 数据链 | `qe_data` | 0010 | 12 |
| F3 EPW 插值 | `interpolation` | 0012 | 13 |
| F4 超导求解 | `superconductivity` | 0013 | 13 |
| F5 结果交付 | `delivery` | 0014 | 10 |

## 关键契约

- 所有阶段清单均带 `schema_version = 1`，并使用 `Artifact { role, relative_path, byte_size,
  sha256, producer_stage, input_digest, format_version }` 与 `Validation { status, checks,
  units, thresholds, evidence }`。未收敛/未定位/部分成功用结构化状态表示，不填零、不连线。
- 数值标准固定：Tc 容差 0.1 K、能隙容差 0.05 meV、λ 相对容差 0.02；Tc 搜索按 0.01 K 刻度
  在 `[min_k, max_k]` 内做有界搜索；未定位返回 `None`。
- 各阶段以 F2→F3→F4→F5 顺序依赖上游清单；上游校验未通过（或缺失）则下游明确拒绝放行。
- 安装体系：6 个家族二进制（qeflow / vaspflow / scflow / wannier90 / substitution / epwflow），
  固定 `current/bin` 覆盖语义，共 52 个兼容入口。

## 已实现能力（行为测试覆盖）

- 一次入口：`epwflow check / prepare / submit / run / status / resume / probe / budget`
  以及四个阶段子命令 `qe-data` / `interpolation` / `superconductivity` / `delivery`。
- F2：一次 SCF + 完整均匀 NSCF、三种 PH 模式（index/coords/nosplit）、q 点映射、
  响应数据归集、可复用导入、同版本同输入数值对照。
- F3：Wannier 化（wannier90/pw2wannier90）、粗网格 EPW、细网格与矩阵元、α²F 与 λ、
  插值质量阈值与虚频/低频规则、分阶段复用。
- F4：iso（Allen-Dynes 线性化 Tc）与 aniso（归一化幂迭代双初值形状一致性）、单次尝试
  一模式一温度、BCS 能隙与重整化函数 Z、0.01 K 有界 Tc 搜索、收敛/未定位分离。
- F5：一次入口自动收集、α²F/能隙 SVG 绘图与 CSV、六组合（3 PH × 2 求解）验收表、
  统一预算账本摘要、与基线回归比对、可移植结果包重绘。

## 隔离验证证据

`cargo test -p epwflow` 全部通过（foundation 10 + qe_data 12 + interpolation 10 +
superconductivity 5 + delivery 4 = 41 个集成/单元测试）。所有测试使用临时目录与可执行
fixture（不启动真实 QE、不触碰真实队列、不纳入 `external/` 子模块）。

## 真实小网格验收状态（诚实标注）

- 本机 `external/q-e/build/bin` 已具备真实 `pw.x` / `ph.x` / `epw.x` / `wannier90.x` /
  `pw2wannier90.x` 可执行文件。
- 完整的六组合（3 PH × 2 求解）真实 MgB2/Si 小网格端到端验收**本次未执行**：本环境缺少
  准备好的赝势集合与计算节点输入，且工作区其它 crate 因系统依赖（spglib/llvm-config）
  无法整体构建，故未做跨 crate 全量构建。epwflow 单 crate 在本机完全构建并通过测试。
- 后续真实验收应在具备赝势与 Slurm/计算节点的环境，按 `docs/overview` 的预算上限
  （8 节点小时、100 GiB）逐项执行，并将输出写入 `docs/local/epwflow-acceptance/`（gitignored）。
