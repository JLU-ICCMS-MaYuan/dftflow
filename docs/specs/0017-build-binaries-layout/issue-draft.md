# 0017 统一 DFTFlow 编译可执行程序目录

- **类型**：`type:maintenance`
- **状态**：本地需求，用户明确授权合并并实施；线上 Gitee 由用户管理。
- **编号约定**：本地草案编号 0017，沿用仓库现有本地 Issue/Spec 编号。Gitee CLI 与连接器在当前环境不可用。
- **关联 Spec**：[spec.md](spec.md)
- **关联需求**：[0011 固定安装与编译历史整理](../0011-build-history-cleanup/issue-draft.md)

## 原始需求记录

> 我希望你能把所有编译好的可执行程序都放在/data/home/mayuan/code/dftflow/bin里面，并且按照目录分割好，比如 epwflow， qeflow， substitutionflow，vaspflow 等等。不要像 /data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/010-public-pbe/bin-window-contrac这样放在测试目录下面。把这一条新建一个 issue ，spec 完成。

> 仅 DFTFlow 自身产物（推荐）

> 而且对于 bin 目录，我建议export到环境变量里面，这样就可以方便任何位置使用了。

> 继续

> 我不想再保留~/.local/share/dftflow/了，删除他吧。既然已经把/data/home/mayuan/code/dftflow/bin里面的各个目录都加到了环境变量里面，每次编译好之后，这些新的可执行文件就可以使用了。对不对。我记得提过这个需求。你是不是没找到。我建议把它变成一个 issue 和 spec 来做。

> 我建议你把 0054 都放到 0017 里面，然后删除 0054，然后开始落地 0017

## 问题与目标

需求建立时，仓库已有 `scripts/install-bin.sh`，能把扁平构建输入分组复制到仓库 `bin/`；主要 workspace 构建通过 Cargo 配置输出到 `target/`，独立 `qf-gen-dyn0` 说明使用其独立 `target/release/`，测试目录中也存在 EPW 可执行副本。本需求统一构建入口、可交付程序目录和测试脚本引用，避免以测试案例目录保存可交付 DFTFlow 二进制；当前结果见 [验收记录](acceptance.md)。

## 范围

- 以仓库根 `bin/` 为 DFTFlow 本机构建与交付产物根目录。
- 按家族归档公开可交付程序：`epwflow`、`qeflow`、`substitutionflow`、`vaspflow`、`wannierflow`、`scflow`、`tools`。
- 协调 workspace/独立工具构建入口、现有分组安装器、固定安装器和 Linux 发布打包脚本。
- 迁移测试目录中的 DFTFlow 可交付程序，并更新活动脚本与使用文档引用。
- Cargo 内部缓存和测试 harness 可放在 `bin/` 下专用构建区，不要求作为家族交付程序发布。
- 提供将七个产品家族目录加入 `PATH` 的 Bash/Zsh 配置方法，使任意工作目录均可直接调用命令；重复加载不重复添加路径。

## 范围外

- QE、VASP、Wannier90 等外部科学软件可执行程序。
- 科学计算行为、运行数据布局和兼容命令名称的改变。
- 自动修改用户 shell 启动文件；文档提供手动配置方法。
- 将 Rust 测试 harness、临时探针或其他测试中间文件按生产家族归档。
- 线上 Issue 创建、推送及测试数据覆盖。

## 验收标准

1. 支持的 DFTFlow 构建入口将 Cargo 构建缓存限制在仓库根 `bin/` 下，并将最终公开程序稳定地整理到家族目录。
2. 七类目录的程序映射完整；EPW 三个阶段程序同属 `bin/epwflow/`，独立 `qf-gen-dyn0` 位于 `bin/tools/`。
3. `test_auto` 不再保存 DFTFlow 可交付程序副本；活动脚本从规范 `bin/` 位置调用。
4. 兼容入口、自动发布及发行链验证通过；退役旧固定安装目标，命令名保持。
5. 重复构建覆盖稳定目的路径，不产生按算例或时间累积的程序目录。
6. 配置 `PATH` 后，在仓库外通过命令名调用九个程序与兼容入口，`command -v` 指向对应家族目录；重复加载配置不重复添加路径。

## 关系

本需求延续 0011 的固定安装与历史目录约束。现有 `scripts/install-bin.sh` 是按家族落盘的复用实现；新工作聚焦构建器输出、来源契约、测试副本迁移及全链路验证，不重写已经工作的安装分组逻辑。

## Documentation Impact

- [ ] 不影响当前功能文档：不适用；构建与程序路径说明需要同步。
- [x] 更新 `docs/overview/dftflow-rust.md`、`docs/overview/tools/qf-gen-dyn0.md`、`tools/qf-gen-dyn0/README.md`、`docs/manual-testing.md`。
- [x] 新增 `docs/specs/0017-build-binaries-layout/`（本地文档及本机验收证据已补齐；平台缺口另列）。
- [x] 更新根 `README.md` 构建和路径示例。
- [x] 在使用说明中提供 `PATH` 配置、Bash/Zsh 持久化方法及仓库外调用验证（README、`scripts/env.sh`，见 [本批验收](acceptance.md)）。
- [x] 核对 `docs/overview/README.md` 是否需要入口变更；若仅更新已有构建页链接则记录不变理由。

## 历史草案确认

用户答复“确认，按此草案继续”，确认本地 Issue 草案与 Spec 需求基线，包括“仅 DFTFlow 自身产物”的范围、根目录分家族组织、测试 harness 与 Cargo 中间产物的边界，以及不变更安装目的地。此确认授权完成文档 Spec，不授权构建实现、移动测试产物或修改安装状态。

用户随后追加 `PATH` 要求，本轮据其原话同步需求、设计与验证指南。配置示例仅在隔离子 shell 验证，不修改用户实际环境或启动文件。

用户再要求“继续”后，已交付可 source 的 `scripts/env.sh`、README 说明和 Bash/Zsh 真实入口回归。US3 已验收；本地 0017 的构建、打包、历史二进制迁移尚未完成，不关闭整体需求。

## 最新实施范围（2026-10-23）

合并的完整需求与来源见 [Spec](spec.md)、[研究](research.md)、[计划](plan.md) 和 [任务](tasks.md)。本需求统一承接重复草稿全部目标：

- 交付构建成功前自动发布到模块目录，不再手动更新第二份用户安装；按模块只更新本次目标，测试和检查不发布。
- 编译、制品准备和启动预检失败保留原可用产品；必要运行兼容性准备不可省略。
- 迁移旧目录内的实际编译、运行依赖和工具，更新元数据及本机配置，核验活动进程、作业与恢复引用。
- 清理指向旧根的自有入口，迁移与验证通过后完整删除 `~/.local/share/dftflow/`，后续操作不重建它。
- 保留历史身份与数据，历史产品副本迁往仓库私有归档并记录原路径、哈希及归属；不改写冻结回执。
- 更新 README、Overview、模块与工具说明、测试指南及 0011 关系。原有“不改变 current 安装目的地”被用户最新目标取代。

新增验收按 Spec SC-008～014 逐项记录。线上链接核验不由本地文件替代；平台不可执行检查单独列明，不计通过。用户本次直接要求合并并实施，不再次要求同范围授权。

## 本机交付记录（2026-10-23）

合并与实现、旧依赖迁移、历史二进制归档和旧目录完整删除已在本机通过验证，详见 [验收记录](acceptance.md)。实际 workspace 388 通过、10 项真实科学用例跳过；独立工具 4 通过，全部 55 个入口启动回归通过。平台发行检查及线上链接未执行项保持可见，本地记录不代替线上关闭。
