# Quantum ESPRESSO（q-e）在 Apple M5 上的安装执行手册

适用场景：在本机（Apple Silicon M5，macOS arm64）从 `external/q-e` 子模块编译安装 Quantum ESPRESSO，供 dftflow 的 `qeflow` 调用 `pw.x` / `ph.x` / `q2r.x` / `matdyn.x` 等。

> 本手册记录的命令均已在本机（GCC 16、Homebrew arm64）实测通过。M5 与 M1–M4 同为 arm64 架构，步骤完全通用。

---

## 一、环境与前置条件

- **芯片**：Apple M5（arm64）。
- **系统**：macOS 较新版本，已安装 Xcode Command Line Tools：`xcode-select --install`。
- **包管理器**：原生 arm64 版 Homebrew，路径 `/opt/homebrew`。**切勿混用 x86_64 版或 Rosetta**，否则架构不匹配。
- **网络**：能访问 `gitlab.com` 与 `formulae.brew.sh`（编译时 CMake 会拉取 q-e 内部子模块）。
- **子模块现状**：`external/q-e` 远程为 `https://gitlab.com/madegan/q-e.git`，远程默认分支 `develop`。编译前建议先更新到最新（见步骤一）。

---

## 二、步骤一：更新 q-e 子模块到最新（推荐）

`external/q-e` 是 dftflow 的 Git submodule。进入子模块拉取最新代码：

```bash
cd /Users/mayuan/Codes/dftflow/external/q-e
git fetch origin
git checkout develop
git pull --ff-only origin develop
```

> 分支选择：`develop`（与现状一致，默认）/`master`（稳定标签）/`develop_omp5`（OpenMP 5 路线）。本手册后续编译基于已更新的最新代码。

---

## 三、步骤二：安装依赖（Homebrew）

```bash
brew install gcc open-mpi fftw cmake
brew install openblas        # BLAS/LAPACK（也可改用系统 Accelerate，见踩坑 3）
brew install hdf5            # 并行 HDF5 IO（若关闭 HDF5 可不装）
```

依赖用途：

| 依赖 | 用途 |
|------|------|
| `gcc` | 提供 `gfortran`（**关键**，QE 的 Fortran 不推荐用 Apple Clang） |
| `open-mpi` | MPI 并行 |
| `fftw` | FFT 变换 |
| `openblas` | BLAS/LAPACK（或改用系统 Accelerate） |
| `cmake` | 构建系统，需 ≥ 3.20 |
| `hdf5` | 可选，HDF5 并行 IO |

确认编译器版本（Homebrew gcc 版本号会随更新变化，**必须按实际填写**）：

```bash
brew list gcc | grep -E 'bin/(gcc|g\+\+|gfortran)'
gfortran --version
```

本机实测为 **GCC 16**：`gcc-16` / `g++-16` / `gfortran`（无版本号的 `gfortran` 软链存在，但 `gcc`/`g++` 只有版本化名 `gcc-16`/`g++-16`）。

---

## 四、步骤三：CMake 配置

进入源码目录，新建独立 `build` 目录（**不可与源码目录相同**，否则 CMake 直接报错）：

```bash
cd /Users/mayuan/Codes/dftflow/external/q-e
mkdir -p build && cd build
```

运行配置。**注意：多行命令里不要写行内 `# 注释`**（见踩坑 1）：

```bash
cmake .. -DCMAKE_Fortran_COMPILER=gfortran -DCMAKE_C_COMPILER=gcc-16 -DCMAKE_CXX_COMPILER=g++-16 -DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_PREFIX_PATH="/opt/homebrew/opt/openblas;/opt/homebrew/opt/hdf5" -DCMAKE_INSTALL_PREFIX=/opt/homebrew/opt/qe -DBLA_VENDOR=OpenBLAS -DQE_ENABLE_OPENMP=ON -DQE_ENABLE_HDF5=ON
```

关键参数说明：

- `-DCMAKE_Fortran_COMPILER=gfortran` / `-DCMAKE_C_COMPILER=gcc-16` / `-DCMAKE_CXX_COMPILER=g++-16`：必须用 Homebrew gcc 工具链，版本号按 `brew list gcc` 实际结果填。
- `-DCMAKE_OSX_ARCHITECTURES=arm64`：强制 arm64，避免误编 x86_64。
- `-DCMAKE_PREFIX_PATH="/opt/homebrew/opt/openblas;/opt/homebrew/opt/hdf5"`：**关键**，让 CMake 找到 Homebrew 的 OpenBLAS 与 HDF5（见踩坑 3）。
- `-DBLA_VENDOR=OpenBLAS`：使用 OpenBLAS。若改用系统 Accelerate，改为 `-DBLA_VENDOR=Apple` 并去掉上面的 openblas prefix。
- `-DQE_ENABLE_OPENMP=ON`：多核加速（gcc 工具链下有效）。
- `-DQE_ENABLE_HDF5=ON`：开启 HDF5（需已装 `hdf5`）。

配置成功标志：`Configuring done` / `Build files have been written to: .../build`，且输出中出现 `Found FFTW3`、`OpenMP`、`MPI`，无 `Could NOT find BLAS/LAPACK` 报错。`CMAKE_CXX_COMPILER unused` 警告无害（QE 只用 Fortran/C）。

---

## 五、步骤四：编译

```bash
cd /Users/mayuan/Codes/dftflow/external/q-e/build
make -j$(sysctl -n hw.ncpu)
```

本机实测全量编译（含 EPW）约数十分钟，仅有警告（EPW 的 `MPI_FILE_READ` 类型不匹配警告、链接时 duplicate libraries 警告）无错误，退出码 0。

如只需 qeflow 常用组件，可只编部分以省时间：

```bash
make pw ph pp        # pw.x / ph.x(含 q2r.x、matdyn.x) / pp.x(含 bands.x、dos.x)
```

---

## 六、步骤五：验证与安装

验证二进制生成（位于 `build/bin`）：

```bash
ls /Users/mayuan/Codes/dftflow/external/q-e/build/bin/ | grep -E '^(pw|ph|q2r|matdyn|bands|dos|pp|epw)\.x$'
/Users/mayuan/Codes/dftflow/external/q-e/build/bin/pw.x -h
```

可选安装到 `CMAKE_INSTALL_PREFIX`（即 `/opt/homebrew/opt/qe`）：

```bash
make install
```

### 并行（MPI / OpenMP）能力验证与用法

本手册的 CMake 配置已同时开启 MPI（`-DQE_ENABLE_MPI_MODULE=ON`）与 OpenMP（`-DQE_ENABLE_OPENMP=ON`），编译出的 `.x` 二进制**本身就是 MPI 版**（不再区分串/并行命名）。

**验证 MPI 已编入并可用**（用 `otool` 看链接，再用 `mpirun` 看启动横幅）：

```bash
otool -L /Users/mayuan/Codes/dftflow/external/q-e/build/bin/pw.x | grep -iE 'mpi|omp'
# 期望看到 libmpi_usempif08 / libmpi (OpenMPI) 与 libgomp (OpenMP)

mpirun -np 2 /Users/mayuan/Codes/dftflow/external/q-e/build/bin/pw.x
# 期望打印 "Program PWSCF v.7.5 starts"，说明 2 个 MPI 进程初始化并通信成功
```

> 注意：`pw.x -h` 单进程也会返回退出码 1（QE 无输入即退出的正常行为），**不能用 `-h` 验证 MPI**；应看启动横幅。

**基本并行调用**（OpenMPI 的 `mpirun` 与 `mpiexec` 等价，`/opt/homebrew/bin/mpirun` 已就绪）：

```bash
mpirun -np 4 /Users/mayuan/Codes/dftflow/external/q-e/build/bin/pw.x < si.scf.in > si.scf.out
```

**MPI × OpenMP 混合并行**（本次编入了 OpenMP）：设 `OMP_NUM_THREADS`，例如 4 MPI 进程 × 2 线程 = 8 线程：

```bash
export OMP_NUM_THREADS=2
mpirun -np 4 /Users/mayuan/Codes/dftflow/external/q-e/build/bin/pw.x < si.scf.in > si.scf.out
```

**实用提醒**：
- M5 核心数有限，**纯 MPI**（`-np` = 物理核数）通常最直接；混合并行在核多、内存带宽吃紧时更划算，笔记本上两种都可试。
- `-np` 不要超过 `sysctl -n hw.ncpu`（含性能核 + 能效核），超出反而因争抢变慢。
- `ph.x`（声子）等还支持 `-nimage` / `-npool` / `-northo` 等更细的并行切分，仅大体系需要。

---

## 七、步骤六：与 dftflow 衔接

三种方式任选其一让 `qeflow` 找到 QE 可执行文件：

1. 临时调用：直接写全路径 `/Users/mayuan/Codes/dftflow/external/q-e/build/bin/pw.x`。
2. 加入 `PATH`：`export PATH="/Users/mayuan/Codes/dftflow/external/q-e/build/bin:$PATH"`（可写进 `~/.zshrc` 持久化）。
3. 已 `make install` 时：用 `/opt/homebrew/opt/qe/bin`。

然后在 dftflow 的 `[qe]` 相关配置段把可执行路径指向上述 `bin` 目录即可。

---

## 八、踩坑记录（本次真实遇到的问题与解决）

### 踩坑 1：多行 `cmake` 命令里的行内 `# 注释` 打断了 `\` 续行

**现象**：

```
CMake Warning: Ignoring extra path from command line: "OpenBLAS"
zsh: command not found: -DQE_ENABLE_OPENMP=ON
zsh: command not found: -DQE_ENABLE_HDF5=ON
```

**原因**：在 zsh 中用 `\` 续行的多行命令里写了 `# 注释`，注释使续行链断裂，后面的 `-D...` 被当作独立命令执行，CMake 还把 `OpenBLAS` 误当成多余路径参数。

**解决**：CMake 命令内**不放任何行内注释**，注释单独成行或写在命令外。本手册步骤三的配置命令即为修正后的干净版本。

### 踩坑 2：`-DCMAKE_C_COMPILER=gcc-14` 找不到

**现象**：

```
The CMAKE_C_COMPILER: gcc-14 is not a full path and was not found in the PATH.
```

**原因**：机器上装的是 **GCC 16**，没有 `gcc-14`。Homebrew gcc 版本号随更新变化（`gcc`/`g++` 仅提供版本化命令如 `gcc-16`，`gfortran` 同时有带/不带版本号软链）。

**解决**：用 `brew list gcc` 与 `gfortran --version` 确认实际版本，本机填 `gcc-16` / `g++-16` / `gfortran`。

### 踩坑 3：`Could NOT find BLAS` / `Could NOT find LAPACK`

**现象**：

```
-- Could NOT find BLAS (missing: BLAS_LIBRARIES)
-- Could NOT find LAPACK (missing: LAPACK_LIBRARIES)
CMake Error at CMakeLists.txt:486 (message):
  Failed to find a complete set of external BLAS/LAPACK library ...
```

**原因**：Homebrew 的 OpenBLAS 库文件（`libopenblas.dylib` 等）**只存在于 `Cellar/openblas/<ver>/lib`，并未软链到 `/opt/homebrew/lib`**。光写 `-DCMAKE_PREFIX_PATH=/opt/homebrew` 仍搜不到；CMake 默认也不搜索 `/opt/homebrew`。

**解决**：把 prefix 直接指向 `opt` 下的具体包目录：

```
-DCMAKE_PREFIX_PATH="/opt/homebrew/opt/openblas;/opt/homebrew/opt/hdf5"
```

备选方案：直接用系统原生 **Accelerate** 框架（免装 OpenBLAS、无路径问题、原生 arm64 性能佳）：

```
-DBLA_VENDOR=Apple
```

并去掉 `openblas` 的 prefix（保留 hdf5 的即可）。

### 踩坑 4（无害）：`CMAKE_CXX_COMPILER` 未被使用

**现象**：配置末尾警告 `Manually-specified variables were not used by the project: CMAKE_CXX_COMPILER`。

**原因**：QE 仅用 Fortran/C，不编 C++，该变量被忽略属正常。

**解决**：无需处理。

### 踩坑 5：编译时自动拉取 q-e 内部子模块

**现象**：配置阶段 CMake 自动 `git clone` 了 `external/wannier90`、`external/mbd`、`external/devxlib` 等 q-e 内部子模块（需联网）。

**解决**：正常现象；之后在 dftflow 提交时，`git status` 会显示 `external/q-e` 指针变化（含嵌套子模块），按提交规范记录即可。

---

## 九、收尾提醒

- 若要先装「最新」q-e，务必先执行步骤一 `git pull`，再重跑步骤三/四；本机首次编译基于的 commit 为 `master` 分支 `770a0b2d`。
- 全量 `build` 目录较大（数 GB），确认磁盘余量。
- M5 为较新芯片：若遇未知指令集问题，先 `softwareupdate` 升级系统与 Xcode CLT 到支持 M5 的版本。
