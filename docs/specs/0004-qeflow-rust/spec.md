# Spec：dftflow 全量 Rust 重写（qeflow / vaspflow / wannier90flow / substitutionflow / scflow / public）

> 由 `big-project-spec-runner` 维护（specify 模式）。本文件写 WHAT/WHY；HOW 见 `plan.md`；任务见 `tasks.md`；技术决策见 `research.md`；验收见 `acceptance.md`；Issue 拆分见 `issue-draft.md`。

## 原始需求记录（用户原话，逐字，禁止改写/删减）

### [2026-10-10] 初始需求

> 在这个 rust分支上将项目中所有剩余的 Python 代码（包括 vaspflow、wannierflow 及其他所有模块）完整迁移并重写为 Rust 实现，确保功能、接口和逻辑与原 Python 版本保持一致。迁移完成后，删除对应的原始 Python 代码文件，保证项目中不再保留任何被替换的 Python 实现。

### [2026-10-10] 追问答复：绘图策略

> 有没有什么办法可以把 matplotlib 编译到 rust 内部。然后就可以继续使用 matplotlib 了。是在不行的话，我觉得还是选择 C 把。但是我不懂什么是 CSV+DAT, 我担心无法保持和之前一样展示图片的效果。但是我确定的是：我不想要 python 调库了，一方面是早没有网络的机器上安装麻烦，另一方面是有些机器性能太差，调库特别慢

### [2026-10-10] 追问答复：测试资产

> 保留 pytest 作黄金基线(保留 test_auto/ 下 Python 测试作为对照验收基线，同时对 Rust 侧补集成测试；最终是否删除另行决定)

### [2026-10-10] 追问答复：科学库替代

> 引入 spglib crate(scflow/utils.py 的 SpacegroupAnalyzer + CifWriter（对称化 CIF 写出）改用 Rust spglib binding), mattersimflow 保留生成 Python(mattersim_opt 本质是生成调用 ASE/mattersim 的 Python 脚本；Rust 只重写生成器，生成的 ML 脚本仍是 Python), 自实现 COHPCAR 解析(vasp_plot_cohp 目前依赖 pymatgen.io.lobster.Cohpcar，改为 Rust 手写 COHPCAR.lobster 解析器), 局部允许保留 Python(对上述无法替代的能力，允许在仓库中保留极少量 Python（作为例外登记）)

### [2026-10-10] 追问答复：工程布局

> 我想选择 A，但是我不想重命名为 dftflow-rust，还是叫 dftflow，以后我会把这个 rust 分支作为主分支的。

### [2026-10-10] 追加需求（认可拆分 + 排除 mattersim）

> 我刚刚更新了big-project 的 skill，你看看按照我的最新的 skill 的要求来执行。(1) 上面的 Issue 拆分与阶段划分是否认可，认可。 其中 mattersimflow 不需要修改为 rust，保持不动；(2) Gitee Issue 用 A 还是 B。这个暂时不需要我来管你先看看我的 skill

### [2026-10-10] 文档先行指令

> 你先把所有的文档都写好了。我在检查检查稳定，我告诉你开始执行后，你再读取所有的文档 后开始执行。

## 澄清记录

### [2026-10-10] 本轮用户选择

- 「改造 mattersim」：限于移除 `public` 依赖的等价调整，不将 `mattersim_opt` 改写为 Rust。
- 「定量容差 (Recommended)」：数值产物按预先固定的字段容差验收。
- 「Epic加子Issue (Recommended)」：0004 为 Epic，P0–P5 待用户建立线上子 Issue 后回填编号。
- 「保留可选链路 (Recommended)」：启用 MatterSim 的 substitution 工作流仍依赖 Python。
- 「仅做功能验收」：性能记录为观察数据，不设通过门槛。
- 「弱机也要运行」：`scflow` 继续完整 Rust 迁移。
- 「静态OpenBLAS」：Eliashberg 矩阵本征值求解采用静态 OpenBLAS，先验证 musl 静态交付。

上述选择取代原始记录中「`mattersimflow` 完全不改」及「Rust 重写 MatterSim 生成器」两种相冲突的旧解释；原始需求原文保留不动。

## 1. 背景与目标

现有 dftflow 以 Python 实现多个第一性原理计算编排 flow（`qeflow`/`vaspflow`/`wannier90flow`/`substitutionflow`/`scflow`/`mattersimflow`/`public`）。在性能较差、且无网络的机器上，Python 解释器启动、动态库导入与模块重载开销显著，运行偏慢且安装麻烦。

本 Spec 将**指定** Python 包完整重写为 Rust：产出多个家族静态二进制（`qeflow`/`vaspflow`/`scflow`/`wannier90`/`substitution`），仅作为**编排层**——通过 `std::process::Command` spawn 外部 QE/VASP/LOBSTER/vaspkit，不内嵌任何 Fortran 计算逻辑。功能、CLI 接口与运行逻辑与原 Python 版本保持一致；迁移完成后删除被替换的 Python 源文件。

## 2. 核心特性（FR）

- **FR-001** 全量复刻 `pyproject.toml` 当前注册的 48 个待迁移 CLI：`qe_*` 17 个、`vasp_*` 19 个、`scflow` 1 个、`wannier_*` 4 个、`substitution*` 7 个。五个家族二进制必须提供全部旧命令名兼容入口，参数、默认值、退出码及脚本调用保持一致；另保留 Python `mattersim_opt`。
- **FR-002** 配置与状态一致性：TOML 递归深合并（保留 `None 不覆盖` 语义）、参数模板与白名单、`workflow_status.json`/`task_status.json` 状态机行为不变。
- **FR-003** 产物等价：文件名、格式、列、单位、顺序、输入卡和状态精确对齐；浮点 CSV/DAT/JSON 按契约中预先固定的容差对照。scflow 的 Tc/λ/ω_log 必须覆盖真实 QE EPC 数据，Eliashberg 本征值用 `ndarray-linalg` + 静态 OpenBLAS 求解。
- **FR-004** 绘图等价：用 `plotters` 重绘 PNG/JPG，并生成现有 scflow PDF；文件名、内容、坐标轴及布局等价，允许非像素级差异。
- **FR-005** 科学库及例外：scflow 对称性用 `spglib`，另实现 CIF 序列化；COHPCAR.lobster 用 Rust 手写解析；`mattersimflow` 保留 Python CLI 与 ML 脚本，只为移除 `public` 依赖作等价调整。
- **FR-006** 离线交付：五个可直接运行的静态二进制及旧命令兼容入口；`test_auto/` pytest 与 Rust 集成测试都须保留并运行。启动时间和内存只记录，不作为通过标准。
- **FR-007** 常规 Rust flow 无 Python 运行时依赖；启用可选 `mattersim_opt` 时仍需目标机 Python。发布目标为 Linux `x86_64-unknown-linux-musl` 静态二进制加 `strip`。

## 3. 范围（In-Scope / Out-of-Scope）

### 迁移范围（6 个包）
`qeflow`、`scflow`、`vaspflow`、`wannier90flow`、`substitutionflow`、`public`。

### 排除范围
- `mattersimflow`：不迁移为 Rust；仅允许移除对 `public` 依赖的最小等价调整。`mattersim_opt` Python CLI 及生成脚本保留。
- `test_auto/` pytest 黄金基线：保留作对照；P0 须将当前被 `.gitignore` 忽略的测试脚本纳入版本管理，冻结 Python 参考环境、样例与输入哈希。
- 不修改 / 不内嵌外部 QE/VASP/LOBSTER/vaspkit 源码。
- 不引入数据库或常驻 daemon。
- 不改变现有 `input.toml` 契约语义与状态机行为（仅用 Rust 复刻）。

## 4. 成功标准（SC）

- **SC-001** 48 个旧命令名及参数可直接调用，输出结构精确、数值在预定容差内；Slurm 脚本和跨 flow 调用不因入口变更失效。
- **SC-002** 五个交付二进制经 musl 静态链接、strip 和离线运行验证；常规路径不调用 Python，可选 MatterSim 路径另行验证。
- **SC-003** 仓库根不再有任何 `xxx-rust` 目录；flow 目录命名为 `qeflow/vaspflow/scflow/wannier90flow/substitutionflow/`；`mattersimflow/` 保持 Python 不变。
- **SC-004** scflow 的 λ/ω_log/Tc 与原 Python 实现数值在预定容差内，零频和 Eliashberg 下限状态精确一致；PNG/JPG/PDF 的实际输出类型、文件名与可视内容等价。
- **SC-005** 已纳入 Git 的 Python pytest 黄金基线在冻结参考版本通过，**且** Rust 对应集成测试通过；两侧使用同一批输入和哈希记录。

## 5. 架构

```mermaid
graph TD
  subgraph 二进制层（各 flow 包 src/bin/）
    BQE[qeflow] BVA[vaspflow] BSC[scflow] BWN[wannier90] BSUB[substitution]
  end
  subgraph 共享基础 crate（crates/）
    CFG[qf-config] CRY[qf-crystal+spglib] INP[qf-input] IO[qf-io]
    ST[qf-state] RN[qf-runner] PL[qf-plot]
  end
  subgraph 领域 crate（crates/）
    PH[qf-phonon] SC[qf-scflow] VF[df-vasp] WN[df-wannier] SUB[df-substitution]
  end
  BQE-->PH; BVA-->VF; BSC-->SC; BWN-->WN; BSUB-->SUB
  VF&WN&SUB&PH&SC-->CRY&INP&IO&ST&RN&PL&CFG
  RN-->EXT[(外部 QE/VASP/LOBSTER/vaspkit)]
  ST-->FS[(workflow_status.json/task_status.json)]
```

## 6. 目录结构（迁移完成后）

```
dftflow/                              # 仓库根 = cargo 虚拟 workspace
├── Cargo.toml                        # [NEW] 根虚拟 workspace，members = crates/* + 各 flow 包
├── crates/                           # 共享基础 crate + 领域库 crate
│   ├── qf-config/ qf-crystal/ qf-input/ qf-io/ qf-state/ qf-runner/ qf-plot/
│   └── qf-phonon/ qf-scflow/ df-vasp/ df-wannier/ df-substitution/
├── qeflow/  vaspflow/  scflow/  wannier90flow/  substitutionflow/
├── mattersimflow/                    # [保留] Python CLI；仅去除 public 依赖
├── pyproject.toml                    # [保留] mattersim_opt 的最小 Python 安装内容
├── test_auto/                        # [保留] 已跟踪的 pytest 黄金基线
├── tests/                            # [NEW] Rust 集成测试
└── docs/specs/0004-qeflow-rust/      # 本 Spec
```

## 7. 关键契约（详见 contracts/config-and-output.md）

- `executable_path` 语义为「启动命令 + bin 目录」，运行时自动拼接 `pw.x/ph.x/q2r.x/matdyn.x`（复刻 `resolve_qe_command`）。
- `[qe]/[slurm]/[qe_phonon]/[scflow]/[vasp]/[wannier]/[substitution]` 配置块与递归覆盖保持兼容。
- 状态机保留三态（unstarted/finished/error）与 q 点 task 状态，续算基于 `split_resume_step` 的 1–11 阶段判定跳过 finished。
- Slurm 两段提交（pre/post）脚本生成逻辑 1:1 对齐现有 Python 行为。
- 现有 `qf-gen-dyn0` 逻辑并入 `qeflow`；交付时不增加第六个二进制。

## 8. 风险与对策

- **R-001 scflow 数值复现**：垂直切片优先验证 `simpson`、Allen-Dynes、Eliashberg 与 Tc 结果与原 Python 数值一致；高斯滤波边界对齐 SciPy `mode='reflect'`；Rust 统一 f64，对照 ISIF4 验收集校准。
- **R-002 substitutionflow 隐式协议**：OUTCAR 启发式探测状态/工作流，需逐条对照 Python 行为并加集成测试。
- **R-003 绘图替代**：`plotters` 覆盖 PNG/JPG；PDF 需经 SVG 后端与纯 Rust 转换组件验证，`spglib` 也不替代 CIF 写出。
- **R-004 目录与依赖**：P0 不能将 `qeflow-rust/` 直接改名为已存在的 `qeflow/`；`substitutionflow` 仍导入 `qeflow.common`，多个包仍导入 `public`。只在 P5 反向依赖清零后删除并改名。
- **R-005 基线与构建**：测试脚本当前被忽略，所引 `validate_scflow_isif4.py` 不存在；静态 OpenBLAS、spglib 与 PDF 转换须先通过 Linux musl 可行性验证。
