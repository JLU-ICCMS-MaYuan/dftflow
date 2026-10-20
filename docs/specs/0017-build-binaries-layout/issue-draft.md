# 0017 统一 DFTFlow 编译可执行程序目录

- **类型**：`type:maintenance`
- **状态**：本地需求草案，已获用户确认；未创建线上 Gitee Issue。
- **编号约定**：本地草案编号 0017，沿用仓库现有本地 Issue/Spec 编号。Gitee CLI 与连接器在当前环境不可用。
- **关联 Spec**：[spec.md](spec.md)
- **关联需求**：[0011 固定安装与编译历史整理](../0011-build-history-cleanup/issue-draft.md)

## 原始需求记录

> 我希望你能把所有编译好的可执行程序都放在/data/home/mayuan/code/dftflow/bin里面，并且按照目录分割好，比如 epwflow， qeflow， substitutionflow，vaspflow 等等。不要像 /data/home/mayuan/code/dftflow/test_auto/MgB2/local_run/010-public-pbe/bin-window-contrac这样放在测试目录下面。把这一条新建一个 issue ，spec 完成。

> 仅 DFTFlow 自身产物（推荐）

## 问题与目标

仓库已有 `scripts/install-bin.sh`，能把扁平构建输入分组复制到仓库 `bin/`；但主要 workspace 构建仍通过 Cargo 配置输出到 `target/`，独立 `qf-gen-dyn0` 说明仍使用其独立 `target/release/`，测试目录中也存在 EPW 可执行副本。统一构建入口、可交付程序目录和测试脚本引用，避免以测试案例目录保存可交付 DFTFlow 二进制。

## 范围

- 以仓库根 `bin/` 为 DFTFlow 本机构建与交付产物根目录。
- 按家族归档公开可交付程序：`epwflow`、`qeflow`、`substitutionflow`、`vaspflow`、`wannierflow`、`scflow`、`tools`。
- 协调 workspace/独立工具构建入口、现有分组安装器、固定安装器和 Linux 发布打包脚本。
- 迁移测试目录中的 DFTFlow 可交付程序，并更新活动脚本与使用文档引用。
- Cargo 内部缓存和测试 harness 可放在 `bin/` 下专用构建区，不要求作为家族交付程序发布。

## 范围外

- QE、VASP、Wannier90 等外部科学软件可执行程序。
- 科学计算行为、运行数据布局、固定用户安装目的地和 PATH 兼容命令的改变。
- 将 Rust 测试 harness、临时探针或其他测试中间文件按生产家族归档。
- 线上 Issue 创建、推送及测试数据覆盖。

## 验收标准

1. 支持的 DFTFlow 构建入口将 Cargo 构建缓存限制在仓库根 `bin/` 下，并将最终公开程序稳定地整理到家族目录。
2. 七类目录的程序映射完整；EPW 三个阶段程序同属 `bin/epwflow/`，独立 `qf-gen-dyn0` 位于 `bin/tools/`。
3. `test_auto` 不再保存 DFTFlow 可交付程序副本；活动脚本从规范 `bin/` 位置调用。
4. 固定安装、兼容入口和 Linux 发布包验证通过，最终安装位置及命令名保持兼容。
5. 重复构建覆盖稳定目的路径，不产生按算例或时间累积的程序目录。

## 关系

本需求延续 0011 的固定安装与历史目录约束。现有 `scripts/install-bin.sh` 是按家族落盘的复用实现；新工作聚焦构建器输出、来源契约、测试副本迁移及全链路验证，不重写已经工作的安装分组逻辑。

## Documentation Impact

- [ ] 不影响当前功能文档：不适用；构建与程序路径说明需要同步。
- [ ] 更新 `docs/overview/dftflow-rust.md`、`docs/overview/tools/qf-gen-dyn0.md`、`tools/qf-gen-dyn0/README.md`、`docs/manual-testing.md`。
- [x] 新增 `docs/specs/0017-build-binaries-layout/`（本地文档已创建，验收证据待实施补充）。
- [ ] 更新根 `README.md` 构建和路径示例。
- [ ] 核对 `docs/overview/README.md` 是否需要入口变更；若仅更新已有构建页链接则记录不变理由。

## 草案确认

用户答复“确认，按此草案继续”，确认本地 Issue 草案与 Spec 需求基线，包括“仅 DFTFlow 自身产物”的范围、根目录分家族组织、测试 harness 与 Cargo 中间产物的边界，以及不变更安装目的地。此确认授权完成文档 Spec，不授权构建实现、移动测试产物或修改安装状态。
