# 快速验证：Wannier 化与 EPW 插值

当前是实施蓝图，epwflow 尚未实现，以下不是已运行证据。完成本项后执行并将结果写入 [验收记录](acceptance.md)。

## 前提

Rust/Cargo 和 workspace 依赖就绪；使用 contracts 的有效 input.toml，结构、赝势和目标程序真实存在。生产提交前完成 F1 版本证据与唯一预算账本登记；运行资源内 run 必须登记当前 allocation。真实路径保存在忽略目录，不将本文占位示例当成实际机器路径。

## 命令与预期

```bash
cargo test -p epwflow --test interpolation
epwflow check -c input.toml --json
epwflow prepare -c input.toml
epwflow submit -c input.toml --until interpolation
epwflow status -w run --refresh --json
```

自动测试须使用隔离临时目录和可执行 fixture，不占用真实队列。真实 submit 只能在自动验证通过后、预算预留内执行。状态可区分已提交/未知/失败/成功，不能根据命令退出零推断科学成功。

## 失败与恢复核验

分别触发 Spec 所列缺文件、错误版本、损坏/篡改、非有限值、预算不足和未知作业；应报告具体字段/阶段并阻断依赖。不改物理参数，修复外部条件后仅用显式 resume 续算，有效完成项执行次数不增加。再次运行与从其他目录启动必须读取相同输入绑定。

## 验收输出

按 tasks 覆盖全部 FR/SC，实际证据存 docs/local/epwflow-acceptance/；先记录未运行，不能通过脚本存在或帮助输出完成数值验收。共享预算耗尽时停止追加并保留现场。
