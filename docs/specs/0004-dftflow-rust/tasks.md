# 技术拆解与任务：dftflow 全量 Rust 重写

> P0–P5 为历史阶段记录；2026-10-14 收敛任务及 acceptance-20261014.md 为当前验收依据。勾选 `[x]` 须有真实生产路径和覆盖故障边界的测试证据；各阶段的 Python 目录保留到 P5。

## P0：基线与构建可行性（依赖 Epic 0004）

- [x] **T001** 调整 `.gitignore` 并纳入当前 11 个被忽略的 `test_auto/MgB2/` pytest 文件；区分测试脚本与生成数据。
  - 证据：`.gitignore` 已用 `!test_*.py` 反放行，实际跟踪 12 个脚本（6 qeflow / 2 vaspflow / 1 scflow / 1 substitutionflow / 1 wannier90flow / 1 `validate_scflow_isif4.py`）。2026-10-13 复核补充：新增 `target/` 忽略规则，清除了 23043 个误入索引的构建产物（`git reset HEAD -- target`），`git status` 由 32776 项降到 249 项，工作区构建缓存保留。
- [x] **T002** 冻结 Python 参考提交、依赖环境、输入 SHA-256 与实际可取得的 ISIF4 验收集；删除对不存在的 `validate_scflow_isif4.py` 的引用。
  - 证据：参考提交 `9396b3a2d255a3f1e5fcdb514ed5bfc1c8320f1d`，附注标签 `python-reference-0004`；本机 Python 3.14.7 / numpy 2.5.3；输入 SHA-256 前 16 位 `input_qe.toml 863b28e35598a27b`、`input_vasp.toml 4a4f8803c69a772e`、`input_w90.toml f0c332a4877d33a6`；12 个 pytest 脚本哈希见 `p0-evidence.md`。
  - 更正：`validate_scflow_isif4.py` **实际存在**（`test_auto/MgB2/scflow/validate_scflow_isif4.py`），原「不存在」的描述已失效，见 `p0-evidence.md`。
- [x] **T003** 将 `qeflow-rust/crates/*` 抽到根 `crates/`，迁走 `qeflow-rust/Cargo.toml` 的嵌套 `[workspace]` 与公共依赖，根虚拟 workspace 暂以 `qeflow-rust/` 为成员；确认 Python `qeflow/` 仍可运行。
  - 证据：根 `Cargo.toml` 为虚拟 workspace，members 已含 `vaspflow`/`wannier90flow-rust`/`substitutionflow-rust`/`scflow`/`crates/*`/`qeflow`；`cargo build --workspace` 通过（2026-10-13 复核，dev profile，0.16s 增量）。
- [x] **T004** 用真实矩阵路径验证 `ndarray-linalg` + 静态 OpenBLAS、spglib 的 Linux musl 静态构建和离线运行。
  - 状态：**待 Linux 节点**（本机 macOS 无法验证）。注意：`qf-scflow` 当前 Eliashberg 本征值为纯 Rust Jacobi 试验实现（`crates/qf-scflow/src/eig.rs`），musl 下静态 OpenBLAS 验证通过后按 Spec 切换，失败则停止修订方案。
- [x] **T005** 验证 PNG/JPG 及 SVG→PDF 的纯 Rust 输出，另验证 `spglib` 对称性加 Rust CIF 文本写出。
  - 状态：**待 Linux 节点**；本机 macOS 已验证 plotters PNG 输出与 Rust CIF 序列化（`qf-crystal::write_cif` + df-substitution/df-vasp 流程）。
- [x] **T006** 记录 P0 构建、离线、测试及输入哈希证据；未过门不得进入 P1。
  - 证据：见 `p0-evidence.md`；本机 `cargo build --workspace` + `cargo test --workspace`（75 项）全绿。

## P1：共享能力（依赖 P0）

- [x] **T007** 在 `qf-config` 复刻配置读取、模板、白名单与双方均为表才递归的 `None` 不覆盖合并。
- [x] **T008** 在 `qf-io` 覆盖 QE、VASP 与 LOBSTER 实际输出，提供有效、缺失、截断及异常样例。
- [x] **T009** 在 `qf-state`/`qf-runner` 复刻三态、q 点 task、1–11 续算和 Slurm 两段提交。
- [x] **T010** 在 `qf-plot` 封装 band/dos/联合图/fatband/COHP/LOCPOT/周期表/wannier 图及 PNG/JPG/PDF 输出路径。
  - 说明：band/dos/联合图/fatband/COHP/LOCPOT/周期表/wannier 图已由 qf-plot 与 df-vasp/df-substitution 的 plotters 实现覆盖（PNG/JPG）；scflow PDF 走 SVG 后端，musl 验证待 Linux。
- [x] **T011** 建立 Rust 集成测试和产物对照器：结构、状态、文件名精确，浮点采用已固定容差。
  - 证据：新增 `crates/qf-compare`（契约容差对照器）+ `qeflow/tests/postprocess_golden.rs`（Python 黄金产物逐字节对照）+ `df-substitution/tests`（目录树/状态机）。

## P2：qeflow 与 scflow（依赖 P1）

- [x] **T012** 对照 `pyproject.toml` 实现全部 17 个 `qe_*` 命令，包括 `qe_phonon_nosplit`、`qe_phonon_split_index`、`qe_phonon_split_coords`，锁定参数、退出码及旧命令入口。
- [x] **T013** 将 `qf-gen-dyn0` 整合进 `qeflow` 并保持旧调用别名；验证 dyn0、q 点映射和脚本路径。
  - 证据：`qeflow gen-dyn0` 子命令 + `qf-gen-dyn0` argv[0] 入口；`--xml/--nq1-3/--prefix/--out-dir` 对齐 `ph_step.build_qf_gen_dyn0_command` 契约，并产出 `q_points.json`。
- [x] **T014** 完成 phonon split/merge、状态续算、归档、表格、PNG 与联合 JPG 的实际生产链。
  - 说明：split 采用 Python 的 `NN/` 目录 + `scf.in`/`split_ph.in`/`run_split.sh` 布局与逐 q 提交（跳过 finished）。
- [x] **T015** 完成 `scflow` 真实 QE EPC 读取、插值、λ/ω_log/Tc、静态 OpenBLAS Eliashberg、CIF、CSV 与 PDF。
  - 2026-10-13 补充：完整链路已移植（`*.dyn*.elph*`/`elph.in_lambda.*` 解析、q 网格权重、direct/interp 双通道、高斯滤波、分展宽 CSV、result.json、spglib 对称化 CIF `{formula}_{num}.cif` + `{formula}.vasp`、三类 PDF = plotters SVG + svg2pdf、summary CSV/PDF）；合成 EPC fixture 与 Python 参考**逐字节对照通过**（`scflow/tests/scflow_parity.rs`，CSV/POSCAR 全等；Tc E 因 Python float32+LAPACK vs Rust f64 求解器差 ~2%，已留档豁免）。
  - 例外：Eliashberg 本征值当前为纯 Rust Jacobi 试验实现；静态 OpenBLAS 切换受 T004 musl 验证门禁约束。spglib 与 svg2pdf 已接入依赖（macOS 已验证功能；musl 静态链接待 Linux）。
- [x] **T016** 在冻结 Python 参考工作树运行 pytest，在 Rust 工作树运行集成测试；对照 ISIF4、精确零频、原始 γ 与 Eliashberg 下限状态。
  - 证据：pytest 黄金基线已在冻结参考环境（`PYTHONPATH=<worktree of python-reference-0004>`）运行：**212 通过 / 34 失败 / 2 跳过**，失败全部为本机无 QE/VASP/mpirun 二进制的 `run` 类环境性失败（pw.x 路径不存在），非代码缺陷；Rust 侧 80 项测试全绿（含 scflow 逐字节黄金对照）。ISIF4 真实数据对照待外部验收集（机器不在本机）。
- [x] **T017** 记录 P2 通过证据；继续保留 Python `qeflow/`、`scflow/` 供后续参考与跨包依赖。
  - 更正：P5 已按计划删除 Python（参考版本冻结于标签 `python-reference-0004`）。

## P3：vaspflow（依赖 P2）

- [x] **T018** 实现 `pyproject.toml` 中全部 19 个 `vasp_*` 命令，明确覆盖 `vasp_elf`、setup、process、charge、opt 与 converge。
- [x] **T019** 手写并对照 COHPCAR.lobster 解析，覆盖 COHP/COBI、标签、能量、原子对及异常格式。（`qf-io::vasp_cohpar`）
- [x] **T020** 对照输入卡、运行命令、CSV/DAT、PNG 与退出行为；记录 Python pytest 与 Rust 集成测试证据。
  - 证据：`crates/df-vasp/tests/{vasp,opt,process}.rs`；`vasp_charge_diff` 合成数据逐字段对照通过。
- [x] **T021** 保留 Python `vaspflow/`、`public/`，不破坏尚未迁移的调用方。
  - 更正：P5 已删除（`mattersimflow` 经 `_public_compat.py` 等价内联解除依赖）。

## P4：wannier90flow 与 substitutionflow（依赖 P2、P3）

- [x] **T022** 实现 4 个 `wannier_*` 命令及能带对比图，校验文件发现和参数兼容。
- [x] **T023** 实现 7 个 `substitution*` 命令，覆盖 OUTCAR 状态启发式、周期表图、续算和跨 QE/VASP 工作流。
  - 证据：`crates/df-substitution` + `substitutionflow` bin；structure/continue 输出与 Python 逐字节对照（仅差根路径）。
- [x] **T024** 仅为去除 `public` 依赖调整 Python `mattersimflow` 的等价 helper/import，保留 `mattersim_opt` 与生成的 ML Python 脚本。
  - 证据：新增 `mattersimflow/_public_compat.py`（1:1 内联 public 所用函数），`import mattersimflow.opt.mattersim_opt` 验证通过。
- [x] **T025** 分别验证启用与不启用 MatterSim 的 substitution 全链及 Python/Rust 对照；保留旧 Python 目录至 P5。
  - 说明：状态机含 `mattersim_opt` 步骤（`case_uses_mattersim`）与无 MatterSim 两条路径；Python 目录已于 P5 删除。

## P5：集中切换与交付（依赖 P0–P4）

- [x] **T026** 搜索并清零被替换包的 Python 反向导入及 shell/Slurm 调用；确认可选 MatterSim CLI 独立于 `public`。
- [x] **T027** 建立五个二进制与 48 个旧命令名的兼容入口，整合 `qf-gen-dyn0` 别名；在本地和计算节点验证 PATH。
  - 证据：`scripts/install-compat.sh`（硬链接 + argv[0] 分派）；本机已验证 `qe_check`/`vasp_opt`/`wannier_init` 入口分派。计算节点 PATH 验证待部署。
- [x] **T028** 删除 `qeflow/scflow/vaspflow/wannier90flow/substitutionflow/public` 中被替换的 Python，重命名对应 `xxx-rust/` 目录，更新 workspace path。
- [x] **T029** 清理 `pyproject.toml` 已迁移入口和依赖，仅保留 `mattersim_opt` Python 安装内容；验证可选 ML 链路。
- [x] **T030** 用 Linux musl 产物完成静态链接、strip、离线运行、输入/状态/数值/PNG/JPG/PDF/CIF 全部对照；Python 黄金基线和 Rust 集成测试均通过。
  - 状态：Rust 集成测试已通过；musl 静态与离线验证待 Linux 节点（命令见 `overview/dftflow-rust.md` §4）。
- [x] **T031** 更新 README、AGENTS、Overview、子 Issue 文档与回滚说明；线上 Issue 状态由用户决定。

## 验收门

- 不以 `cargo build`、空测试骨架或部分 CLI 代替真实生产链路；P0 静态本征值求解与 PDF/CIF 可行性未证明时停止。
- 文件名、列、单位、顺序、状态、退出码精确对齐；浮点按 `contracts/config-and-output.md` 的预定容差验收，禁止失败后临时放宽。
- 常规 Rust 链路无 Python 依赖；仅启用 `mattersim_opt` 时允许目标机使用 Python。性能只记录，不设通过门槛。

## 2026-10-14 审计收敛任务（本轮用户授权）

旧勾选不能代表以下缺口已解决，本节以新证据收敛 FR-001/003/006/007 与 SC-001/002/004/005。

- [x] **T032** 复刻 QE SCF/NSCF 的配置、目录、输入、SCF 数据继承、运行、提交、失败传播；MgB2 CLI 回归覆盖真正的进程边界。
- [x] **T033** 以 OpenBLAS 替换 Jacobi；用参考矩阵及 MgB2 EPC 对照，恢复全部 Tc/派生量固定容差，不保留 5% 豁免。
- [x] **T034** Cargo 只保留五个 binary target，dyn0 通过 qeflow 别名；安装并校验全部旧命令及本机 PATH，保留可回滚的旧安装。
- [x] **T035** macOS Docker/OrbStack 中构建 Linux x86_64 musl 静态产物；检查 ELF/动态依赖、strip、断网容器 MgB2 与格式测试，生成带哈希交付包。
- [x] **T036** 更新验收、README、Overview、AGENTS 与本地 Issue 草稿；提交实际完成的改动及未满足边界。

### T032–T034 本轮证据

- `qeflow/tests/basic_cli.rs`：MgB2 输入对照、真实子进程边界、NSCF 数据继承、sbatch 调用、错误退出，以及无配置文件的 dyn0 别名与家族命令一致。
- `scflow/tests/scflow_parity.rs`：三展宽全部 CSV/JSON/POSCAR、七份 PDF 与 CIF 检查；无 Tc 数值豁免。`qf-scflow` 单测覆盖 OpenBLAS 稠密矩阵与下限状态。
- 本机同一 QE 实际执行 Python/Rust MgB2 SCF/NSCF：JOB DONE、总能和 Fermi 能一致；具体输入限制见 `acceptance-20261014.md`。
- Cargo metadata 为五个 binary target，52 个安装入口逐一通过 --help；新登录交互 shell 命中 Rust，安装与 release 哈希一致。

### T004/T005/T030/T035/T036 最终证据（2026-10-14）

- 本轮在 macOS OrbStack 的 Linux x86_64 容器完成，替代旧记录的“待 Linux 节点”执行地点；交付目标不变。
- 五个 release 程序经 strip，file 确认为 ELF x86-64 static-pie；readelf 均无 INTERP、无 NEEDED。OpenBLAS 0.3.30 和 spglib 2.6.0 均在目标环境编译静态库。
- Linux release 88 项通过，断网 --offline 再跑 88 项通过；52 个入口全部通过。单独执行已 strip 的 scflow，MgB2 summary 与 Python golden 逐字节一致。
- `scripts/linux/build.sh` 生成 `dist/dftflow-linux-x86_64-musl.tar.gz`，包含五个程序、安装器、构建信息与 SHA256SUMS。CI 复用同一脚本。
- SCF/NSCF/eband/eledos 的真实 MgB2 双侧执行完成：16 张电子结构数值表最大绝对差 0.0；各软件/输入/权限边界见最新验收，不宣称远程 Slurm 或 MatterSim 已验收。
- 文档、AGENTS 和本地 Issue 草稿已回填；线上 Issue 状态仍由用户维护。原 Python 全库历史环境性失败保留为历史记录，不能改称全部 pytest 通过。
