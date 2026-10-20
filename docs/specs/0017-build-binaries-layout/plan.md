# 实施计划：统一 DFTFlow 构建可执行程序目录

**Issue**：[本地 0017 草案](issue-draft.md)

**日期**：2026-10-20

**Spec**：[spec.md](spec.md)

## 摘要

让受支持的 DFTFlow 构建入口使用仓库根 `bin/` 作为产物根：Cargo target/cache 位于专用 `.build/`，最终公开程序由稳定清单整理到功能家族目录。复用现有 `install-bin.sh` 分组映射，调整固定安装和 Linux 打包的来源适配，迁移 EPW 测试目录中的三份程序并更新活动引用。安装后的 `current/bin` 与命令入口不变。

## 技术上下文

追加 `PATH` 使用设计：将七个产品家族的绝对路径加入命令搜索路径，提供保留原有路径、可重复加载的 Bash/Zsh 示例。使用说明覆盖当前会话、新交互终端、同名命令定位及批处理环境边界；不修改用户启动文件。

- **语言与工具**：Rust 2021、Cargo workspace、Bash；Linux musl 构建使用 Docker。
- **主要入口**：`scripts/build-native.sh`、`scripts/linux/build.sh`、独立工具 `tools/qf-gen-dyn0`。
- **测试**：`cargo test --workspace`、EPW 契约回归、`scripts/tests/*.sh`、帮助启动、Linux 静态包离线验证。
- **平台**：当前原生 Linux x86_64 与 macOS arm64；Linux musl x86_64 发行包。
- **数据存储**：无运行时状态或持久数据模型；构建缓存和二进制均为本地忽略产物。
- **约束**：保留平台 Cargo 配置、安装器固定目的地、兼容入口和 Linux 打包结果。
- **规模**：9 个 DFTFlow 可交付程序（workspace 八个，独立工具一个），七个产品家族目录。

## 质量门

| 约束来源 | 强制要求 | 设计满足方式 | 状态 |
| --- | --- | --- | --- |
| 用户确认的 Spec | 最终程序位于仓库根 `bin/` 家族目录 | 输出清单和路径集合验证 | 待实现 |
| 用户确认的 Spec | 不在测试目录存放 DFTFlow 产品程序 | 引用扫描、文件类型盘点与测试脚本运行 | 待实现 |
| 0011 当前安装约束 | 覆盖固定 `current/bin`，不累积历史版本 | 适配输入源，不改变安装目的地 | 待实现 |
| `.gitignore` | 本机产物不进入 Git | 保留 `/bin/` 忽略并检查状态 | 待实现 |
| 现有 grouped installer | 54 个兼容入口和家族映射稳定 | 复用已有映射和安装回归 | 待实现 |
| 用户追加 PATH 要求 | 任意目录按命令名调用，重复加载不重复追加 | 七家族绝对路径配置、仓库外命令解析与帮助启动 | Bash/Zsh 示例已验证；README 集成待完成 |

## 源代码结构

```text
scripts/build-native.sh                 # 统一根目录 target/cache 与归档
scripts/install-bin.sh                  # 现有家族分组及核验
scripts/install-current.sh              # 固定安装来源适配
scripts/linux/build.sh                  # musl 构建与发行包来源
tools/qf-gen-dyn0/README.md             # 独立工具构建示例
README.md                               # 构建/产物目录说明
docs/overview/dftflow-rust.md           # 构建和安装说明
docs/overview/tools/qf-gen-dyn0.md      # 独立工具说明
docs/manual-testing.md                  # 测试命令和产物路径
test_auto/MgB2/local_run/010-public-pbe # 现存 EPW 副本及引用，先核验后迁移
```

**结构选择**：构建器拥有缓存位置与产物归档，`install-bin.sh` 继续拥有家族归属与入口映射，固定安装/发行打包只负责从受验证来源消费文件。不得把测试算例管理器变成二进制存储模块。

## 需求到设计映射

| 来源 | 设计组件 | 验证方式 |
| --- | --- | --- |
| FR-001 / SC-002 | `build-native.sh` target-dir | 删除旧 target 后检查缓存与输出路径 |
| FR-002/003 / SC-001 | 构建 staging + `install-bin.sh` | 九程序清单及 `--help` |
| FR-004 / SC-004 | current 安装与 Linux packaging | 隔离安装测试、musl 包校验 |
| FR-005/006 / SC-003 | EPW 测试脚本及运行目录迁移 | `rg` 引用扫描、ELF 文件清点、脚本回归 |
| FR-007/008 / SC-005 | `.gitignore` 与稳定路径写入 | `git status`、二次构建前后路径/哈希 |
| FR-009/010 / SC-006/007 | `quickstart.md` 配置示例与 README 使用说明 | 隔离 Bash/Zsh 重复加载、仓库外 `command -v` 和 `--help` |

## 阶段与依赖

1. 基础：验证所有构建入口和引用清单；定义唯一产品清单及 staging 契约。
2. US1：实现 workspace/独立工具统一缓存根与产品归档，适配安装和 Linux 打包。
3. US2：核验作业与引用，迁出测试目录程序，更新脚本与说明。
4. US3：提供手动 PATH 配置、持久化指引和仓库外命令验证。
5. 完善：空缓存双构建、完整测试、帮助启动、Linux 发布与文档检查。

## 复杂度说明

Cargo 的编译缓存结构与产品家族结构不同，因此保留构建专用缓存子目录并在成功后归档。直接复制所有 `target/release` 内容会混入依赖和测试 harness，无法构成稳定产品清单。
