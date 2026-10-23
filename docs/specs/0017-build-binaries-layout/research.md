# 历史研究（2026-10-20）

以下是原设计记录；其中保留 current 安装的内容已被末尾最新实施设计替代。历史记录不作为当前实现方案。

# 构建可执行程序目录调研

## 现状核对

| 触点 | 当前行为 | 设计影响 |
| --- | --- | --- |
| 根 `Cargo.toml` | workspace 含 `qeflow`、`vaspflow`、`wannier90flow`、`substitutionflow`、`scflow`、`epwflow` 与共享 crate | workspace 主程序需统一复制/整理到家族目录 |
| `.cargo/linux-x86_64.toml`、`.cargo/macos-arm64.toml` | Cargo target 配置由 `scripts/build-native.sh` 选择 | 构建入口应覆写/派生统一 target 根，不能绕过平台链接参数 |
| `scripts/build-native.sh` | 默认 `cargo build --release --workspace --locked`；用户参数直接透传 | 统一包装需支持 build/test/check 等参数透传，测试产生的目标也不能写进算例目录 |
| `scripts/install-bin.sh` | 已校验扁平八主程序，可选 dyn0，映射到 `bin/<family>/`，在真实程序旁建立兼容命令软链接并核验入口 | 复用其分组语义；扩展来源支持或由 build wrapper 向其提供 staging 目录 |
| `scripts/install-current.sh` | 接受扁平 SOURCE_BIN 并覆盖 `current/bin` | 保持安装路径；可从家族目录构造受校验 staging，不另造用户安装布局 |
| `scripts/linux/build.sh` | 在容器 `/build` 中执行 musl 构建，再生成 `dist/linux-x86_64-musl/bin` | 发布包仍可扁平，但来源及哈希要关联本次构建快照 |
| `tools/qf-gen-dyn0/Cargo.toml` | 独立 Cargo 工程；README 使用 `target/release` | 独立入口须采用根 `bin` 构建缓存并输出 `bin/tools/qf-gen-dyn0` |
| 根 `.gitignore` | 已忽略 `/bin/` 与 `target/` | 继续忽略所有本地生成制品，不加入可执行文件 |
| `test_auto/MgB2/local_run/010-public-pbe/bin-window-contract/` | 存在 EPW 三程序 ELF 副本 | 需要读引用、作业、哈希后迁出并改用根 `bin/epwflow/` |

## 决策 1：保留 Cargo 缓存与家族交付层分离

**决策**：通过受支持 wrapper 为 Cargo 设置仓库内 `bin/.build` 作为 target 根；构建成功后按唯一清单将公开 CLI 复制/整理到 `bin/<family>/`。缓存及 harness 保留在 `.build`，家族目录只放交付程序。

**理由**：Cargo target layout 按 profile/target/deps 组织，无法直接表达本需求的家族分组；把仓库构建 target 根放在 `bin/.build/` 可避免中间可执行文件散落在 `test_auto`，再由现有 grouped install 逻辑复用分组映射。Linux 容器内部可继续将缓存放在挂载卷 `/build`，只要交付文件进入仓库规范目录。

**备选方案**：仅把最终文件复制到 `bin/` 并继续在仓库 `target/` 编译。拒绝原因：不满足构建产物统一置于根 `bin/` 的用户要求。

**证据**：`scripts/build-native.sh` 当前直接执行 Cargo；README 已描述 `scripts/install-bin.sh` 分组目录；Cargo workspace 与独立工程默认均使用各自 target。

## 决策 2：复用现有家族映射，不重复实现安装入口映射

**决策**：以 `scripts/install-bin.sh` 中已维护的家族映射为规范依据，提取或复用映射逻辑给构建 staging；兼容命令保持同组软链接；固定安装器继续接收经过验证的扁平快照或适配后的明确输入。

**理由**：兼容入口有 54 个命令名但仅八个 workspace 主程序，另有独立 dyn0；复制第二份映射将产生漂移。

**备选方案**：新增 build-only 的并行硬编码映射。拒绝原因：相同二进制的家族归属需要在构建与安装之间保持一致。

## 决策 3：迁移测试二进制前先核验活动引用

**决策**：迁移每个当前测试目录程序前，检查 Slurm 作业脚本、已提交作业/进程、脚本与配置引用、程序哈希和可复建来源；更新引用后才删除/迁出副本。

**理由**：EPW 三个程序已有真实验收目录副本，简单删除可能使正在运行的任务或可复查的历史脚本断链。

**备选方案**：直接删除并在失败时恢复。拒绝原因：路径和作业引用风险可由只读核查避免。

## 决策 4：向 PATH 显式加入家族目录

**决策**：提供可在 Bash/Zsh 中 source 的 `scripts/env.sh`，从自身位置定位仓库 `bin/`，显式加入七个家族的绝对路径，保留原有 `PATH`，已存在的目录不重复添加。唯一实现放在脚本，README 和 [quickstart.md](quickstart.md) 复用同一 source 命令，避免维护重复 export 逻辑。

**理由**：`PATH` 不递归搜索子目录。直接加入根 `bin/` 无法找到家族程序；加入 `.build` 或递归枚举目录会将测试与内部工具混入命令搜索范围。

**备选方案**：新增根 `bin` 扁平软链接层。当前家族目录已有兼容入口，额外维护一层链接对本需求没有必要。

**生效边界**：source 仅导出当前 shell 的 `PATH`。用户可手动将同一 source 行加入 `~/.bashrc` 或 `~/.zshrc`；Bash 登录环境需要其已使用的登录配置加载该行或 `~/.bashrc`。不自动编辑用户启动文件，也不承诺独立调度作业继承交互 shell 环境。缺少 `bin/` 时返回失败且不修改 PATH；直接执行脚本给出 source 提示并退出失败。

**验证**：`scripts/tests/env.sh` 在 Bash/Zsh 中使用 55 个真实入口，验证仓库外调用、重复加载、含空格的仓库位置、缺失目录保护和临时 HOME 新终端加载。完整结果见 [本批验收](acceptance.md)。

## 程序映射基线

| Cargo/工具程序 | 规范交付路径 |
| --- | --- |
| `epw_phonon` | `bin/epwflow/epw_phonon` |
| `epw_wannier` | `bin/epwflow/epw_wannier` |
| `epw_superconductivity` | `bin/epwflow/epw_superconductivity` |
| `qeflow` | `bin/qeflow/qeflow` |
| `substitution` | `bin/substitutionflow/substitution` |
| `vaspflow` | `bin/vaspflow/vaspflow` |
| `wannier90` | `bin/wannierflow/wannier90` |
| `scflow` | `bin/scflow/scflow` |
| `qf-gen-dyn0` | `bin/tools/qf-gen-dyn0` |

## 未决技术验证

- Cargo `--target-dir` 与现有平台 `--config` 组合是否可通过统一脚本及独立工具入口实现，需在实现阶段用空 target 目录实测。
- Linux Docker 构建的 `/build` 卷是否需要保留外部缓存，同时将最终产物同步进仓库 `bin/`；最终公开路径必须符合 Spec，容器中间缓存可继续在 `/build`。
- Windows 未列入当前支持构建平台；不新增平台承诺。

# 2026-10-23：合并的本机依赖调查

## 调查边界

2026-10-23，针对[本次需求](spec.md)读取历史规格、当前代码、本机命令路径和 ELF 信息。仅记录现状与后续设计输入，未移动程序、改变真实终端配置或删除目录。线上 Gitee 由用户管理；未执行线上写操作。

## 已有需求确实存在

[0017](spec.md) 原文明确要求：所有 DFTFlow 自有程序位于仓库 `bin/`，按模块分组，不放在测试目录；后来追加 PATH 环境加载要求。

[0017 技术任务](tasks.md) 中 T016～T018 已完成 PATH 加载及验证。T004 的构建后产品归档、T005 的独立工具适配和其他构建／打包／历史副本迁移任务仍未完成。[当时验收](acceptance.md) 也明确构建仍使用 `target/`。

旧 0017 还保留“不改变 current 安装目的地”的范围；[0011](../0011-build-history-cleanup/issue-draft.md) 的历史方案保留 native 依赖。本次最新用户要求改变这一目标，但不能倒改上述历史证据或假称迁移已经完成。

## 构建、发布和查找目前是分开的

| 项目 | 实际证据 | 结论 |
| --- | --- | --- |
| 原生构建入口 | `scripts/build-native.sh` 最后直接 `exec cargo`，没有成功后的模块发布步骤 | 仅完成编译不会更新日常模块程序 |
| Linux 原始输出 | `.cargo/linux-x86_64.toml` 的 `target-dir = "target/linux-x86_64"` | 新 release 产物位于 `target/linux-x86_64/x86_64-unknown-linux-gnu/release/` |
| 模块发布 | `scripts/install-bin.sh` 从显式来源复制并验证，再更新 `bin/<模块>/` | 已有映射可复用，尚未接入自动构建链 |
| 运行环境 | `scripts/env.sh` 按自身位置将七个模块目录加入 PATH | 只影响实际 source 它的 shell 及其后代，不复制二进制 |
| 旧用户安装 | `scripts/install-current.sh` 默认写入 `~/.local/share/dftflow/current/bin` | 后续需要退役或调整，防止重建旧目录 |
| 运行库打包 | `scripts/linux/package-native-runtime.sh` 复制一组配套库并修改 ELF interpreter 与 RPATH | 发布前仍需完成目标环境需要的兼容准备 |

本次工具进程的 PATH 中没有仓库模块目录，相关项只有重复的 `~/.local/bin`。默认 `epw_phonon` 的软链接实际指向 `~/.local/share/dftflow/current/bin/epw_phonon`。

在不修改真实启动文件的 Bash 子 shell 中 source 仓库 `scripts/env.sh`、切换 `/tmp` 后，三个 EPW 程序、qeflow、qe_scf、vaspflow、wannier90、substitution、scflow、qf-gen-dyn0 均解析到仓库规范模块目录。此结果说明加载脚本有效，不证明所有用户终端已经配置。

检查本机 `.bashrc`、`.bash_profile`、`.profile` 及存在的 Zsh 启动文件，没有找到该脚本或仓库模块路径配置；`.zshrc` 不存在。该观察仅适用于此次读取的文件，不推断其他终端或调度环境。

## 旧目录包含实际依赖

旧根目录的直接子目录为 `current/`、`native/`、`tools/`，并非只有可重复复制的程序安装。

| 子目录 | 普通文件数 | 符号链接数 | 普通文件字节数 |
| --- | ---: | ---: | ---: |
| current | 8 | 0 | 736761016 |
| native | 42 | 4 | 95517574 |
| tools | 1 | 0 | 3649489 |

只读枚举 `~/.local/bin` 发现 55 个直接指向该旧根目录的自有软链接，包含独立工具入口：

```text
~/.local/bin/qf-gen-dyn0
  -> ~/.local/share/dftflow/tools/qf-gen-dyn0/current/bin/qf-gen-dyn0
```

对仓库全部九个产品运行 `patchelf --print-interpreter`，均得到：

```text
/data/home/mayuan/.local/share/dftflow/native/linux-x86_64/lib/ld-linux-x86-64.so.2
```

全部九个产品的 RPATH 也包含同一旧前缀中的 `lib`。因此即使 PATH 已指向仓库，直接删除旧目录仍会破坏仓库程序启动；PATH 和 ELF 依赖是两个不同事实。

忽略的本机 `docs/local/build-linux-x86_64.env` 仍将 `DFTFLOW_NATIVE_PREFIX` 指向旧 `native/linux-x86_64`，`RUSTFLAGS` 也从该变量生成链接和 RPATH 参数。README 的开发库、编译和兼容打包示例同样沿用旧前缀；只迁移 ELF 不更新构建配置会导致后续构建失败或重新引入旧依赖。

## 后续设计需要覆盖的事实

1. 沿用七个公开模块目录与现有产品／兼容入口清单；私有缓存、开发依赖和运行库使用仓库私有区域，目录及生命周期在 Plan 明确。
2. 交付构建必须串联“编译 → 必要的兼容制品准备 → 启动与来源检查 → 更新模块目录”，不能在最后复制未经验证的 Cargo 原件。按模块更新和非交付操作必须区分。
3. 依赖迁移须检查 `.pc`、CMake 配置、符号链接、构建环境和 ELF 路径等实际元数据；搬目录并不保证全部路径自动改变。
4. 删除前要完成活动引用调查。此次仅做需求所需的路径与依赖核验，尚未查询活动进程、Slurm 队列或全部历史工作区，不能宣称删除前置条件已满足。
5. 冻结回执和历史脚本应保持原始身份；对仍需继续的工作区按现有导入／新尝试机制或经设计的迁移方式处理，不以重写旧哈希充当兼容。
6. 迁移验证应覆盖旧路径不可访问时的构建和启动、发布失败保护、完整及模块更新、新终端 PATH、旧入口清理和删除后不重建。必要时使用隔离环境验证清理流程；不提交真实科学作业。

以上调查作为下列实施设计的事实输入；本次用户已明确授权合并并实施。


## 最新实施设计与理由

- 使用 `bin/.build/<平台>/` 保存 Cargo 输出、`bin/.native/<平台>/` 保存私有编译和运行依赖；家族目录仅发布产品和兼容入口。
- 构建 wrapper 读取 Cargo JSON 制品报告，以 `jq` 提取本次可执行产品；只有 build 操作发布。复用分组安装逻辑，统一产品映射和入口清单；在替换前完成全部制品启动验证。
- Linux 原生发布从匹配平台的私有前缀准备加载器和库，修正本次快照的解释器与运行路径；macOS 不应用 ELF 操作。显式交叉目标不当作宿主日常程序发布。
- 源码构建、测试和检查的 Cargo 缓存均归仓库；独立工具经同一 wrapper 的 manifest-path 构建，避免第二套构建逻辑。
- 发布只消费本次 Cargo 报告的目标；部分构建不能从目录中搜出旧程序冒充本次结果。入口映射从唯一清单生成。
- 当前声子作业 1345792 绑定仓库 EPW 程序；编译与隔离测试可进行，但实际发布和删除先核验该作业终态及后续活动作业。冻结脚本与身份记录不修改。
- 旧安装器退役并明确失败提示；不再提供默认写入旧根的路径。Linux 发行包保持扁平格式，由规范制品快照生成并校验。
- 旧依赖元数据中的前缀在迁移副本中更新；原目录先保留用于删除前验证，完成启动、重建和引用核查后删除，不留下同名链接。
