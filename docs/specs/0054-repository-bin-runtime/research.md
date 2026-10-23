# 0054：已有需求与本机依赖调查

## 调查边界

2026-10-23，针对[本次需求](spec.md)读取历史规格、当前代码、本机命令路径和 ELF 信息。仅记录现状与后续设计输入，未移动程序、改变真实终端配置或删除目录。线上 Gitee 由用户管理；未执行线上写操作。

## 已有需求确实存在

[0017](../0017-build-binaries-layout/spec.md) 原文明确要求：所有 DFTFlow 自有程序位于仓库 `bin/`，按模块分组，不放在测试目录；后来追加 PATH 环境加载要求。

[0017 技术任务](../0017-build-binaries-layout/tasks.md) 中 T016～T018 已完成 PATH 加载及验证。T004 的构建后产品归档、T005 的独立工具适配和其他构建／打包／历史副本迁移任务仍未完成。[当时验收](../0017-build-binaries-layout/acceptance.md) 也明确构建仍使用 `target/`。

旧 0017 还保留“不改变 current 安装目的地”的范围；[0011](../0011-build-history-cleanup/issue-draft.md) 的历史方案保留 native 依赖。0054 的最新用户要求改变这一目标，但不能倒改上述历史证据或假称迁移已经完成。

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

以上是方案输入，尚未创建正式 Plan／Tasks，也未将建议写为已经获批的实现契约。
