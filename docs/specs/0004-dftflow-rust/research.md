# 调研与技术决策：dftflow 全量 Rust 重写

> 本文件记录已确认选择与仍需 P0 实证的技术风险。用户原话见 `spec.md`；本文件不改写原始记录。

## 架构与工程布局

- 借鉴 FireWorks 的状态机、队列适配和 Workflow DAG，使用文件系统状态文件，不引入数据库或 daemon。
- 迁移期间根 Cargo workspace 收录 `crates/*` 与 `qeflow-rust/` 等过渡目录；Python `qeflow/` 已存在，P0 不能直接把 Rust 目录改为同名。全部五个 flow 经 P2–P4 对照后，在 P5 反向依赖清零时集中切换目录。
- 五个交付二进制为 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`。`pyproject.toml` 当前有 48 个待迁移公开命令与 1 个保留的 `mattersim_opt`；兼容入口不复制 48 份二进制。现有 `qf-gen-dyn0` 功能并入 `qeflow` 并保留所需调用别名。

## 数据、数值与科学库

- `qf-config::merge_recursive` 复刻 `public/config_templates.py:_merge_dict_in_place`：双方均为表才递归，override 为 `None` 不覆盖，其余整体替换。
- 文件名、列、单位、排序、状态、退出码及输入卡语义精确一致；浮点 CSV/DAT/JSON 使用契约中事前写定的字段容差。零频、原始 γ、Eliashberg 下限状态分别验收，不把搜索下限当成有效物理 Tc。
- 用户选择静态 OpenBLAS：`ndarray-linalg` 处理 Eliashberg 矩阵本征值。当前 `qf-scflow` 切片的 Jacobi 是试验实现，不能代替已确认的交付方案；P0 先用真实矩阵验证 OpenBLAS 的 musl 静态链接与离线运行，失败则停止并修订方案，不擅自换库。
- `spglib` 仅提供对称性识别与等价位置；Rust 仍需自行写出符合现有结果的 CIF 文本及 POSCAR。COHPCAR.lobster 由 Rust 手写解析，覆盖 COHP/COBI、标签、能量及异常输入。

## 绘图与文件形态

- Python 现有输出不只有 PNG：声子联合图为 JPG，scflow 使用 PDF。`plotters` 用于图形内容；PNG/JPG 走位图输出，PDF 使用 SVG 后端加纯 Rust `svg2pdf` 转换。P0 必须验证可读性、字体、坐标轴及 Linux 静态构建。
- 内容与布局等价而非像素级相同；文件名、扩展名、图层、曲线、标注和坐标范围不能丢失。CSV/DAT 数值按预定容差，不再写“字节级 1:1”作为所有浮点产物的验收条件。

## Python 例外与跨包依赖

- `mattersim_opt` 保留 Python CLI，继续生成调用 ASE/MatterSim 的 Python 脚本。用户允许只为去除 `public` 依赖调整其 helper/import；启用 substitution 的可选 ML 步骤时，目标机仍需原有 Python 环境。不启用该步骤的五个 Rust flow 无 Python 运行时依赖。
- 现有 Python `substitutionflow/prepare_qe_pseudo.py` 导入 `qeflow.common`，Wannier、substitution、MatterSim 等导入 `public`；因此 P2/P3 完成后仍保留旧 Python 包，P5 才能统一删改。
- `pyproject.toml` 最终保留 `mattersim_opt` 的最小 Python 安装内容，不可整段删除。外部 QE/VASP/LOBSTER/vaspkit 仍是独立运行依赖，不由 Rust 静态二进制内嵌。

## 测试、构建与 Issue

- `test_auto/MgB2/` 的 11 个测试脚本当前被 `.gitignore` 忽略，未形成 Git 可复现黄金基线；P0 必须跟踪它们、冻结 Python 参考提交、环境与输入哈希。文档原指向的 `validate_scflow_isif4.py` 不存在，ISIF4 对照须绑定可取得的真实验收资料。
- 发布目标为 `x86_64-unknown-linux-musl` 静态链接并 strip。当前 musl 骨架不能证明未来的 OpenBLAS、spglib、PDF 全部可静态交付；P0 是阻断门。Darwin 只承担开发构建。
- 用户选择性能只记录、不设通过门槛，因此重写不能仅凭本计划宣称达成量化提速。
- 0004 作为 Epic，P0–P5 是待编号子 Issue 草稿；线上编号、标签与双向链接由用户建立后回填。P1–P4 的正式 Feature Spec 使用真实 Issue number；本目录暂保存跨阶段资料。

### [2026-10-12] vaspkit KPOINTS 生成公式黑盒标定与方案 A 落地（P3 vaspflow E2E 发现）

- **问题来源**：MgB2 本地 E2E 中发现 Rust `kpoints::kmesh_to_grid` 与 vaspkit task 102 输出不一致（MgB2 `kmesh=0.04`：Rust 59 59 44 vs vaspkit 12 12 7），破坏 T020 文件级精确对齐。
- **源码获取结论**：vaspkit 为二进制分发（GitHub 无官方源码仓库、发行包不含 src、官网 404），`kmesh.F90` 无法获取，只能黑盒标定。
- **标定方法**：受控 POSCAR（立方/正交/六方多组 a、c）× 多组 R 值，共 30+ 样本；确认交互路径恒定（102 → K-Mesh Scheme "2"=Gamma → "Input Kmesh-Resolved Value (in Units of 2*PI/Angstrom)"），排除菜单分支干扰。
- **标定结论**：
  1. vaspkit 的 kmesh 语义为 **2π/Å 单位的 Kmesh-Resolved Value**（非 Å⁻¹ 的 KSPACING）；
  2. 立方/正交全部方向、六方 c 向精确符合 `N_i = round(|b_i|/(2π·R))`（a=3,K=0.04→8；正交 3/4/5→8 6 5）；
  3. 六方面内 a 向存在**与对称性相关的量化**（实测 15/12/9，均为 3 的倍数，台阶边界非线性），无法用闭式公式复刻；
  4. 原 Rust 回退公式存在 **2π 单位错误**（把 Å⁻¹ 当 2π/Å 用），已修复。
- **方案 A 落地**（用户确认）：`df_vasp::kpoints::write_kpoints_via_vaspkit` 与 Python `run_vaspkit` 完全一致地调用 vaspkit（同 cwd、同 stdin `102\n2\n{kmesh}\n`），vaspkit 不可用/失败时回退纯 Rust 公式（单位已修正，六方面内留已知偏差并文档化）。`build_vasp_inputs` 顺序改为 POSCAR → INCAR → POTCAR → KPOINTS，对齐 Python `setup()`（实测 vaspkit 不覆盖已存在的 INCAR）。
- **验证**：双场景集成通过（vaspkit 在 PATH → 12 12 7 与 Python 一致；PATH 隐藏 vaspkit → 回退 9 9 7）；`cargo test --workspace` 33 套件全绿。
- **pytest 对照终态**（Python 参考 = git 6385e45，sc-wiki env）：66 项中 60 过（49 API/CLI + 11 run/submit）；6 项未过全部为环境/版本因素——mattersim×4（本机无 `mattersim` ML 包）、`vasp_opt_rv4_api`（测试文件比 6385e45 模板新，stage4 期望 ENCUT=800）、`vasp_lobsterin_submit`（需本地 lobster 路径，本地化后通过）。
- **环境差异记录**：`[vasp_locpot]` 的显式 `NGX/Y/Z=40` 绑定集群 FFT 粗网格；本地 FFTW 选出的粗网格为 24/24/28（细网格 48/48/56），显式值必须取粗网格否则 VASP 拒读 CHGCAR（`charge density could not be read`）。这是跨机器 FFT 库差异，不是代码缺陷；本地验证用 24/24/28。
