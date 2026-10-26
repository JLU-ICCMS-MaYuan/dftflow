# 0057 验收记录

## 实现与进程测试

Rust `epw_clear` 仅处理约定三处 history，默认只读预览，`--yes` 删除全部历史内容、保留存在的历史根。独立布局读取不加载科学配置、不写回任务或结算预算。活动／未知任务、现用引用和链接祖先在删除前拒绝；历史内部链接不跟随。删除错误列出具体路径并返回非零。

`CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test --locked -p epwflow --test clear_history` 最终 **15 项通过，0 失败，0 忽略**。日志留于忽略的 `docs/local/0057-history-cleanup/final-clear-tests.log`。

覆盖默认／显式预览、三类全部清空及根 inode 保留、重复执行、缺失根不创建、带空格路径和工作区外调用、链接及祖先、JSON／脚本／相对路径／链接引用、当前尝试快照引用、活动记录链接、活动／未知任务及未登记队列作业、终态 accounting、独立提交回执只读解析、旧快照不覆盖当前状态、非法布局／记录／调度查询、在历史内调用、互斥参数、真实 unlink 权限失败和范围外同名 history 保留。权限失败用例无跳过；以 root 测试时仅将 CLI 子进程降为普通 uid。

构建发布 fixture 回归 `bash scripts/tests/build-publish.sh` 已通过：新增清理入口随 EPW 模块发布，覆盖 fresh 增量、模块隔离、失败保护、活动进程及作业保护、独立工具和带空格路径。shell 语法与任务 diff 空白检查通过。实际安装及真实清理证据见下文。

## 真实现场基线变化

首次只读基线为 1385 个现用文件／链接（包含外部预算）。删除前重验发现另一项工作已更新工作区 README 及部分 `results/` 图形／清单；当时本任务未删除历史。保留首次清单和差异于 `live-before.initial.json`、`predelete-baseline-check.json`，在外部写入结束后另建最终删除前基线，不把这些变化归因于清理，也不声称初始基线完全一致。

## 边界

没有新增真实 QE／EPW 计算或 Slurm 提交。测试及安装验证不替代科学验收；线上 Issue 与双向远程链接继续由用户管理，未声称线上关闭。0056 的扩展测试曾有入口 `(deleted)` 失败，首次批次和隔离复测分别记录于 [0056 验收](../0056-epw-stable-workdir/acceptance.md)。

## 构建与实际入口

以 `f91419b` 为基线导出临时源码副本，仅加入本项 Cargo 注册、模块注册、清理实现／入口／测试；逐字节核对副本中的绘图源码仍与基线一致。经原仓库构建入口执行 `build --release --locked --manifest-path <隔离副本>/Cargo.toml -p epwflow --bins`，四个 EPW 入口构建、打包并自动发布成功。没有切换分支。源码位置与基线记录在本地 `build-source.json`，构建日志为 `release-build.log`。

在仓库外加载 `scripts/env.sh`，实际 `command -v` 返回仓库 `bin/epwflow/` 下的四个程序；`epw_clear --help` 与约定参数一致。四个发布程序的 SHA-256 与 `bin/.build/published/<程序>.json` 一致，本次指纹副本保留于 `published-verified.json`。

`bash scripts/tests/env.sh`：Bash、Zsh 各验证 56 个真实入口，覆盖仓库外、带空格路径、重复加载、原 PATH 保留、临时 HOME 新终端加载，以及缺失 bin 和直接执行保护；退出 0，日志 `env-test.log`。本次未开展 macOS／musl 或新增计算节点验收。

## 真实 MgB₂ 清理

工作区：`examples/MgB2/sugon-amd/run/epwflow`。最终清理前重新建立现用清单：1385 个文件／链接（含外部预算账本），240941615 字节。先以实际已发布 CLI 预览，再执行 `--yes`；两处已有 history 根 inode 保持，缺失根没有创建，无备份或回收站。

| 历史根 | 清理前 | 结果 |
| --- | --- | --- |
| `epw-state/history` | 26 个子目录、95 个文件／链接、14819538 逻辑字节 | 内容全部清空，根保留 |
| `epw-state/native-attempts/history` | 不存在 | 正常报告无需清理，未创建 |
| `epw-state/native-jobs/history` | 23 个文件、97387 逻辑字节 | 内容全部清空，根保留 |

合计删除 26 个子目录、118 个文件／链接、14916925 逻辑字节；硬链接等因素使逻辑字节不能作为实际回收磁盘空间。第二次 `--yes` 正常完成，两处为零，缺失根仍缺失。

清理、只读状态和重复清理后，1385 个现用文件／链接及外部预算逐项一致，112 个现用目录 inode 全部不变。八个阶段 `scf/nscf/electronic/phonon/wannier/coarse/fine/superconductivity` 清理前后均为 `reuse`。`workflow_status=succeeded`、`operation_status=ok` 和既有 `workflow_complete=false` 均保持；父尝试目标为 fine，本项不改变其完成语义，也不据此宣布科学全链验收。

本地证据：`live-before.initial.json`、`predelete-baseline-check.json`、最终 `live-before.json`、`live-after-check.json`、`history-roots-before.json`、`status-before-final.stdout`、`status-after.stdout`、`clear-preview.*`、`clear-delete.*`、`clear-idempotent.*`、`cleanup-result.json` 与 `real-cleanup.log`，均位于忽略的 `docs/local/0057-history-cleanup/`。原始计算与预算未纳入 Git。

## 最终文档与安装门

`bash scripts/tests/install-bin.sh <真实 release 目录> <独立 dyn0>` 完整退出 0。覆盖九个主程序和独立工具的 56 个入口、重复安装、四个 EPW 程序独立安装及其他模块保留、缺失／损坏来源拒绝、目标目录／模块链接冲突和源目标重叠保护；每次失败后均复核原安装可用，日志 `install-test.log`。未用占位程序代替本项真实安装回归；构建 fixture 单独记录。

两项 Issue／Spec、根 README、模块手册及相关 Overview 的 23 份 Markdown 本地链接检查通过；新 Rust 文件格式、修改脚本 Bash 语法和任务 diff 空白检查通过。本地 FR／SC 与技术任务及验收对应完成，远程平台门继续由用户管理。0056 与 0057 分别提交，现有 AGENTS.md 和并行绘图改动按路径及字节隔离。
