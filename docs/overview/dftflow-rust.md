# dftflow Rust 当前实现（Issue 0004）

五个家族程序为 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`。对应源码位于五个 flow 目录，共享能力位于 `crates/`。迁移范围内的旧 Python 包已删除；`mattersimflow` 和 pytest 验收资产按约定保留。

## 命令与安装

- `scripts/install-compat.sh` 安装五个家族名、48 个旧名和额外 dyn0 别名，总计 52 个唯一入口；先检查源程序并备份旧入口，使用指向稳定版本目录的软链接。
- `qf-gen-dyn0` 由 `qeflow` 的 argv[0] 分派处理；Cargo 仅有五个 binary target。
- `scripts/verify-entries.sh` 在隔离目录逐一运行全部入口的 `--help`。命令名测试不替代各功能完整计算验收。

## QE 基础计算

SCF/NSCF 分别读取 `[qe_scf]` / `[qe_nscf]` 的嵌套配置，在 `qe_scf/` / `qe_nscf/` 写输入、POSCAR 与 run_qe.sh；SCF 使用 automatic k 网格，NSCF 使用完整 crystal 网格，支持 wan。

`-i/--input`、`-c/--config`、`-r/--run`、`-s/--submit` 与原版对齐；默认配置 input.toml。`-r` 启动外部 QE 并传播失败，`-s` 调用 sbatch；NSCF 复制 SCF 的 XML、电荷密度和 paw.txt，不复制波函数。电子能带与 DOS 路径共用实际执行工具，能带缺少 KPATH.in 时调用 vaspkit。

## scflow

读取 QE EPC、插值、direct/interp 两通道、λ/ω_log/Tc、CSV/JSON、spglib CIF/POSCAR 和七份合成样例 PDF。Eliashberg 通过 ndarray-linalg 0.17 调用 OpenBLAS；没有 Jacobi 回退。固定容差验收包含 Tc 与所有派生量、零频原始 gamma 与下限状态，不保留 5% 豁免。

macOS 通过 Homebrew 系统静态 OpenBLAS 接入；Linux 原生依赖在目标构建环境独立编译。系统原生库是编译链接依赖，QE/VASP 等仍是运行时外部程序。

## 已验证事实与边界

- macOS Rust 全工作区 88 项测试、聚焦 Python MgB2 9 项测试、52 个安装入口通过。
- MgB2 真实 SCF/NSCF 双侧运行完成，SCF 总能与两步 Fermi 能逐输出数字一致，能带/DOS 共 16 张数值表解析值完全相同；使用同一套 QE 自带 MgB2 UPF 赝势。
- scflow 合成 MgB2 三展宽 CSV、summary、POSCAR、全部 JSON 数值和状态通过严格黄金对照；这不是实际材料 EPC 物理验收。
- Linux x86_64 musl 五个静态 ELF 已在当前 Mac 的容器中构建并通过 88 项 release 测试、88 项断网重跑及 52 个入口检查；重建流程为 `scripts/linux/build.sh`；真实 Slurm 和可选 MatterSim 环境另行核验。

## 追踪

- [Spec](../specs/0004-dftflow-rust/spec.md)
- [2026-10-14 收敛验收](../specs/0004-dftflow-rust/acceptance-20261014.md)
- [安装与运行](../specs/0004-dftflow-rust/quickstart.md)
- [Gitee IKKGLP](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)：线上状态由用户管理。

- 已安装基础 CLI pytest：SCF、NSCF、eband、eledos 四项通过；Rust 额外覆盖 VASPKIT 标题、能带三步与 DOS 两步子进程执行及 SCF 数据继承。
