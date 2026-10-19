# 快速验证：配置、环境检查与受控执行基础

配置检查与输入绑定已有实现；完整真实自动链仍未通过验收。下面区分隔离回归与本机命令，验证结果见[验收记录](acceptance.md)。

## 前提

Rust/Cargo 和 workspace 依赖就绪；使用 contracts 的有效配置，结构、赝势和目标程序真实存在。生产提交前完成 F1 版本证据与唯一预算账本登记；运行资源内 run 必须登记当前 allocation。真实路径保存在忽略目录，不将本文占位示例当成实际机器路径。

配置文件名可通过 `-c/--config` 指定。本机 MgB₂ PBE 示例按用户最新选择使用 `inputepw.toml`，保留同目录供 qeflow 使用的 `input.toml`；下面命令采用该名称。隔离回归 fixture 中的 `input.toml` 仍是有效 epwflow 配置，不需要重命名。

## 命令与预期

在仓库根目录执行隔离回归：

```bash
bash "scripts/build-native.sh" test -p epwflow --locked --offline --test foundation
```

在本机 PBE 配置目录、使用受测构建和对应运行库执行：

```bash
epwflow check -c inputepw.toml --json
epwflow prepare -c inputepw.toml
epwflow status -w epwflow --json
```

`prepare` 创建输入绑定，不执行科学计算。提交前先按[模块手册](../../../epwflow/README.md)核查当前账本、EPW 版本及独立参考阻断；以下命令会提交作业，不能作为只读验证直接执行：

```bash
epwflow submit -c inputepw.toml
```

自动测试须使用隔离临时目录和可执行 fixture，不占用真实队列。真实 submit 只能在自动验证通过后、预算预留内执行。状态可区分已提交/未知/失败/成功，不能根据命令退出零推断科学成功。

## 失败与恢复核验

分别触发 Spec 所列缺文件、错误版本、损坏/篡改、非有限值、预算不足和未知作业；应报告具体字段/阶段并阻断依赖。不改物理参数，修复外部条件后仅用显式 resume 续算，有效完成项执行次数不增加。再次运行与从其他目录启动必须读取相同输入绑定。

## 验收输出

按 tasks 覆盖全部 FR/SC，实际证据存 docs/local/epwflow-acceptance/；先记录未运行，不能通过脚本存在或帮助输出完成数值验收。共享预算耗尽时停止追加并保留现场。
