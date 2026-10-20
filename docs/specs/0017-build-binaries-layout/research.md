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
