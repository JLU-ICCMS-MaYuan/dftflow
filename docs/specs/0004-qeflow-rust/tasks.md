# 技术拆解与任务：dftflow 全量 Rust 重写

> 本任务表描述未来实施，不表示当前已完成。勾选 `[x]` 须有真实生产路径和覆盖故障边界的测试证据；各阶段的 Python 目录保留到 P5。

## P0：基线与构建可行性（依赖 Epic 0004）

- [ ] **T001** 调整 `.gitignore` 并纳入当前 11 个被忽略的 `test_auto/MgB2/` pytest 文件；区分测试脚本与生成数据。
- [ ] **T002** 冻结 Python 参考提交、依赖环境、输入 SHA-256 与实际可取得的 ISIF4 验收集；删除对不存在的 `validate_scflow_isif4.py` 的引用。
- [ ] **T003** 将 `qeflow-rust/crates/*` 抽到根 `crates/`，迁走 `qeflow-rust/Cargo.toml` 的嵌套 `[workspace]` 与公共依赖，根虚拟 workspace 暂以 `qeflow-rust/` 为成员；确认 Python `qeflow/` 仍可运行。
- [ ] **T004** 用真实矩阵路径验证 `ndarray-linalg` + 静态 OpenBLAS、spglib 的 Linux musl 静态构建和离线运行。
- [ ] **T005** 验证 PNG/JPG 及 SVG→PDF 的纯 Rust 输出，另验证 `spglib` 对称性加 Rust CIF 文本写出。
- [ ] **T006** 记录 P0 构建、离线、测试及输入哈希证据；未过门不得进入 P1。

## P1：共享能力（依赖 P0）

- [ ] **T007** 在 `qf-config` 复刻配置读取、模板、白名单与双方均为表才递归的 `None` 不覆盖合并。
- [ ] **T008** 在 `qf-io` 覆盖 QE、VASP 与 LOBSTER 实际输出，提供有效、缺失、截断及异常样例。
- [ ] **T009** 在 `qf-state`/`qf-runner` 复刻三态、q 点 task、1–11 续算和 Slurm 两段提交。
- [ ] **T010** 在 `qf-plot` 封装 band/dos/联合图/fatband/COHP/LOCPOT/周期表/wannier 图及 PNG/JPG/PDF 输出路径。
- [ ] **T011** 建立 Rust 集成测试和产物对照器：结构、状态、文件名精确，浮点采用已固定容差。

## P2：qeflow 与 scflow（依赖 P1）

- [ ] **T012** 对照 `pyproject.toml` 实现全部 17 个 `qe_*` 命令，包括 `qe_phonon_nosplit`、`qe_phonon_split_index`、`qe_phonon_split_coords`，锁定参数、退出码及旧命令入口。
- [ ] **T013** 将 `qf-gen-dyn0` 整合进 `qeflow` 并保持旧调用别名；验证 dyn0、q 点映射和脚本路径。
- [ ] **T014** 完成 phonon split/merge、状态续算、归档、表格、PNG 与联合 JPG 的实际生产链。
- [ ] **T015** 完成 `scflow` 真实 QE EPC 读取、插值、λ/ω_log/Tc、静态 OpenBLAS Eliashberg、CIF、CSV 与 PDF。
- [ ] **T016** 在冻结 Python 参考工作树运行 pytest，在 Rust 工作树运行集成测试；对照 ISIF4、精确零频、原始 γ 与 Eliashberg 下限状态。
- [ ] **T017** 记录 P2 通过证据；继续保留 Python `qeflow/`、`scflow/` 供后续参考与跨包依赖。

## P3：vaspflow（依赖 P2）

- [ ] **T018** 实现 `pyproject.toml` 中全部 19 个 `vasp_*` 命令，明确覆盖 `vasp_elf`、setup、process、charge、opt 与 converge。
- [ ] **T019** 手写并对照 COHPCAR.lobster 解析，覆盖 COHP/COBI、标签、能量、原子对及异常格式。
- [ ] **T020** 对照输入卡、运行命令、CSV/DAT、PNG 与退出行为；记录 Python pytest 与 Rust 集成测试证据。
- [ ] **T021** 保留 Python `vaspflow/`、`public/`，不破坏尚未迁移的调用方。

## P4：wannier90flow 与 substitutionflow（依赖 P2、P3）

- [ ] **T022** 实现 4 个 `wannier_*` 命令及能带对比图，校验文件发现和参数兼容。
- [ ] **T023** 实现 7 个 `substitution*` 命令，覆盖 OUTCAR 状态启发式、周期表图、续算和跨 QE/VASP 工作流。
- [ ] **T024** 仅为去除 `public` 依赖调整 Python `mattersimflow` 的等价 helper/import，保留 `mattersim_opt` 与生成的 ML Python 脚本。
- [ ] **T025** 分别验证启用与不启用 MatterSim 的 substitution 全链及 Python/Rust 对照；保留旧 Python 目录至 P5。

## P5：集中切换与交付（依赖 P0–P4）

- [ ] **T026** 搜索并清零被替换包的 Python 反向导入及 shell/Slurm 调用；确认可选 MatterSim CLI 独立于 `public`。
- [ ] **T027** 建立五个二进制与 48 个旧命令名的兼容入口，整合 `qf-gen-dyn0` 别名；在本地和计算节点验证 PATH。
- [ ] **T028** 删除 `qeflow/scflow/vaspflow/wannier90flow/substitutionflow/public` 中被替换的 Python，重命名对应 `xxx-rust/` 目录，更新 workspace path。
- [ ] **T029** 清理 `pyproject.toml` 已迁移入口和依赖，仅保留 `mattersim_opt` Python 安装内容；验证可选 ML 链路。
- [ ] **T030** 用 Linux musl 产物完成静态链接、strip、离线运行、输入/状态/数值/PNG/JPG/PDF/CIF 全部对照；Python 黄金基线和 Rust 集成测试均通过。
- [ ] **T031** 更新 README、AGENTS、Overview、子 Issue 文档与回滚说明；线上 Issue 状态由用户决定。

## 验收门

- 不以 `cargo build`、空测试骨架或部分 CLI 代替真实生产链路；P0 静态本征值求解与 PDF/CIF 可行性未证明时停止。
- 文件名、列、单位、顺序、状态、退出码精确对齐；浮点按 `contracts/config-and-output.md` 的预定容差验收，禁止失败后临时放宽。
- 常规 Rust 链路无 Python 依赖；仅启用 `mattersim_opt` 时允许目标机使用 Python。性能只记录，不设通过门槛。
