# DFTFlow Rust 架构、构建与安装

[返回功能总览](README.md)

## 当前交付物

主 workspace 生成九个程序：原五个家族程序 qeflow、vaspflow、scflow、wannier90、substitution，三个 EPW 阶段程序 epw_phonon、epw_wannier、epw_superconductivity，以及历史清理工具 [epw_clear](epwflow/epw_clear.md)。原五个家族通过 argv[0] 识别兼容命令，共 51 个唯一入口；加四个 EPW 程序后，主安装器验证 **55 个入口**。四个 EPW 入口是独立二进制，不分派到旧 epwflow 命令。

[独立 qf-gen-dyn0](tools/qf-gen-dyn0.md) 是另一个 Cargo 工程，单独构建，不计入主 workspace 的九个目标或 55 个入口；分组安装器可通过显式参数一并安装它。保留的 Python mattersim_opt 通过 pyproject.toml 提供，不由 Rust 安装器安装。源码中的语言迁移范围与历史验收见 [0004](../specs/0004-dftflow-rust/spec.md)。

| 程序 | 实现职责 | 用法导航 |
| --- | --- | --- |
| qeflow | QE 输入、状态、依赖链和后处理 | [CLI](qeflow/README.md) |
| vaspflow | VASP/LOBSTER 输入与数据分析 | [CLI](vaspflow/README.md) |
| wannier90 | Wannier 接口、能窗与绘图 | [CLI](wannierflow/README.md) |
| scflow | EPC 数值分析与结果输出 | [CLI](scflow/README.md) |
| substitution | 元素替换、状态与跨 flow 调用 | [CLI](substitutionflow/README.md) |
| 三个 EPW 阶段程序 | QE 声子数据、Wannier／插值、细网格与原生超导流程；科学验收边界见模块页 | [CLI](epwflow/README.md) |

## 公共模块关系

CLI 读取 [qf-config](../../crates/qf-config/src/lib.rs) 的配置，经 qf-crystal/qf-input/qf-io 处理结构、输入与数据；qf-runner 运行外部命令和作业，qf-state 维护文件状态。声子专用拆分、合并、归档与绘图在 qf-phonon，超导数值在 qf-scflow，VASP 与替换专用逻辑在 df-vasp/df-substitution。

QE 0006 的完整调度在 [phonon_chain.rs](../../qeflow/src/phonon_chain.rs)，单独初始化在 [phonon_init.rs](../../qeflow/src/phonon_init.rs)。两者使用真实 QE only_init 获取列表；不依赖独立 dyn0 算法。工作流状态文件与 Slurm 作业记录共同用于恢复，不需要常驻服务或数据库。

## 按平台构建

在仓库根目录运行：

```bash
bash scripts/build-native.sh
# 已具备全部依赖缓存时可显式离线构建
bash scripts/build-native.sh build --release --workspace --locked --offline
```

[build-native.sh](../../scripts/build-native.sh) 自动选择平台，避免来回修改同一份编译文件：

| 平台 | 配置 | 默认产物目录 |
| --- | --- | --- |
| Linux x86_64 原生 | [.cargo/linux-x86_64.toml](../../.cargo/linux-x86_64.toml) | bin/.build/linux-x86_64/x86_64-unknown-linux-gnu/release |
| macOS arm64 原生 | [.cargo/macos-arm64.toml](../../.cargo/macos-arm64.toml) | bin/.build/macos-arm64/aarch64-apple-darwin/release |

平台公共配置保持独立；本机依赖路径可由忽略目录中的 `build-<platform>.env` 和 DFTFLOW_NATIVE_PREFIX 提供。musl 另走 [scripts/linux/build.sh](../../scripts/linux/build.sh)，不能把当前原生 glibc 构建称作 musl 静态制品。

## 依赖与离线使用

构建需要 Rust/Cargo、编译工具及所用原生库，主数值依赖包括 OpenBLAS 与 spglib；spglib 绑定还涉及编译/头文件工具链。Cargo.lock 固定 Rust 依赖，绘图采用 plotters/svg2pdf。运行具体计算仍需配置 QE/VASP/Wannier90 等外部程序；Slurm 提交需相应命令可用，部分输入生成或数据导出使用 VASPKIT，COHP 分析需 LOBSTER 结果。

首次获得缺失依赖或模型通常需要下载，也可从其他机器复制离线包和缓存。依赖齐全后 Cargo --offline 可拒绝网络访问；现成二进制不需要在每次运行时联网安装 OpenBLAS。外部求解器、赝势、运行库和字体等运行资源仍须就绪。MatterSim 单独使用 Python 环境和模型，不是所有 Rust CLI 的前置依赖。

## 从源码到终端命令

[build-native.sh](../../scripts/build-native.sh) 对 `build` 读取本次 Cargo JSON 制品报告，使用 [唯一产品映射](../../scripts/product-layout.sh)，将成功构建的自有 CLI 自动发布到仓库 `bin/<模块>/`。无需额外用户安装；`test`、`check` 和仅构建库不会替换日常程序。交付构建需要 `jq`，Linux 私有运行库打包需要 `patchelf`。

```bash
bash scripts/build-native.sh build --release --workspace --locked
bash scripts/build-native.sh build --release --locked -p epwflow --bins
bash scripts/build-native.sh build --release --locked --manifest-path tools/qf-gen-dyn0/Cargo.toml
```

原始制品、缓存与测试 harness 位于 `bin/.build/<平台>/`，本机私有编译和运行依赖可位于 `bin/.native/<平台>/`。公开模块仍为 epwflow、qeflow、vaspflow、scflow、wannierflow、substitutionflow 和 tools，兼容名使用组内相对链接。独立工具仍单独构建；主 workspace 构建不顺带更新它。显式交叉目标只构建，不替换宿主程序。

发布前完成所选产品全部入口启动验证、目标冲突检查及本机／Slurm 活动引用检查，再逐文件替换；编译和预检失败保留原可用程序。发布锁阻止并发写入，不宣称整组事务。来源与发布后 SHA-256 保存于 `bin/.build/published/<程序>.json`，fresh 增量构建仍会发布。

[install-bin.sh](../../scripts/install-bin.sh) 保留为显式制品发布入口，支持 `--products CSV` 或 `--module FAMILY`，与自动构建复用同一清单；[install-current.sh](../../scripts/install-current.sh) 已退役，明确失败且不创建旧用户安装。兼容入口生成器要求显式目标目录，不再默认写入用户命令目录。

本机依赖迁移、历史二进制归档和旧用户目录删除的实际状态见 [0017 验收](../specs/0017-build-binaries-layout/acceptance.md)。历史脚本与身份记录不自动改写；迁移清单保留原路径及哈希，日常入口不使用历史副本。

### 在任意目录调用程序

在 Bash/Zsh 中加载 [scripts/env.sh](../../scripts/env.sh)，脚本从自身位置定位仓库 `bin/`，将七个公开家族目录加入并导出 `PATH`，保留原有路径，将已有模块路径也移到前面，重复加载保持幂等：

```bash
source "/path/to/dftflow/scripts/env.sh"
command -v epw_phonon qeflow qe_scf
```

把脚本路径替换为实际仓库位置即可；脚本路径含空格也受支持。`PATH` 不递归搜索子目录，不加入缓存和测试目录。需要新交互终端自动加载时，由用户将同一 `source` 行加入 Bash 的 `~/.bashrc` 或 Zsh 的 `~/.zshrc`；Bash 登录 shell 的配置仍需加载相应文件。加载脚本不会编辑这些启动文件。批处理作业应显式加载环境或使用绝对路径。

环境加载回归覆盖 Bash/Zsh、仓库外 56 个真实入口、重复加载、原有路径保留、含空格的仓库位置、缺失 `bin` 时的失败保护以及临时 HOME 中新交互终端加载。原构建布局证据见 [0017 验收](../specs/0017-build-binaries-layout/acceptance.md)，新增清理入口的回归见 [0057 验收](../specs/0057-epw-clear-history/acceptance.md)。

Linux 配置 `DFTFLOW_NATIVE_PREFIX` 时，交付构建自动调用 [package-native-runtime.sh](../../scripts/linux/package-native-runtime.sh)，准备一致的加载器与运行库并用 patchelf 修正制品；已有前缀的库版本冲突会拒绝覆盖。该方式依赖共享可读路径，不代表跨任意 Linux 的通用静态包；不通过修改系统库或全局 LD_LIBRARY_PATH 来切换。

## 已留档的重新编译事实

[0017 验收](../specs/0017-build-binaries-layout/acceptance.md) 记录了 2026-10-23 的本机 Linux 原生交付：新的私有缓存中构建八个主程序和独立 dyn0，自动发布九个产品；workspace 388 项通过、10 项显式真实科学用例跳过，独立工具 4 项通过，Bash/Zsh 的 55 个实际入口回归通过。旧安装依赖迁入仓库，54 份历史产品按原哈希归档，旧用户安装根及其 55 个自有入口已删除；删除后构建、测试和启动不重建旧根。macOS、Docker/musl 及新增计算节点实测未执行，不能由本机证据替代。

[0006 验收记录末节](../specs/0006-qe-phonon-submit-chain/acceptance.md) 记载了用户要求全量重编译后的交付：从空 target 目录构建五份主程序与独立工具，主 workspace 97 项及独立工具 4 项测试通过，51 个主入口核验通过，并完成本机安装切换。真实 QE 初始化也有完成记录。

以上是原先全量重编译的既有证据。后续 0006 的默认 outdir 更新另完成 32 项定向测试、五主程序重新编译安装、51 入口隔离检查和安装后输入生成核验；细节见同一验收文档的 US4 节。历史不同阶段的 52 个入口记录包含当时内嵌 dyn0；不能直接当作当前主安装器数量。各家族功能的实际适用范围与已知限制以对应命令页为准，入口帮助通过不等于所有科学计算流程均完成验收。
