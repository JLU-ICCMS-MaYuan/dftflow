# dftflow

DFT 计算工作流自动化（Issue 0004：全量 Rust 重写）。

五个 Rust 家族程序为 `qeflow`、`vaspflow`、`scflow`、`wannier90`、`substitution`；`mattersim_opt` 按约定保留 Python。

开发中的 [epwflow](epwflow/README.md) 提供 `epw_phonon`、`epw_wannier`、`epw_superconductivity` 三个阶段程序和历史清理工具 `epw_clear`，支持检查、脚本生成、提交和恢复；完整真实原生超导链仍在验收。qeflow 的 nosplit 流程现可通过 `[qe_phonon].retain_epw_data=true` 归档 EPW 所需声子数据，见 [保留数据说明](qeflow/README.md#为-epw-保留声子数据)。MgB₂ 运行数据位于 `test_auto/MgB2/local_run/` 的编号目录；源码、受测构建和当前安装分别核对。

## 分支约定

- `master`：默认主分支，承接原 `rust` 分支的 Rust 实现及后续开发。
- `python`：保留原 `master` 分支的 Python 版本，切换时的提交为 `7a172fe`。
- `python-reference-0004`：迁移数值对照使用的冻结标签（`9396b3a`），与 `python` 分支用途不同，继续保留。

## 编译产物与程序目录

受支持的交付构建入口在成功返回前自动将本次产品发布到仓库 `bin/<模块>/`。无需再执行第二次用户安装；仅编译过的模块更新，`test`、`check` 和库构建不发布 CLI。`bin/` 是本机制品目录，不提交 Git。

```text
仓库/bin/
├── epwflow/          epw_phonon、epw_wannier、epw_superconductivity、epw_clear
├── qeflow/           qeflow 与 qe_* 兼容入口
├── vaspflow/         vaspflow 与 vasp_* 兼容入口
├── substitutionflow/ substitution 与 substitution_* 兼容入口
├── scflow/           scflow
├── wannierflow/      wannier90 与 wannier_* 兼容入口
├── tools/            独立 qf-gen-dyn0
├── .build/           平台缓存、Cargo 原始制品及发布指纹
├── .native/          本机开发依赖、私有加载器与运行库
└── .history/         有迁移清单的历史证据，不加入 PATH
```

组内兼容入口使用相对软链接，例如 `bin/qeflow/qe_scf -> qeflow`。构建不包含外部 `pw.x`、`ph.x`、`epw.x` 或 VASP。独立 dyn0 仍是另一个 Cargo 工程，经同一构建入口发布到 `bin/tools/`。

旧 `scripts/install-current.sh` 已退役，不再维护 `~/.local/share/dftflow/`。原有安装依赖迁移、历史二进制归档与当前验证边界见 [0017 验收](docs/specs/0017-build-binaries-layout/acceptance.md)。

### 在任意目录直接使用命令

在 Bash 或 Zsh 的当前终端加载环境脚本，即可从任意目录使用命令：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
```

脚本从自身位置定位仓库，将七个模块目录加入并导出 `PATH`；重复加载不会重复添加，原有路径保留。`PATH` 不递归搜索子目录，因此需要分别加入家族目录。其他机器把上面的脚本位置改为实际路径即可；构建缓存和测试目录不加入 `PATH`。

如需新终端自动生效，将同一行手动加入所用 shell 的 `~/.bashrc` 或 `~/.zshrc`。Bash 登录 shell 还需其登录配置加载 `~/.bashrc` 或同一配置行。脚本只影响当前 shell 及其后代，不自动修改启动文件。完整步骤见 [PATH 配置与验证](docs/specs/0017-build-binaries-layout/quickstart.md#从任意目录使用命令)。

```bash
command -v epw_phonon epw_wannier epw_superconductivity epw_clear qe_scf
epw_phonon --help
```

确认显示的是仓库 `bin/<模块>/` 下的入口后，进入计算目录直接执行命令，例如 `epw_phonon -c epwinput.toml`。添加 `PATH` 本身不提交计算，也不迁移已有运行绑定。

更新按程序逐文件原子替换，不保证所有程序同时切换；应在没有活动计算链时更新。生成脚本和 EPW 身份记录可能固定了程序绝对路径、链接目标和哈希，仅复制到新目录不会自动迁移这些绑定；已有运行按其恢复约定处理，新运行使用 `bin/<模块>/`。

本次唯一入口与依赖迁移见 [0017 操作指南](docs/specs/0017-build-binaries-layout/quickstart.md)；旧安装清理的历史说明保留在 [0011](docs/specs/0011-build-history-cleanup/quickstart.md)，不作为新的安装契约。

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

先将下例 `/path/to/dftflow` 替换为仓库绝对路径，再在依赖源码构建目录执行；需要已有 C/Fortran 编译器和下载工具：

```bash
export DFTFLOW_NATIVE_PREFIX="/path/to/dftflow/bin/.native/linux-x86_64"
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
# 八个 workspace 程序的 release 构建
bash scripts/build-native.sh

# 完整 release 测试
bash scripts/build-native.sh test --release --workspace --locked

# 仅修改 qeflow 时，构建对应家族
bash scripts/build-native.sh build --release --locked -p qeflow

# Rust 依赖已缓存时，可明确禁止 Cargo 联网
bash scripts/build-native.sh build --release --locked --offline -p qeflow
```

交付构建另需 `jq`；Linux 配置私有依赖前缀时还需 `patchelf` 和匹配主机的运行库。脚本按 `uname` 选择 `.cargo/linux-x86_64.toml` 或 `.cargo/macos-arm64.toml`，先加载不入 Git 的 `docs/local/build-<平台>.env`。无参数时实际执行 `cargo --config .cargo/<平台>.toml build --release --workspace --locked`；`--locked` 固定 Cargo.lock，`--offline` 需要依赖已完整缓存。

Linux 用户前缀的本机环境文件可以按下例编写；路径根据机器调整：

```bash
# docs/local/build-linux-x86_64.env
export DFTFLOW_NATIVE_PREFIX="/path/to/dftflow/bin/.native/linux-x86_64"
export RUSTFLAGS="-L native=$DFTFLOW_NATIVE_PREFIX/lib -L native=$DFTFLOW_NATIVE_PREFIX/lib64 -C link-arg=-Wl,-rpath,$DFTFLOW_NATIVE_PREFIX/lib -C link-arg=-Wl,-rpath,$DFTFLOW_NATIVE_PREFIX/lib64"
# 如 bindgen 无法发现 libclang，再按实际安装位置设置 LIBCLANG_PATH
```

构建脚本自动为该前缀设置 `PKG_CONFIG_PATH`、`LIBRARY_PATH` 和头文件搜索路径。依赖在系统目录中时不必使用这个前缀；静态 OpenBLAS/spglib 不意味着整个 glibc 程序已完全静态链接，字体库、C/Fortran 运行库等仍可能是动态依赖。

| 构建平台 | Cargo 配置 | 二进制目录 |
| --- | --- | --- |
| Linux x86_64 原生（glibc） | `.cargo/linux-x86_64.toml` | `bin/.build/linux-x86_64/x86_64-unknown-linux-gnu/release/` |
| macOS arm64 原生 | `.cargo/macos-arm64.toml` | `bin/.build/macos-arm64/aarch64-apple-darwin/release/` |
| Linux x86_64 musl 静态交付 | `.cargo/linux-host.toml` + `scripts/linux/Dockerfile` | `dist/linux-x86_64-musl/bin/` |

macOS Mach-O 不能直接在 Linux 执行。在 macOS Docker/OrbStack 或安装了 Docker 的 Linux 上，以下脚本构建真正的 Linux amd64 ELF，安装容器内原生依赖并检查静态链接和断网运行；不是使用宿主机的 Homebrew OpenBLAS 交叉链接：

```bash
bash scripts/linux/build.sh
```

它使用单独 Docker 构建卷保存中间产物，把交付包导出到 `dist/linux-x86_64-musl/`。首次需要下载依赖、基础镜像及 Cargo 包。运行 QE/VASP 等计算仍需要目标机器上配置外部求解器。

## 更新与旧节点兼容

日常更新直接重新运行交付构建，例如：

```bash
bash scripts/build-native.sh build --release --locked -p epwflow --bins
bash scripts/build-native.sh build --release --locked --manifest-path tools/qf-gen-dyn0/Cargo.toml
source "/path/to/dftflow/scripts/env.sh"
```

构建入口读取本次 Cargo 制品报告，完成必要的兼容打包、全部选中入口启动检查与目标预检，再发布到固定模块路径。来源与发布后 SHA-256 记录在 `bin/.build/published/<程序>.json`。增量构建的 fresh 制品同样会发布；只构建 EPW 不更新其他模块。编译或预检失败保留现有程序。显式交叉目标只构建，不替换宿主模块程序。

Linux 上配置 `DFTFLOW_NATIVE_PREFIX` 时，自动设置私有 ELF interpreter 和运行库搜索路径；该共享前缀必须在目标节点可读。运行库与加载器必须配套；已有前缀出现库版本冲突时拒绝覆盖。不要把含私有 glibc 的目录加入全局 `LD_LIBRARY_PATH`。未配置私有前缀的原生程序使用系统运行库；跨节点兼容性应另行核验，通用静态交付仍使用 musl。

发布会检查选中产品的本机进程和 Slurm 脚本引用；发现活动引用时保留现有程序并报告作业或进程。发布按文件替换，不宣称整组事务；历史科学身份不会被修改。可用 `scripts/install-bin.sh --products 程序列表 /已验证制品目录` 显式发布外部准备的快照，日常构建无需另执行它。

`qe_scf` / `qe_nscf` 支持 `-i/--input`、`-c/--config`、`-r/--run`、`-s/--submit`；默认配置 `input.toml`。仅准备时不运行，`-r` 实际执行，`-s` 提交 `run_qe.sh`；运行和提交互斥。NSCF 继承 SCF 的 XML、电荷密度和 PAW 数据。

## QE／EPW 编号工作区

新 EPW 配置使用 `layout = "numbered"`、`work_dir = "."` 和根 `input.toml`。
阶段重算保留编号工作目录本身，只将旧内容归档到 `epw-state/history/`；已经进入目录的终端无需重新 `cd`，当前 Slurm 日志持续可读。
在工作区根执行 `epw_clear --dry-run` 预览三类历史，`epw_clear --yes` 全部清空；现用计算与结果保留。详见 [清理用法](docs/overview/epwflow/epw_clear.md)。
修改当前配置后直接执行 `epw_wannier --stage wannier -s`；已有相容的 QE 自动复用。
`--stage coarse` 到粗网格，`epw_superconductivity --stage fine` 到细网格。
第 3 步 QE 子任务完成即发布单侧图；第 5 步完成电子能带／总 DOS 对比，第 6 步完成声子谱／TDOS 对比及总览，图片、数据和来源按阶段保存在工作区根 `results/`。
新版阶段作业内自动完成后处理和发布，正常完成后无需再次执行公开命令触发出图；第 5 步不等待声子。详见[阶段结果说明](docs/specs/0050-epw-stage-results/quickstart.md)。
输入、`submit.sh` 和日志位于各实际计算目录，空 pool 在生成、提交及脚本启动时阻断。
旧工作区可只读核验后一次性导入，不为目录改版重跑 QE。
详见 [模块手册](epwflow/README.md) 和 [MgB₂ 直接操作说明](examples/MgB2/sugon-amd/run/README.md)。
更新三个 EPW 阶段程序及清理工具可使用 `bash scripts/build-native.sh build --release --locked -p epwflow --bins`。

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
