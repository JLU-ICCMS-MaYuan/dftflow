# dftflow

DFT 计算工作流自动化（Issue 0004：全量 Rust 重写）。

五个 Rust 家族程序为 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`；`mattersim_opt` 按约定保留 Python。当前验收范围与证据见 [0004 收敛验收](docs/specs/0004-dftflow-rust/acceptance-20261014.md)。

## 分支约定

- `master`：默认主分支，承接原 `rust` 分支的 Rust 实现及后续开发。
- `python`：保留原 `master` 分支的 Python 版本，切换时的提交为 `7a172fe`。
- `python-reference-0004`：迁移数值对照使用的冻结标签（`9396b3a`），与 `python` 分支用途不同，继续保留。

## 构建

```bash
# macOS arm64 本机构建：先安装原生依赖
brew install openblas spglib

# Linux x86_64 / macOS arm64：脚本按本机平台选择独立配置
bash scripts/build-native.sh
bash scripts/build-native.sh test --release --workspace --locked

# 在 macOS Docker/OrbStack 内构建 Linux x86_64 musl 并验证
bash scripts/linux/build.sh
```

macOS Mach-O 文件不能直接在 Linux 执行；Linux 交付脚本使用 Linux 编译环境产出 ELF，并检查静态链接和断网运行。目标计算节点仍需提供 QE/VASP 等外部计算程序。

| 构建平台 | Cargo 配置 | 二进制目录 |
| --- | --- | --- |
| Linux x86_64 原生（glibc） | `.cargo/linux-x86_64.toml` | `target/linux-x86_64/x86_64-unknown-linux-gnu/release/` |
| macOS arm64 原生 | `.cargo/macos-arm64.toml` | `target/macos-arm64/aarch64-apple-darwin/release/` |
| Linux x86_64 musl 静态交付 | `.cargo/linux-host.toml` + `scripts/linux/Dockerfile` | `dist/linux-x86_64-musl/bin/` |

Linux 原生构建需要 Rust、C/C++/Fortran 编译器、libclang、静态 OpenBLAS、spglib 头文件与库，以及 fontconfig/freetype 开发包。非系统依赖可通过 `DFTFLOW_NATIVE_PREFIX` 指定前缀（含 `include/`、`lib/pkgconfig/` 或 `lib64/pkgconfig/`）；共享库运行路径通过链接期 rpath 或 `LD_LIBRARY_PATH` 提供。机器特有的环境变量放在不入 Git 的 `docs/local/build-linux-x86_64.env` 中，由脚本自动加载。原生 glibc 产物用于本机，跨节点独立交付使用 musl 包。

登录节点与计算节点的 glibc 不同时，可用 `scripts/linux/package-native-runtime.sh SOURCE_BIN_DIR DEST_BIN_DIR NATIVE_PREFIX` 打包配套私有 glibc 加载器（需要 patchelf）。该版本依赖共享目录路径，避免将私有 glibc 加入全局 LD_LIBRARY_PATH；通用独立交付仍使用 musl 包。

自行编译静态 spglib 时启用 `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`，以支持 Linux 原生 Rust 程序的 PIE 链接。

## 安装命令

```bash
bash scripts/install-compat.sh /absolute/path/to/binaries ~/.local/bin
export PATH="$HOME/.local/bin:$PATH"
bash scripts/verify-entries.sh /absolute/path/to/binaries
```

安装器提供全部 48 个旧名以及家族入口，共 51 个名称、五份二进制。独立 [dyn0 工具](tools/qf-gen-dyn0/README.md) 单独构建，不由安装器安装。入口为指向源目录的软链接，因此应将二进制放在稳定版本目录中再安装。已有同名入口会先备份；安装目录必须与源目录不同。

`qe_scf` / `qe_nscf` 支持 `-i/--input`、`-c/--config`、`-r/--run`、`-s/--submit`；默认配置 `input.toml`。仅准备时不运行，`-r` 实际执行，`-s` 提交 `run_qe.sh`；运行和提交互斥。NSCF 继承 SCF 的 XML、电荷密度和 PAW 数据。

## QE 声子一次提交

[MgB₂ 分机器算例](examples/MgB2/README.md) 支持 index、coords 和 nosplit。分 q 模式仅计算一次 SCF，入口自动安排所有 q 点与依赖后的合并/绘图；详见 [0006](docs/specs/0006-qe-phonon-submit-chain/spec.md)。

## 命令速查

- **QE**：`qe_scf` `qe_nscf` `qe_eband` `qe_eledos` `qe_process_eband` `qe_process_eledos` `qe_ph_nosplit` `qe_merge` `qe_q2r` `qe_matdyn_phband` `qe_matdyn_phdos` `qe_process_phonon` `qe_split_submit` `qe_check` `qe_phonon_nosplit` `qe_phonon_split_index` `qe_phonon_split_coords`
- **VASP**：`vasp_scf` `vasp_scf_converge` `vasp_eband` `vasp_process_eband` `vasp_eledos` `vasp_process_eledos` `vasp_idos_computed` `vasp_fermi` `vasp_cohp_uniform` `vasp_cohp_fatband` `vasp_lobsterin` `vasp_plot_cohp` `vasp_plot_kcohp` `vasp_locpot` `vasp_process_locpot` `vasp_charge` `vasp_charge_diff` `vasp_opt` `vasp_elf`
- **Wannier**：`wannier_init` `wannier_post` `wannier_window` `wannier_plot`
- **Substitution**：`substitution` `substitution_structure` `substitution_continue` `substitution_prepare_potcar` `substitution_prepare_qe_pseudo` `substitution_plot_eledos` `substitution_check_phonon_stability`
- **scflow**：`scflow -c input.toml`（λ/ω_log/Tc、a2F、Eliashberg；详见 [scflow README](scflow/README.md) 与 [功能总览](docs/overview/scflow.md)）
- **MatterSim（Python 例外）**：`pip install .` 后使用 `mattersim_opt`

配置契约（`input.toml` 递归覆盖、状态机、容差）见 [0004 契约](docs/specs/0004-dftflow-rust/contracts/config-and-output.md)。

## High Throughput Substitution

- `mattersim_opt` 通过外部 MatterSim Python 环境运行两阶段预优化：先只优化原子坐标，再在目标压力下联合优化晶格和原子坐标，输出 `mattersim_opt/CONTCAR` 供后续 `vasp_opt` 使用。
- `substitution_structure` 是高通量替换工作流的 prepare 步骤：从 `[substitutionflow]` 读取 `XA/XB` 原子序号范围，生成 `<Z>.XA_<元素>/period_n/<Z>.<formula>` 目录树，并同时写出替换结构、case `input.toml` 与初始状态文件。
- 每个 `period_n/` 目录保存一份静态 `workflow_manifest.json`，只记录输入结构、`XA` 元素、压力列表和该组 case 清单。
- 每个 case 下生成的 `input.toml` 是后续 `vasp_*`、`qe_phonon_*` 和 `substitution_continue` 的统一配置真源；它默认完整继承根配置中的共享块和子工作流参数块，只对 `chgcar_path`、`wavecar_path` 做 case 级相对路径重写。
- 每个 case 的 `case_status.json` 记录各压力点在 `mattersim_opt / vasp_opt / vasp_scf / vasp_eledos / vasp_process_eledos / qe_phonon` 的 `finished / error / unstarted` 状态（OUTCAR 启发式判定，无 daemon）；根目录 `workflow_status.json` 汇总供续算。
- `[substitutionflow].submit_groups` 控制 `-s/--submit` 的提交分组；一个 `case + pressure` 是不可拆分的完整提交单元，逐组 `sbatch submit_group_NN.sh`。
- `[substitutionflow].phonon_mode` 控制末尾 QE 声子总工作流，允许 `nosplit / split_index / split_coords`，默认 `split_index`；split 模式下 `qe_split_submit` 会进入 `NN/` 目录逐 q 点 `sbatch run_split.sh`。
- `substitution_continue -o <dir> -c <input.toml>`（`-rfs` 刷新状态 / `-rws` 重写 error、`-r/-s` 运行/提交）按状态文件从第一个非 `finished` 步骤续算。
- `substitution_prepare_potcar --pot <lib>` 与 `substitution_prepare_qe_pseudo --pp <dir>` 准备本地 `potcar_lib/`、`qe_pseudo_lib/`（交互式选号，选择记录即时写盘）。
- `substitution_plot_eledos` / `substitution_check_phonon_stability` 输出周期表式批量总览（`eledos_summary_<P>.*`、`phonon_stability_summary_<P>.png`；最小频率 `< -1.0 THz` 判 `unstable`）。

示例 `input_substitution.toml` 片段：

```toml
pp_dir = "qe_pseudo_lib"

[qe]
executable_path = "/opt/qe/bin"
lattice_mode = "ibrav_angstrom"

# lattice_mode 允许：
# - "ibrav_angstrom"：默认，自动判断 ibrav 并写 A/B/C/cos*
# - "ibrav_celldm"：自动判断 ibrav 并写 celldm(n)
# - "cell_parameters"：旧格式，写 ibrav = 0 + CELL_PARAMETERS {angstrom}
# 不要手动写 ibrav、A/B/C、cosAB/cosAC/cosBC 或 celldm(1..6)。

[qe_phonon]
prefix = "ht_ph"

[qe_phonon.ph]
qpoints = [2, 2, 1]
recover = true

[substitutionflow]
XA = "3,4"
XB = "3-5"
pressures = [20, 50]
structure_filename = "XAXBH7-P63mmc.vasp"
phonon_mode = "split_index"
```

## 展示算例

项目展示算例统一放在 [examples/](examples/README.md)，按体系和机器组织；运行输出保存在不跟踪的 `run/` 目录。

## 文档

- 实现总览：[docs/overview/dftflow-rust.md](docs/overview/dftflow-rust.md)
- Spec / 契约 / 验收：[docs/specs/0004-dftflow-rust/](docs/specs/0004-dftflow-rust/spec.md)
- 各 flow 的 README：`qeflow/` `vaspflow/` `scflow/` `wannier90flow/` `substitutionflow/` `mattersimflow/`
- 架构决策：[docs/adr/](docs/adr/README.md)
