# 0017 构建、环境与旧安装迁移验收

## 2026-10-20：历史 PATH 批次范围

2026-10-20，落实用户 PATH 追加要求后的“继续”：交付 US3 的环境加载脚本、README 使用说明、真实 shell 回归及 Overview 回写。用户通过 source 启用当前 shell，通过手动添加同一 source 行启用后续终端。未自动修改用户启动文件。

## 已实现行为与证据

| 需求 | 实现 | 验证与结果 |
| --- | --- | --- |
| FR-009 / SC-006 | `scripts/env.sh` 根据自身路径定位仓库，显式加入七个家族目录 | Bash/Zsh 各验证 55 个真实命令来源及 `--help`，全部通过 |
| FR-009 / SC-007 | 已存在的家族目录不重复追加，原有 PATH 保留 | 加载两次后 PATH 字符串一致，原 `/usr/bin:/bin` 保留 |
| FR-010 / SC-007 | README/quickstart 提供当前会话及新终端用法 | 临时 HOME 写入 `.bashrc`/`.zshrc`，新 Bash/Zsh 交互终端自动解析 EPW 程序并帮助启动成功 |
| US3 路径边界 | 不依赖当前工作目录，支持含空格的仓库路径 | 普通仓库和临时含空格仓库各运行 55 入口；两种 shell 均通过 |
| US3 失败保护 | 缺失 bin 时加载失败，直接执行给出 source 提示 | Bash/Zsh 缺目录失败后原 PATH 不变；直接执行返回失败 |

所有回归由 `bash scripts/tests/env.sh` 执行。含空格的临时仓库复用现有家族程序链接，不重新编译或修改程序本身。两种 shell、两种仓库位置共验证 220 次帮助启动；额外验证两个新交互终端。测试使用临时 HOME，不读取或写入真实用户启动文件。

文档检查覆盖本目录 8 篇 Markdown、根 README 与构建 Overview：54 个本地链接、24 段 Bash/Zsh 示例及两个脚本语法检查通过，`git diff --check` 通过。

## 当前边界

- 本批仅交付 PATH 部分；构建脚本仍使用原 `target/`，未完成迁入 `bin/.build/`、发布打包适配及空缓存重编译验收。
- 已有历史 EPW 记录绑定旧程序绝对路径、链接目标及哈希。原生程序副本和历史身份记录保持原样，不能用此次环境加载回归视为已经迁移。
- 根 `bin/` 的 55 个入口为本机当前安装，沿用 0011 分组安装的既有来源；此次未把它们称为最新源码重新构建的制品。
- 不提交科学作业，不修改物理参数或现有计算结果；不切换 `~/.local/bin`，不操作线上 Issue 或推送。
- 实测环境为本机 Linux 的 Bash/Zsh；不以此声称已完成 macOS 或所有集群平台验收。

US3、T016～T018 已完成，其余未勾选任务继续保留；本地 0017 整体不关闭。

## 2026-10-23：合并需求并实施

本节为最新结果，前面的 PATH 批次记录保留为历史。用户明确要求把重复需求全部并入 0017、删除重复目录并实施。本次保留两份完整原话和研究记录，删除重复目录的四篇文档，修复 0011 的关系链接；不操作线上 Gitee 或推送。

### 实现及验证

| 验收项 | 实际结果与证据 |
| --- | --- |
| SC-001/002/008：构建与产品 | 从新的 `bin/.build/linux-x86_64/` 缓存构建 workspace；八个主程序自动发布。独立 manifest 构建 dyn0，自动发布到 `bin/tools/`。九个产品、七个家族、共 55 个入口；无需第二次安装。 |
| SC-005/009：重复及部分构建 | 实际 Cargo 隔离回归覆盖完整、fresh 增量、模块和独立工具构建；路径包含空格。现场 EPW 部分构建只替换三份 EPW 程序的 inode，其余六份不变。源码和发布后指纹分别记录，二者可因运行库打包而不同。 |
| SC-004/009：失败保护 | 编译失败、帮助启动失败、缺失或损坏源、目标目录冲突、源目标重叠均拒绝替换；本机运行中的产品及 Slurm 脚本的直接引用拒绝覆盖；`test`、`check` 和库构建不发布。 |
| SC-006/007/013：终端入口 | Bash/Zsh 各验证 55 个真实入口，覆盖仓库外、普通及含空格路径、重复加载、旧目录优先级、缺失模块和临时 HOME 新终端。保留原有系统 PATH；没有修改真实启动文件。 |
| SC-003/011：历史迁移 | 54 份历史产品迁往 `bin/.history/0017/<原仓库相对路径>`，共 10,636,543,808 字节，迁移后 SHA-256 逐份一致；`test_auto` 九类产品的 ELF 副本复查为零。 |
| SC-010：编译与运行依赖 | 原生依赖迁入 `bin/.native/linux-x86_64/`，更新四份 pkg-config 元数据和忽略的本机环境配置。九个产品的解释器、RPATH、加载器依赖清单均无旧根引用，`--help`、私有加载器 `--verify` 和 `--list` 通过。 |
| SC-010/012：旧目录删除 | 旧路径不可访问时实际重新构建并自动发布八个主程序和 dyn0，通过九程序启动与指纹验证后删除旧内容；仅清理指向旧根的 55 个自有入口。删除后环境、工具测试和再次启动结果见下方现场证据，不以目录移动代替删除。 |
| SC-014：文档 | 同步 README、EPW 与 dyn0 说明、手工测试指南、已有 Overview 构建页及工具页。Overview 根导航已有对应入口，无新增页面，故无需改导航。未更改科学生产代码。 |

发布按产品逐文件替换，预检在替换前完成，不宣称整组文件系统事务。活动检查针对本机进程和可读取 Slurm 提交脚本对规范产品绝对路径的直接引用，不声称能解析任意外部包装脚本。

### 回归命令与结果

```bash
CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh build --release --workspace --locked
CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh build --release --manifest-path tools/qf-gen-dyn0/Cargo.toml --locked
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --workspace --locked --no-fail-fast
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --manifest-path tools/qf-gen-dyn0/Cargo.toml --locked
bash scripts/tests/build-publish.sh
bash scripts/tests/env.sh
bash scripts/tests/install-current.sh
```

- workspace 全量回归退出 0：**388 通过、0 失败、10 ignored**，78 组测试结果；独立 dyn0 **4 通过、0 失败**。
- `build-publish.sh` 使用实际 Cargo 编译的隔离工程通过；不是用伪造 Cargo 输出替代构建验证。
- 分组安装回归使用本次八份实际产品快照及独立 dyn0，通过重复更新、55 入口、失败保护。旧别名迁移回归通过：旧 qeflow 的 dyn0 别名备份移除，独立工具和独立链接保留。
- 旧固定安装入口明确退出失败，临时 HOME 下不创建旧根。
- 15 份修改或新增的 Bash 脚本及 Linux 打包脚本中两段嵌入式容器脚本均通过 `bash -n`。
- 现场算例使用新 `bin/epwflow/epw_phonon` 执行 `--status --json`，退出 0、stderr 为空；这是只读状态验证，不是科学计算验收。

### 十项 ignored 的具体原因

这些是既有显式真实科学用例，不计通过，不因本次构建改造自动启动。

| 用例 | 未执行原因 |
| --- | --- |
| `real_bound_native_decay_files_render_without_scientific_calls` | 需显式指定真实粗网格回执及全新派生目录，只读科学数据。 |
| `real_workspace_wdata_readonly` | 需显式提供已有 MgB₂ 配置，只读检查 wdata。 |
| `actual_lda_coarse_restart_and_wannier` | 依赖本机忽略的真实 LDA 粗网格，须显式指定只读核验。 |
| `actual_lda_native_layout` | 需指定已有 LDA 证据的 `EPWFLOW_REAL_QE_ROOT`。 |
| `existing_real_qe_data_plot_without_scientific_execution` | 需显式指定已有真实工作区和全新图像目录。 |
| `actual_lda_fine_matrices` | 真实 40³/20³ EPC，需显式配置、原共享预算预留及 Slurm 执行。 |
| `actual_lda_fine_readback` | 需显式指定已经终止的真实 EPC 证据，只读重验。 |
| `actual_lda_native_nonlinear_diagnostic` | 固定非线性诊断须有对应确认、历史次数及原共享预算。 |
| `actual_lda_native_tc` | 真实两模式 Tc，需冻结来源、原共享预算及显式 Slurm 执行。 |
| `numbered_real_legacy_pw_is_read_only` | 需显式设置本机 MgB₂ 配置，只读验证。 |

### 现场证据与保留边界

原始日志和机器路径清单位于忽略的 `docs/local/0017-bin-runtime-migration/`，不提交二进制或运行数据。主要文件为：

- `workspace-build-final.log`、`dyn0-build.log`、`workspace-tests.log`、`dyn0-tests.log`、`test-summary.json`。
- `build-publish-tests.log`、`install-tests.log`、`install-compat-tests.log`、`env-tests.log`。
- `dependency-migration.json`、`published-before-cleanup.json`、`published-after-cleanup.json`、`partial-build-verification.json`、`final-source-verification.json`；九产品的源指纹匹配当前原始制品，公开路径集合在重复构建前后保持一致。
- `test-binaries-before.json`、`test-binaries-migrated.json`、`test-nonproducts-before.json`；历史迁移清单另存 `bin/.history/0017/migration.json`。
- `post-cleanup-build.log`、`post-cleanup-dyn0-build.log`、`post-cleanup-env-tests.log`、`post-cleanup-dyn0-tests.log`、`post-delete-dyn0-build.log`、`removed-legacy-entries.json`、`cleanup-complete.json`。完整删除旧内容后再次实际构建并自动发布 dyn0，退出 0，旧根仍未重建。

迁移时额外核对的 22,057 份非产品文件大小与修改时间保持一致；这里不把元数据核对称为对全部科学数据重新计算 SHA-256。冻结脚本、科学输出、回执及原绝对路径身份没有改写。历史产品归档用于查证原哈希，不承诺在删除旧运行库后能直接执行旧副本；既有续算仍遵守已有身份校验或新尝试契约。

初查时作业 1345792 直接引用仓库 EPW 程序，未在其使用期间发布；等待其终态后重新查询队列为空，并在删除前检查本用户进程及映射无旧根依赖。没有取消作业、提交科学作业或消耗新科学验收预算。

### 平台缺口与用户操作

- **Linux x86_64 原生**：已实际构建、自动发布、全量测试与旧安装迁移。
- **macOS arm64**：配置和共用脚本已适配；本机没有 macOS 环境，未实际构建或执行，不计通过。
- **Docker/musl**：发行来源改为仓库私有快照，发布包同时包含共享入口清单；本机没有 Docker 命令，未执行容器构建、musl 实际测试或断网验证。仅完成外层及嵌入脚本语法核验，T015/CHK013 的平台验收保持未完成。
- **计算节点**：保留原私有加载器打包机制，完成本机 ELF 和依赖验证；没有新增节点启动或科学作业，不能声称完成新的计算节点实测。
- **线上状态**：用户管理线上 Issue；本地合并、提交不表示线上创建、关系更新或关闭，CHK011 保留未核验。

当前终端由用户执行以下命令后使用仓库产品；后续构建使用上方 wrapper 即自动更新公开程序：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
hash -r
command -v epw_phonon qeflow qf-gen-dyn0
```

新终端持久化仍由用户把同一 source 行加入相应启动文件。本机实施与验证完成不代替上述平台验收或线上关闭。
