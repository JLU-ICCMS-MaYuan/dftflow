# epwflow 当前入口与验证边界

## 当前结论

截至 2026-10-16，源码基线 `62fda76` 已有第六个 Rust 程序 epwflow 及五个模块族，但公开 `submit/run/resume` 尚未接通科学流程。现有 41 项隔离测试通过，不能据此把它作为已验证的真实 EPW 超导工作流使用。

本页依据代码、实际 CLI 和真实 QE XML 解析复现修正此前“全流程已实现”的表述；检查证据见 [0015 验收记录](../../specs/0015-epwflow-mgb2-real-validation/acceptance.md)。

## 公开命令及当前行为

| 命令 | 实际行为与限制 |
| --- | --- |
| `epwflow check -c C [--json]` | 加载并检查配置/文件；运行版本固定报告 unverified，不代表已探测 |
| `epwflow prepare -c C` | 准备输入绑定；不执行科学链路 |
| `epwflow submit -c C`、`run -c C` | 当前无条件返回 not_implemented；没有 `--until` 参数 |
| `epwflow status -w W [--refresh] [--json]` | 读取本地状态；当前分派忽略 refresh，不查询调度器 |
| `epwflow resume -w W -s/-r` | 尚未接通科学续算 |
| `epwflow probe -c C -s/-r` | 提交探测路径尚未接通；不可把帮助信息当作版本证据 |
| `epwflow budget status/reserve/attach/reconcile` | 有本地记账函数；reconcile 当前未查询 Slurm 终态即清空活动分配，不足以证明预算/并发受控 |
| `epwflow qe-data check/import` | 按自身 manifest 检查/导入；不直接识别旧 qeflow 声子目录，保存目录的普通文件复制也有缺陷 |
| `epwflow interpolation check/import` | 读取自身插值清单；不能独立证明真实 EPW 数据有效 |
| `epwflow superconductivity check/import` | 读取自身求解清单；清单中的数值来源受下述限制 |
| `epwflow delivery collect/results/budget` | 从已有结构化结果收集、读取及显示；操作成功不等于科学结果正确 |

可直接核查构建入口：

```bash
cargo test -p epwflow --locked --offline
./target/debug/epwflow --help
./target/debug/epwflow qe-data --help
./target/debug/epwflow interpolation --help
```

当前没有经真实验收的“一次提交续算 EPW”使用命令。安装脚本已列入六程序/52 入口；这不证明当前机器默认入口已安装受测制品，本次未切换安装。

## 数据与科学结果的限制

- **QE 解析**：`qe_data/nscf.rs` 期待 k_point 内嵌 kpoint_coord 和 e；实际 QE 7.4.1 保存 XML 的坐标直接位于 k_point 文本，本征值在相邻 eigenvalues。对三套真实 MgB₂ 保存 XML 调用当前解析器均失败。
- **数据衔接**：生成器固定使用不同的 scf/nscf/ph 前缀；阶段日志写入 `.epwflow/logs/`，部分消费者却要求独立的 `epw.out`。现有测试通过 fixture 自行写这些文件，没有证明真实 stdout 的衔接。
- **α²F**：`interpolation/scheduler.rs` 当前从频率公式构造 α²F，未从实际 EPW 谱读取；这些数值不能称为 MgB₂ 的真实电子声子谱。缺少独立质量参考时，当前代码还会跳过该项检查。
- **超导**：`superconductivity` 当前采用经验 Tc、归一化迭代和 BCS 型能隙；未实现原约定的两种原生 EPW Eliashberg 求解。both 使用 iso 的 Tc，不能当作两种独立结果。
- **验收阈值**：此前本页列出的 Tc 0.1 K、能隙 0.05 meV、λ 相对差 0.02 不是已确认的完整验收标准。正式标准分别在 [0010](../../specs/0010-epwflow-qe-data/spec.md)、[0012](../../specs/0012-epwflow-interpolation/spec.md)、[0013](../../specs/0013-epwflow-superconductivity-tc/spec.md)，当前实现尚未据此完成真实对照。

## 已有证据

2026-10-16 对同一源码运行 41 项测试，foundation 10、qe_data 12、interpolation 10、superconductivity 5、delivery 4 全部通过。其模拟程序、简化文件和数学函数测试覆盖不能替代实际 QE/EPW 格式与物理对照。

本地已有 MgB₂ nosplit/index/coords 三套 2×2×1 声子结果，各含 2 个不可约 q 点及相关保存文件；已做只读盘点，尚未证明当前 epwflow 能复用它们。未提交本次真实 EPW 作业，未取得三模式插值或超导数值验收证据。机器路径与原始日志只保存在本地忽略目录。

## 源码依据

公开命令见 `epwflow/src/foundation/cli.rs`；恢复与预算见 `snapshot.rs`、`budget.rs`；数据导入与 XML 见 `qe_data/reuse.rs`、`nscf.rs`；结果来源见 `interpolation/scheduler.rs`、`superconductivity/{scheduler,iso,aniso,gap}.rs`。模块存在不等于完整工作流可用。
