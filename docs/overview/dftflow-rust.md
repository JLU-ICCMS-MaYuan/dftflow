# DFTFlow Rust 架构、构建与安装

[返回功能总览](README.md)

## 当前交付物

主 workspace 生成五个程序：qeflow、vaspflow、scflow、wannier90、substitution。它们通过 argv[0] 识别兼容命令，避免为每个旧 CLI 单独构建二进制。安装器创建 **51 个唯一入口**：48 个旧命令名，加 qeflow/vaspflow/wannier90 三个家族名；scflow、substitution 已计入旧名，不再重复相加。

[独立 qf-gen-dyn0](tools/qf-gen-dyn0.md) 是另一个 Cargo 工程，单独构建与安装，不计入主 workspace 的五个目标或 51 个入口。保留的 Python mattersim_opt 通过 pyproject.toml 提供，也不由 Rust 安装器安装。源码中的语言迁移范围与历史验收见 [0004](../specs/0004-dftflow-rust/spec.md)。

| 程序 | 实现职责 | 用法导航 |
| --- | --- | --- |
| qeflow | QE 输入、状态、依赖链和后处理 | [CLI](qeflow/README.md) |
| vaspflow | VASP/LOBSTER 输入与数据分析 | [CLI](vaspflow/README.md) |
| wannier90 | Wannier 接口、能窗与绘图 | [CLI](wannierflow/README.md) |
| scflow | EPC 数值分析与结果输出 | [CLI](scflow/README.md) |
| substitution | 元素替换、状态与跨 flow 调用 | [CLI](substitutionflow/README.md) |

## 公共模块关系

CLI 读取 [qf-config](../../crates/qf-config/src/lib.rs) 的配置，经 qf-crystal/qf-input/qf-io 处理结构、输入与数据；qf-runner 运行外部命令和作业，qf-state 维护文件状态。声子专用拆分、合并、归档与绘图在 qf-phonon，超导数值在 qf-scflow，VASP 与替换专用逻辑在 df-vasp/df-substitution。

QE 0006 的完整调度在 [phonon_chain.rs](../../qeflow/src/phonon_chain.rs)，单独初始化在 [phonon_init.rs](../../qeflow/src/phonon_init.rs)。两者使用真实 QE only_init 获取列表；不依赖独立 dyn0 算法。工作流状态文件与 Slurm 作业记录共同用于恢复，不需要常驻服务或数据库。

## 按平台构建

在仓库根目录运行（以下为使用说明，本次未执行）：

```bash
bash scripts/build-native.sh
# 已具备全部依赖缓存时可显式离线构建
bash scripts/build-native.sh build --release --workspace --locked --offline
```

[build-native.sh](../../scripts/build-native.sh) 自动选择平台，避免来回修改同一份编译文件：

| 平台 | 配置 | 默认产物目录 |
| --- | --- | --- |
| Linux x86_64 原生 | [.cargo/linux-x86_64.toml](../../.cargo/linux-x86_64.toml) | target/linux-x86_64/x86_64-unknown-linux-gnu/release |
| macOS arm64 原生 | [.cargo/macos-arm64.toml](../../.cargo/macos-arm64.toml) | target/macos-arm64/aarch64-apple-darwin/release |

平台公共配置保持独立；本机依赖路径可由忽略目录中的 `build-<platform>.env` 和 DFTFLOW_NATIVE_PREFIX 提供。musl 另走 [scripts/linux/build.sh](../../scripts/linux/build.sh)，不能把当前原生 glibc 构建称作 musl 静态制品。

## 依赖与离线使用

构建需要 Rust/Cargo、编译工具及所用原生库，主数值依赖包括 OpenBLAS 与 spglib；spglib 绑定还涉及编译/头文件工具链。Cargo.lock 固定 Rust 依赖，绘图采用 plotters/svg2pdf。运行具体计算仍需配置 QE/VASP/Wannier90 等外部程序；Slurm 提交需相应命令可用，部分输入生成或数据导出使用 VASPKIT，COHP 分析需 LOBSTER 结果。

首次获得缺失依赖或模型通常需要下载，也可从其他机器复制离线包和缓存。依赖齐全后 Cargo --offline 可拒绝网络访问；现成二进制不需要在每次运行时联网安装 OpenBLAS。外部求解器、赝势、运行库和字体等运行资源仍须就绪。MatterSim 单独使用 Python 环境和模型，不是所有 Rust CLI 的前置依赖。

## 从源码到终端命令

“日常使用的程序”指终端 PATH 实际找到的可执行文件；“新代码”指工作树中的源码。编辑源码不会自动更新安装目录，必须编译并把入口指向新制品。

```bash
# 查看当前终端选中了什么
command -v qeflow
command -v qe_phonon_split_index
command -v qf-gen-dyn0

# 安装已经构建并保存到稳定版本目录的五个程序
bash scripts/install-compat.sh /path/to/stable-version/bin
```

[install-compat.sh](../../scripts/install-compat.sh) 默认安装到 ~/.local/bin，第二参数可指定其他目录；源与目标目录须不同。它备份旧入口后建立软链接，源版本目录需持续保留。更新主程序时移除原指向 qeflow 的旧 dyn0 别名，但保留已独立安装的 qf-gen-dyn0。这是“主程序升级”和“独立工具升级”的边界。

登录节点与计算节点 glibc 不匹配时，项目提供 [package-native-runtime.sh](../../scripts/linux/package-native-runtime.sh)，打包一致的加载器与运行库并用 patchelf 修正制品。该方式依赖共享可读路径，不代表跨任意 Linux 的通用静态包；不通过修改系统库或全局 LD_LIBRARY_PATH 来切换。

## 已留档的重新编译事实

[0006 验收记录末节](../specs/0006-qe-phonon-submit-chain/acceptance.md) 记载了用户要求全量重编译后的交付：从空 target 目录构建五份主程序与独立工具，主 workspace 97 项及独立工具 4 项测试通过，51 个主入口核验通过，并完成本机安装切换。真实 QE 初始化也有完成记录。

以上是原先全量重编译的既有证据。后续 0006 的默认 outdir 更新另完成 32 项定向测试、五主程序重新编译安装、51 入口隔离检查和安装后输入生成核验；细节见同一验收文档的 US4 节。历史不同阶段的 52 个入口记录包含当时内嵌 dyn0；不能直接当作当前主安装器数量。各家族功能的实际适用范围与已知限制以对应命令页为准，入口帮助通过不等于所有科学计算流程均完成验收。
