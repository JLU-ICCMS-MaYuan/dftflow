# 快速验证：QE 三模式数据准备

> 2026-11-17 最新约定：下文旧预算登记、账本读写和全局两个作业限制已经撤销，相关旧任务不再执行。当前实现与接口以 [0058 追加需求](../0058-epw-remove-resource-limits/spec.md) 为准；历史科学证据保留。

当前是实施蓝图，epwflow 尚未实现，以下不是已运行证据。完成本项后执行并将结果写入 [验收记录](acceptance.md)。

## 前提

Rust/Cargo 和 workspace 依赖就绪；使用 contracts 的有效 input.toml，结构、赝势和目标程序真实存在。生产提交前完成 F1 版本证据与唯一预算账本登记；运行资源内 run 必须登记当前 allocation。真实路径保存在忽略目录，不将本文占位示例当成实际机器路径。

## 命令与预期

```bash
cargo test -p epwflow --test qe_data
epwflow check -c input.toml --json
epwflow prepare -c input.toml
epwflow submit -c input.toml --until qe-data
epwflow status -w run --refresh --json
```

自动测试须使用隔离临时目录和可执行 fixture，不占用真实队列。真实 submit 只能在自动验证通过后、预算预留内执行。状态可区分已提交/未知/失败/成功，不能根据命令退出零推断科学成功。

## 失败与恢复核验

分别触发 Spec 所列缺文件、错误版本、损坏/篡改、非有限值、预算不足和未知作业；应报告具体字段/阶段并阻断依赖。不改物理参数，修复外部条件后仅用显式 resume 续算，有效完成项执行次数不增加。再次运行与从其他目录启动必须读取相同输入绑定。

## 验收输出

按 tasks 覆盖全部 FR/SC，实际证据存 docs/local/epwflow-acceptance/；先记录未运行，不能通过脚本存在或帮助输出完成数值验收。共享预算耗尽时停止追加并保留现场。


## 2026-10-27 外部声子导入

本轮已批准需求、澄清与实施边界见 [完整补充](external-phonon-request.md)。沿用 0010，整合 0048、0060、0061；真实验证及发布证据在本轮完成后追加。


### 原始 save 的单命令接续

先准备当前已完成且可验证的 SCF/NSCF、独立的外部 save 和配置，再使用正式入口：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
epw_phonon --config "epwinput.toml" --import-phonon "external-phonon/save" -s
```

该命令在一个作业内完成原始声子导入、QE 声子谱/DOS/绘图及完成登记，不运行 SCF/NSCF/PH。旧嵌套配置自动受控升级并备份。用 `epw_phonon --config "epwinput.toml" --stage phonon --status` 只读查看完成状态；声子阶段回执、QE manifest 和结果目录须完整。失败用同一导入命令追加 `--resume`；不要删除回执。

完成后执行 `epw_wannier --config "epwinput.toml" --stage wannier -s`。它可补第三阶段电子参考，复用现有 SCF/NSCF/PH；本轮不继续 EPC 或超导。位置和完成判断详见 [SiP₂ 操作说明](../../../examples/SiP2/README.md)。
