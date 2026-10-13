# 计划：dftflow 全量 Rust 重写（qeflow / vaspflow / wannier90flow / substitutionflow / scflow / public）

> 规划文档由 `big-project-planer` 维护；详细 Feature 设计 / 任务拆解见 `docs/specs/0004-dftflow-rust/`。
> 状态：文档修订；Rust 迁移须待用户在另一台机器明确启动。Gitee 线上 Issue 由用户维护。

## 一、目标与背景

在 `qeflow-rust` 分支上，把 dftflow 仓库内**指定** Python 实现完整重写为 Rust，功能、CLI 接口与运行逻辑与原 Python 版本保持一致；迁移完成后删除被替换的 Python 源文件。最终以该 Rust 分支作为项目主分支。

**范围排除及后续澄清**：`mattersimflow` 不迁移为 Rust，保留 Python CLI、生成的 ML 脚本与可选工作流。为最终移除共享 `public/`，允许只调整 `mattersimflow` 对 `public` 的导入和等价 helper。启用可选 MatterSim 步骤时，目标机仍需 Python。

## 二、原始需求（用户原话，逐字留档）

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

### [2026-10-10] 本轮澄清

- 用户选择「改造 mattersim」：限于去除 `public` 依赖，保留 Python CLI 与生成脚本。
- 用户选择「定量容差 (Recommended)」：文件结构精确对齐，浮点值按事先固定的容差验收。
- 用户选择「Epic加子Issue (Recommended)」：0004 为 Epic，P0–P5 待线上建立后回填真实编号。
- 用户选择「保留可选链路 (Recommended)」：仅启用 MatterSim 时允许 Python 运行时依赖。
- 用户选择「仅做功能验收」：记录性能数据，但不设性能通过门槛。
- 用户确认 `scflow` 仍需在弱机运行，继续完整 Rust 迁移；Eliashberg 矩阵求解选择「静态OpenBLAS」。

## 三、范围与能力地图

### 迁移范围（5 个 flow 与 1 个共享包）

| 包 | 家族二进制 | 领域库 crate | 当前命令数与边界 |
| --- | --- | --- | --- |
| `qeflow` | `qeflow` | `qf-phonon` | 17 个 `qe_*`；现有 Rust 切片另有 `qf-gen-dyn0` |
| `scflow` | `scflow` | `qf-scflow` | 1 个；真实 QE EPC、Tc、CIF 与 PDF |
| `vaspflow` | `vaspflow` | `df-vasp` | 19 个，含 `vasp_elf` 与 COHPCAR |
| `wannier90flow` | `wannier90` | `df-wannier` | 4 个 |
| `substitutionflow` | `substitution` | `df-substitution` | 7 个，保留可选 MatterSim 链路 |
| `public` | 无 | 共享 crate | 移除前须清空 Python 反向依赖 |

共 **48 个待迁移公开命令**，以 `pyproject.toml` 的 `[project.scripts]` 为清单；`mattersim_opt` 是保留的第 49 个 Python 命令。五个家族二进制之外，用同名兼容入口保持旧命令、参数和 Slurm 脚本调用。现有 `qf-gen-dyn0` 最终并入 `qeflow`，不作为第六个交付二进制。

### 排除范围
- `mattersimflow`：保留 Python CLI 与可选 ML 运行路径；只允许去除 `public` 依赖的最小等价调整。
- `test_auto/`：保留 pytest 对照；目前测试脚本被 `.gitignore` 忽略，P0 必须将其纳入版本管理，并冻结 Python 参考版本与输入哈希。

### 共享基础 crate（仓库根 `crates/`）
`qf-config` / `qf-crystal` / `qf-input` / `qf-io` / `qf-state` / `qf-runner` / `qf-plot` + 领域库 `qf-phonon` / `qf-scflow` / `df-vasp` / `df-wannier` / `df-substitution`，被各 flow 包以 path 依赖复用。

## 四、关键技术决策

- **并行目录、最终集中切换**：P0 根 workspace 继续引用 `qeflow-rust/`，新 flow 在 `xxx-rust/` 开发；Python 目录留到 P5。只有反向导入与旧命令调用全部切到 Rust 后，才删除被替换 Python、将 Rust 目录改为最终名称，避免 P0 的同名目录冲突。
- **共享基础 crate 抽到仓库根 `crates/`**：避免把共享逻辑全堆进单一 flow 包。
- **qf-config 递归深合并**：严格复刻 `public/config_templates.py:_merge_dict_in_place` 的「双方均 dict 才递归、None 不覆盖、其余整体替换」语义。
- **qf-io 真实解析层**：scf.out/ph.out/dyn*/dos.xml/projwfc/EIGENVAL/OUTCAR/LOCPOT/CHGCAR/COHPCAR.lobster。
- **科学库替代**：`spglib` 提供对称性，Rust 另行序列化 CIF；Eliashberg 本征值由 `ndarray-linalg` + 静态 OpenBLAS 求解；COHPCAR.lobster 用 Rust 手写解析。
- **绘图等价非像素级**：`plotters` 输出 PNG/JPG；scflow 原有 PDF 通过 SVG 绘制后用纯 Rust `svg2pdf` 转换。文件名、图层、坐标轴与内容需对照。
- **数值验收**：文件清单、列、单位、排序、状态与输入卡语义精确一致；浮点 CSV/DAT/JSON 按 `contracts/config-and-output.md` 的预定容差比较，不因失败临时放宽。
- **发布边界**：五个 `x86_64-unknown-linux-musl` 静态二进制加旧命令兼容入口；普通 Rust flow 不依赖 Python。仅 MatterSim 可选步骤需要其原有 Python 环境。性能仅记录，不设通过门槛。

## 五、阶段路线（详见 Spec 的 plan.md / tasks.md）

- **P0 基线与可行性**：收录 11 个被忽略的 pytest 文件，冻结 Python 参考环境和输入哈希；共享 crate 抽到根 `crates/`，根 workspace 暂含 `qeflow-rust/`；验证 musl 静态 OpenBLAS、spglib、PNG/JPG/PDF 与离线运行。不提前改名或删除 Python。
- **P1 共享能力**：配置、文件解析、状态机、执行器、绘图和对照测试框架；P0 通过后开始。
- **P2 QE 与 scflow**：完成 17 个 QE 命令、`qf-gen-dyn0` 整合及独立 `scflow`；Python 参考版本和 Rust 集成测试分别验证，保持旧目录并存。
- **P3 VASP**：完成 19 个 VASP 命令及 COHPCAR、图；保持 Python `vaspflow/` 与 `public/` 以供未迁移包使用。
- **P4 Wannier 与 substitution**：完成 4+7 个命令及跨 flow 工作流；仅为移除 `public` 依赖调整 MatterSim helper；最终联调依赖 P2、P3。
- **P5 集中切换**：确认 Python 反向依赖清零，安装旧命令兼容入口，验证可选 MatterSim；删除被替换 Python 并将 Rust 目录改名，`pyproject.toml` 只保留 MatterSim Python 安装内容；完成 Linux 静态交付、文档与回滚验证。

## 六、风险

1. scflow 数值复现（Python 混合 float16/float32/float64，Rust 统一 f64；高斯滤波边界对齐 SciPy `mode='reflect'`）。
2. substitutionflow status/workflow 隐式协议多（OUTCAR 启发式探测）。
3. plotters 非像素级一致（已达成一致，内容等价）。
4. 静态构建风险：当前 `qf-scflow` 切片尚未链接 OpenBLAS；`spglib`、OpenBLAS 与 PDF 转换必须先通过 Linux musl 完整构建与离线运行门。
5. 测试资产风险：现有 `test_auto/MgB2/` 的 11 个测试脚本未被 Git 跟踪，文档曾指向不存在的 `validate_scflow_isif4.py`；未冻结基线前不能据此宣称验收通过。
6. 切换风险：Python `substitutionflow` 导入 `qeflow.common`，多个包导入 `public`；P5 前保留旧目录与可回滚的并行运行环境。

## 七、Issue 管理口径

- 本仓库远端为 **Gitee**（现有 Issue `IKKGLP`，本地编号 0004）；线上创建、标签和父子关系由用户负责，本次只维护本地草稿。
- 0004 定位为 Epic，P0–P5 见 `docs/specs/0004-dftflow-rust/issue-draft.md`。子 Issue 尚无真实编号，线上双向链接尚未建立；取得编号后，P1–P4 分别建立对应编号的 Feature Spec。现有 0004 Spec 在过渡期只保存跨阶段需求与设计，不代表子 Issue 已建立。
