# dftflow

DFT 计算工作流自动化（Issue 0004：全量 Rust 重写）。

五个 Rust 家族程序为 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`；`mattersim_opt` 按约定保留 Python。

## 分支约定

- `master`：默认主分支，承接原 `rust` 分支的 Rust 实现及后续开发。
- `python`：保留原 `master` 分支的 Python 版本，切换时的提交为 `7a172fe`。
- `python-reference-0004`：迁移数值对照使用的冻结标签（`9396b3a`），与 `python` 分支用途不同，继续保留。

## 编译产物与安装目录

编译和安装是两步：Cargo 先把产物写进仓库的 `target/`，再把需要使用的程序更新到固定 `current/bin/` 目录，最后在 `~/.local/bin` 创建命令软链接。`~` 是当前用户的家目录；下面的路径是本机采用的安装布局，不是 Cargo 强制指定的目录。

```text
仓库/
├── target/linux-x86_64/x86_64-unknown-linux-gnu/release/  Linux 原生构建产物
├── target/macos-arm64/aarch64-apple-darwin/release/      Apple Silicon 构建产物
├── dist/linux-x86_64-musl/                              Linux 静态交付包
└── docs/local/                                        本机环境、构建日志与验收证据

~/.local/
├── bin/                                               PATH 中的命令入口
│   ├── qeflow -> ../share/dftflow/current/bin/qeflow
│   ├── qe_phonon_split_coords -> 同一个 qeflow
└── share/dftflow/
    ├── current/bin/                            五个家族程序的当前安装副本
    ├── native/linux-x86_64/                            本机原生依赖与兼容运行库
    │   ├── include/                                   OpenBLAS、spglib 等头文件
    │   ├── lib/                                       库、私有 glibc 加载器及 pkgconfig/
    │   ├── lib64/                                     本机 spglib 静态库等及 pkgconfig/
    │   └── bin/                                       依赖工具预留目录，本机目前为空
    └── tools/qf-gen-dyn0/current/bin/                   独立 dyn0 工具
```

图中软链接是示意关系，安装器实际写入固定程序目录的绝对路径。各目录用途如下：

| 目录 | 存放内容与用途 |
| --- | --- |
| 仓库 `target/` | 编译缓存、中间文件和原始可执行文件；不同平台单独存放，`release/` 中还有 Cargo 的其他产物。清理构建缓存不会删除已经复制到 `current/bin/` 的安装副本。 |
| `current/bin/` | 当前使用的 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`。更新覆盖同一路径，不保留时间戳版本。 |
| `native/linux-x86_64/include/` | 编译需要的 C 头文件，例如 `cblas.h`、`openblas_config.h`、`spglib.h`。 |
| `native/linux-x86_64/lib/` | 本机 OpenBLAS 0.3.30 的 `libopenblas.a`、绘图所需字体等共享库，以及为旧 glibc 计算节点准备的加载器、libc 等运行库。`pkgconfig/*.pc` 告诉编译工具到哪里找头文件和库。 |
| `native/linux-x86_64/lib64/` | 本机 spglib 2.6.0 的 `libsymspg.a` 和相关构建元数据；`lib`/`lib64` 是依赖安装工具的目录选择，并不表示本机同时装了 32 位和 64 位两套程序。 |
| `tools/` | 不属于五个家族程序的独立工具，目前是 `qf-gen-dyn0`；由它自己的 Cargo 工程单独编译和安装。 |
| `~/.local/bin/` | 用户输入的 `qe_*` 等命令所在位置。软链接决定当前使用哪个版本，加入 PATH 后无需每次输入完整路径。 |
| 仓库 `docs/local/` | 不入 Git 的机器路径、编译环境、日志、制品哈希和测试图片；不是运行程序必须遍历的安装目录。 |

选择 `~/.local/share/dftflow/` 是为了使用用户权限安装，集中保存当前程序、原生依赖和独立工具。主程序入口固定指向 `current/bin/`，后续更新无需调整 PATH；安装器默认不再备份旧入口。历史 `.dftflow-backup-*` 不属于当前安装流程。

更新按程序逐文件原子替换；已经运行的进程继续使用原文件，新启动的命令使用更新后的程序。五个程序并非整组同时切换，应在没有活动计算链时更新，避免同一链使用不同程序版本。生成脚本引用 `current` 时，提交后的配置快照仍固定，但程序本身不锁定历史版本。`native/` 被私有加载器引用，必须保留。

历史安装副本与构建缓存的清理方法见 [编译历史清理指南](docs/specs/0011-build-history-cleanup/quickstart.md)。

## 编译依赖：OpenBLAS 与两平台安装

项目的 `ndarray-linalg` 启用 `openblas-static` 和 `openblas-system`：需要能够被编译工具找到的本机 OpenBLAS 开发文件和静态库 `libopenblas.a`，使用默认 LP64 接口。这里的 OpenBLAS 是 Rust 数值计算的依赖，与 QE 自身使用哪套 BLAS 独立；只安装 Python 的 numpy 并不能替代这些开发文件。另需 spglib 头文件/库、libclang，以及字体绘图依赖。

### macOS：Apple Silicon

当前原生配置支持 macOS arm64，使用 `/opt/homebrew` 下的 Homebrew；Intel Mac 不在此脚本的平台分派范围内。

```bash
# 首次安装 Apple 命令行开发工具；已安装时无需重复
xcode-select --install

# 安装依赖；gcc 提供 gfortran，llvm 提供 libclang
brew install openblas spglib gcc cmake pkg-config llvm fontconfig freetype

export PKG_CONFIG_PATH="$(brew --prefix openblas)/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LIBRARY_PATH="$(brew --prefix openblas)/lib:$(brew --prefix)/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"

# 验证 OpenBLAS 开发文件可见
pkg-config --modversion openblas
test -f "$(brew --prefix openblas)/lib/libopenblas.a"
```

另需安装 Rust/Cargo，可按 [Rust 官方安装指南](https://www.rust-lang.org/tools/install) 使用 rustup。Homebrew 依赖和说明见 [OpenBLAS 配方](https://formulae.brew.sh/formula/openblas)。上面的 `export` 只影响当前终端，也可保存到本地 `docs/local/build-macos-arm64.env`；不要复制 Linux 的 glibc 或 Linux 库到 macOS。

### Linux：系统开发包

以 Debian/Ubuntu 为例，有管理员权限时可安装：

```bash
sudo apt-get update
sudo apt-get install build-essential gfortran cmake pkg-config clang libclang-dev \
    libopenblas-dev libsymspg-dev libfontconfig1-dev libfreetype6-dev

pkg-config --modversion openblas
pkg-config --libs --static openblas
```

发行版包名可能不同，例如 RHEL 系通常使用 `*-devel` 名称；按管理员提供的开发环境配置。这里要安装开发包，而不只是运行时共享库。系统 OpenBLAS 版本由发行版决定；如果需要复现本机的 0.3.30、没有管理员权限，或集群节点需要统一依赖，可以使用下面的用户前缀源码安装。

### Linux：本机采用的 OpenBLAS 源码方案

本机使用 OpenBLAS **0.3.30、GENERIC、单线程、静态库**；`GENERIC` 避免仅针对编译机的 CPU 指令调优，`USE_THREAD=0` 避免数值库内部线程与工作流并行叠加。它不替代 QE/MPI 的并行设置。

以下命令在专门的依赖源码构建目录执行，需要已有 C/Fortran 编译器和下载工具：

```bash
export DFTFLOW_NATIVE_PREFIX="$HOME/.local/share/dftflow/native/linux-x86_64"
mkdir -p "$DFTFLOW_NATIVE_PREFIX"

curl -fL https://github.com/OpenMathLib/OpenBLAS/releases/download/v0.3.30/OpenBLAS-0.3.30.tar.gz \
    -o OpenBLAS-0.3.30.tar.gz
tar -xzf OpenBLAS-0.3.30.tar.gz
cd OpenBLAS-0.3.30
make -j4 TARGET=GENERIC DYNAMIC_ARCH=0 USE_THREAD=0 NO_SHARED=1
make PREFIX="$DFTFLOW_NATIVE_PREFIX" TARGET=GENERIC DYNAMIC_ARCH=0 USE_THREAD=0 NO_SHARED=1 install
cd ..
```

`NO_SHARED=1` 只构建静态 OpenBLAS；构建和安装时使用同一组选项。参考 [OpenBLAS 官方安装说明](https://www.openmathlib.org/OpenBLAS/docs/install/)；本项目静态交付的固定参数也可查阅 [Dockerfile](scripts/linux/Dockerfile)。

本机 spglib **2.6.0** 同样采用源码静态安装，启用 PIC 以支持 Linux 原生程序的 PIE 链接：

```bash
curl -fL https://codeload.github.com/spglib/spglib/tar.gz/refs/tags/v2.6.0 \
    -o spglib-2.6.0.tar.gz
tar -xzf spglib-2.6.0.tar.gz
cmake -S spglib-2.6.0 -B spglib-build \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DSPGLIB_SHARED_LIBS=OFF -DSPGLIB_WITH_Fortran=OFF -DSPGLIB_WITH_TESTS=OFF \
    -DCMAKE_INSTALL_PREFIX="$DFTFLOW_NATIVE_PREFIX"
cmake --build spglib-build -j4
cmake --install spglib-build
```

这两步不安装编译器、libclang 或字体开发库；无管理员权限时仍需集群模块或管理员提供它们。`libsymspg.a` 可能安装在 `lib/` 或 `lib64/`，均应纳入依赖搜索路径。

## 构建命令

在仓库根目录执行，Rust/Cargo 与上述原生依赖应已就绪：

```bash
# 五个家族程序的 release 构建
bash scripts/build-native.sh

# 完整 release 测试
bash scripts/build-native.sh test --release --workspace --locked

# 仅修改 qeflow 时，构建对应家族
bash scripts/build-native.sh build --release --locked -p qeflow

# Rust 依赖已缓存时，可明确禁止 Cargo 联网
bash scripts/build-native.sh build --release --locked --offline -p qeflow
```

脚本按 `uname` 选择 `.cargo/linux-x86_64.toml` 或 `.cargo/macos-arm64.toml`，先加载不入 Git 的 `docs/local/build-<平台>.env`。无参数时实际执行 `cargo --config .cargo/<平台>.toml build --release --workspace --locked`；`--locked` 固定 Cargo.lock，`--offline` 需要依赖已完整缓存。

Linux 用户前缀的本机环境文件可以按下例编写；路径根据机器调整：

```bash
# docs/local/build-linux-x86_64.env
export DFTFLOW_NATIVE_PREFIX="$HOME/.local/share/dftflow/native/linux-x86_64"
export RUSTFLAGS="-L native=$DFTFLOW_NATIVE_PREFIX/lib -L native=$DFTFLOW_NATIVE_PREFIX/lib64 -C link-arg=-Wl,-rpath,$DFTFLOW_NATIVE_PREFIX/lib -C link-arg=-Wl,-rpath,$DFTFLOW_NATIVE_PREFIX/lib64"
# 如 bindgen 无法发现 libclang，再按实际安装位置设置 LIBCLANG_PATH
```

构建脚本自动为该前缀设置 `PKG_CONFIG_PATH`、`LIBRARY_PATH` 和头文件搜索路径。依赖在系统目录中时不必使用这个前缀；静态 OpenBLAS/spglib 不意味着整个 glibc 程序已完全静态链接，字体库、C/Fortran 运行库等仍可能是动态依赖。

| 构建平台 | Cargo 配置 | 二进制目录 |
| --- | --- | --- |
| Linux x86_64 原生（glibc） | `.cargo/linux-x86_64.toml` | `target/linux-x86_64/x86_64-unknown-linux-gnu/release/` |
| macOS arm64 原生 | `.cargo/macos-arm64.toml` | `target/macos-arm64/aarch64-apple-darwin/release/` |
| Linux x86_64 musl 静态交付 | `.cargo/linux-host.toml` + `scripts/linux/Dockerfile` | `dist/linux-x86_64-musl/bin/` |

macOS Mach-O 不能直接在 Linux 执行。在 macOS Docker/OrbStack 或安装了 Docker 的 Linux 上，以下脚本构建真正的 Linux amd64 ELF，安装容器内原生依赖并检查静态链接和断网运行；不是使用宿主机的 Homebrew OpenBLAS 交叉链接：

```bash
bash scripts/linux/build.sh
```

它使用单独 Docker 构建卷保存中间产物，把交付包导出到 `dist/linux-x86_64-musl/`。首次需要下载依赖、基础镜像及 Cargo 包。运行 QE/VASP 等计算仍需要目标机器上配置外部求解器。

## 安装、更新与旧节点兼容

普通原生安装：编译完成后调用固定路径安装器。Linux 示例（macOS 使用对应构建目录）：

```bash
build_bin="$PWD/target/linux-x86_64/x86_64-unknown-linux-gnu/release"
bash scripts/install-current.sh "$build_bin"
```

脚本先复制到临时目录并验证 51 个入口，再逐文件替换 `~/.local/share/dftflow/current/bin/`，最后更新 `~/.local/bin/` 的固定软链接；成功或失败退出均清理临时副本。支持额外参数指定安装根目录与入口目录，不生成历史版本。

安装器提供 48 个旧名及家族入口，共 51 个命令名，底层是五份二进制；按入口名字分派到对应子命令。已有同名入口默认覆盖（设置 `DFTFLOW_BACKUP_ENTRIES=1` 才备份），安装目录必须与源目录不同。独立 [dyn0 工具](tools/qf-gen-dyn0/README.md) 从自己的 Cargo 工程编译，不由主安装器安装。

本机登录节点与计算节点的 glibc 不一致，实际安装副本额外设置了私有加载器和运行库搜索路径，原始 `target/` 产物保留。这是当前共享文件系统上的兼容打包方式：

```bash
# Linux x86_64；先安装 patchelf，并配置本机 DFTFLOW_NATIVE_PREFIX
package_dir=$(mktemp -d)
bash scripts/linux/package-native-runtime.sh \
    "$build_bin" "$package_dir" "$DFTFLOW_NATIVE_PREFIX"
bash scripts/install-current.sh "$package_dir"
rm -r -- "$package_dir"
```

先完成兼容打包，再安装到 `current`。打包设置私有 ELF interpreter 和 rpath；计算节点必须能访问同一共享路径。不要把含私有 glibc 的目录加入全局 `LD_LIBRARY_PATH`。此方式绑定本机运行库路径；通用 Linux 静态交付仍使用 musl 包。

`qe_scf` / `qe_nscf` 支持 `-i/--input`、`-c/--config`、`-r/--run`、`-s/--submit`；默认配置 `input.toml`。仅准备时不运行，`-r` 实际执行，`-s` 提交 `run_qe.sh`；运行和提交互斥。NSCF 继承 SCF 的 XML、电荷密度和 PAW 数据。

## QE 声子一次提交

[MgB₂ 分机器算例](examples/MgB2/README.md) 支持 index、coords 和 nosplit。分 q 模式仅计算一次 SCF，入口自动安排所有 q 点与依赖后的合并/绘图；详见 [0006](docs/specs/0006-qe-phonon-submit-chain/spec.md)。

三种声子聚合入口均支持 `-i/--input`（别名 `-p/--poscar`），例如 `qe_phonon_nosplit -c input.toml -i POSCAR`。nosplit 不传结构参数时优先复用工作目录 POSCAR，再查找当前目录 POSCAR/CONTCAR；完整执行边界见 [nosplit 说明](docs/overview/qeflow/qe_phonon_nosplit.md)。

已有声子后处理结果可通过 `qe_process_phonon -c input.toml -i POSCAR --plot` 仅更新 CSV 和图；重绘前备份旧图。声子谱使用高对称点路径横轴和完整频率纵轴，联合图右侧 DOS 共享频率轴，见 [0009 验收](docs/specs/0009-phonon-band-plot-rewrite/acceptance.md)。

## 命令速查

- **QE**：`qe_scf` `qe_nscf` `qe_eband` `qe_eledos` `qe_process_eband` `qe_process_eledos` `qe_ph_nosplit` `qe_merge` `qe_q2r` `qe_matdyn_phband` `qe_matdyn_phdos` `qe_process_phonon` `qe_split_submit` `qe_check` `qe_phonon_nosplit` `qe_phonon_split_index` `qe_phonon_split_coords`
- **VASP**：`vasp_scf` `vasp_scf_converge` `vasp_eband` `vasp_process_eband` `vasp_eledos` `vasp_process_eledos` `vasp_idos_computed` `vasp_fermi` `vasp_cohp_uniform` `vasp_cohp_fatband` `vasp_lobsterin` `vasp_plot_cohp` `vasp_plot_kcohp` `vasp_locpot` `vasp_process_locpot` `vasp_charge` `vasp_charge_diff` `vasp_opt` `vasp_elf`
- **Wannier**：`wannier_init` `wannier_post` `wannier_window` `wannier_plot`
- **Substitution**：`substitution` `substitution_structure` `substitution_continue` `substitution_prepare_potcar` `substitution_prepare_qe_pseudo` `substitution_plot_eledos` `substitution_check_phonon_stability`
- **scflow**：`scflow -c input.toml`（λ/ω_log/Tc、a2F、Eliashberg；详见 [scflow README](scflow/README.md) 与 [功能总览](docs/overview/scflow.md)）
- **MatterSim（Python 例外）**：`pip install .` 后使用 `mattersim_opt`

配置契约（`input.toml` 递归覆盖、状态机、容差）见 [0004 契约](docs/specs/0004-dftflow-rust/contracts/config-and-output.md)。

Wannier 自动选窗已统一采用指定 AiiDA 工作流的默认“能量冻结＋投影解纠缠”：`wannier90 window data-file-schema.xml --num-wann 5`。旧 `--auto nbnd1 num_wann nbnd3` 已弃用；初始化需配置实际 NSCF `bands_file`，epwflow 则从 F2 清单绑定本征值。实际程序需支持投影参数，不能静默退回旧方法，详见 [选窗使用说明](docs/specs/0016-wannier-window-selection/quickstart.md)。

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

声子 DOS 默认插值网格是 PH 网格各方向的两倍，显式 `[qe_phonon.phdos].qpoints` 优先。绘图高斯平滑默认 `plot_sigma_cm1 = 5.0`（cm⁻¹），调大更平滑、设为 0 关闭；使用 `qe_process_phonon --plot` 重画，原始 DOS 和 CSV 保留。

声子聚合流程保存 `input-init.toml` 和原结构名加 `-init` 的备份，已提交链路读取工作目录快照。修改外部配置后需用原目录的 `input-init.toml` 续算；旧目录和已生成旧脚本不会因安装新版而自动迁移。
