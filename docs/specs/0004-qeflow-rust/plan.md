# 实施计划：dftflow 全量 Rust 重写（HOW）

> 由 `big-project-spec-runner` 维护。WHAT/WHY 见 `spec.md`，执行顺序见 `tasks.md`。此文档先行；用户在另一台机器明确启动后才实施代码迁移。

## 方法与依赖

P0–P4 让现有 Python 包与 `xxx-rust/` 并行存在。P0 根 workspace 暂时引用 `qeflow-rust/`，不得直接改名为已存在的 Python `qeflow/`。Python 黄金基线在冻结参考工作树运行，Rust 集成测试在迁移工作树使用同一批输入。P5 证明反向导入、命令调用和可选 MatterSim 链路均可用后，才统一删除被替换的 Python 并改名。P4 的跨 flow 联调依赖 P2、P3；P5 依赖全部前置阶段。

## P0：基线与静态交付可行性

- 调整 `.gitignore`，把当前被忽略的 11 个 `test_auto/MgB2/` pytest 文件纳入 Git；冻结 Python 参考提交、依赖环境、样例输入与 SHA-256。`test_auto/MgB2/validate_scflow_isif4.py` 并不存在；ISIF4 须使用已确认的外部验收集或补齐可跟踪的真实数据对照脚本。
- 把 `qeflow-rust/crates/` 的共享与领域 crate 抽到仓库根 `crates/`；将 `qeflow-rust/Cargo.toml` 中现有嵌套 `[workspace]` 与公共依赖声明迁至仓库根，根虚拟 workspace 先收录 `crates/*` 和 `qeflow-rust/`，随后再收录其他 `xxx-rust/`。
- 用真实矩阵路径验证 `ndarray-linalg` + 静态 OpenBLAS、`spglib` 的 Linux musl 静态构建和离线运行；验证 `plotters` PNG/JPG、SVG 后端加纯 Rust `svg2pdf` 生成 PDF，以及 Rust CIF 序列化。
- 退出门：参考 pytest 可运行、输入哈希可复核、根 `cargo build` 与完整 musl 静态样例均通过。原生依赖未过门时停止，不静默换数值求解器。此阶段不删、不改名 Python 包。

## P1：共享基础 crate

- `qf-config` 复刻递归合并、模板、白名单；`qf-io` 读取 QE/VASP/LOBSTER 原生输出；`qf-state` 与 `qf-runner` 复刻三态、q 点 task、Slurm 两段提交和续算。
- `qf-plot` 统一各类图的轴、数据、字体和输出格式。建立 Rust 集成测试与对照器：文件结构及状态精确比较，浮点按 `contracts/config-and-output.md` 预定容差比较。
- 退出门：共享 crate 的生产路径和对应对照测试通过；不以空测试骨架充当完成证据。

## P2：qeflow 与 scflow

- 实现 `pyproject.toml` 中全部 17 个 `qe_*` 命令。现有 `qf-gen-dyn0` 功能并入 `qeflow`，生成的 shell/Slurm 脚本仍可通过兼容入口调用。
- `qf-phonon` 覆盖 split/merge、dyn0、状态续算、归档、CSV 与 PNG/JPG。独立 `scflow` 覆盖真实 QE EPC 输入、插值、λ/ω_log/Tc、`spglib` 对称性与 CIF 写出、CSV 和 PDF；Eliashberg 矩阵本征值由静态 OpenBLAS 求解。
- 退出门：冻结 Python 参考 pytest 与 Rust 集成测试分别通过，含零频、求解下限、ISIF4 真实数据及脚本行为；旧 Python `qeflow/`、`scflow/` 仍保留，因为未迁移的 `substitutionflow` 导入 `qeflow.common`。

## P3：vaspflow

- 在 `vaspflow-rust/` 实现 19 个 `vasp_*` 命令，含 `vasp_elf`、全部 setup/process/opt/charge/COHP、手写 COHPCAR.lobster 解析与图。
- 退出门：旧参数、输入卡、状态、数值和图片对照通过；保留 Python `vaspflow/` 与 `public/`，后者仍被 Wannier、substitution 和 MatterSim 引用。

## P4：wannier90flow、substitutionflow 与可选 MatterSim

- 分别在 `wannier90flow-rust/`、`substitutionflow-rust/` 实现 4 个 `wannier_*` 与 7 个 `substitution*` 命令；对照能带图、周期表图、OUTCAR 状态启发式及跨 flow 脚本。
- 只为去除 `public` 依赖等价调整 Python `mattersimflow` 的 helper/import；保留 `mattersim_opt` CLI 及其生成的 ML Python 脚本。验证启用和不启用 MatterSim 的 substitution 链路。
- 退出门：P2、P3 已通过，两条跨 flow 路径与各自 Python/Rust 对照通过；仍不提前删除原 Python 包。

## P5：集中切换与验收

- 检查所有被替换 Python 包的反向导入、脚本调用及配置引用；清零后删除旧源码，将 Rust 目录改为最终名称并更新 workspace path。保留 `test_auto/` 与 Python 参考提交作长期对照。
- 五个家族二进制提供 48 个旧命令名兼容入口；把 `qf-gen-dyn0` 作为 `qeflow` 内部命令并保留所需别名。验证本地 PATH、Slurm 计算节点 PATH、退出码与生成脚本。
- 检查五个 Linux musl 二进制已静态链接并 strip，在离线环境运行；PNG/JPG/PDF、CIF、CSV/DAT、状态文件和物理状态通过约定验收。`pyproject.toml` 只保留 `mattersim_opt` 所需 Python 安装内容。
- 更新 README、AGENTS 与已实现 Overview；保留旧 Python 参考提交和可回滚安装包。线上子 Issue 与 Epic 由用户自行核验、关闭，本计划不改变 Gitee 状态。

## 共通实现约束

- `qf-config::merge_recursive` 仅对双方均为表的值递归；`None` 不覆盖，其余整体替换。CLI 复刻旧参数、默认值、命令名与退出码；旧名字作为五个二进制的兼容入口，不靠复制 48 份二进制实现。
- 通过 `qf-runner` 执行外部 QE/VASP/LOBSTER/vaspkit。大数组采用有界内存读取；不得在数值对照前擅自降为 `f32`。普通 Rust flow 不依赖 Python，只有启用 MatterSim 时使用其原有 Python 环境。
- 性能记录同机冷启动和峰值内存，但用户选择只做功能验收，性能数据不设通过门槛。
