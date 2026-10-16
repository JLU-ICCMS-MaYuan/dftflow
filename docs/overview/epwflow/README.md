# epwflow 当前入口与验证边界

## 当前结论

截至 2026-10-16，epwflow 已开始复用 qeflow 的 QE 基础库，但公开 `submit/run/resume` 尚未接通科学流程。输入与完成检查的复用已通过定向回归；原生 EPW 科学链路仍未完成真实验收。

本页依据代码、实际 CLI、回归测试和真实 QE XML 只读解析记录当前状态；检查证据见 [0015 验收记录](../../specs/0015-epwflow-mgb2-real-validation/acceptance.md)。

## 公开命令及当前行为

| 命令 | 实际行为与限制 |
| --- | --- |
| `epwflow check -c C [--json]` | 加载并检查配置/文件；运行版本固定报告 unverified，不代表已探测 |
| `epwflow prepare -c C` | 准备输入绑定；不执行科学链路 |
| `epwflow submit -c C`、`run -c C` | 当前无条件返回 not_implemented；没有 `--until` 参数 |
| `epwflow status -w W [--refresh] [--json]` | 读取本地状态；当前分派忽略 refresh，不查询调度器 |
| `epwflow resume -w W -s/-r` | 尚未接通科学续算 |
| `epwflow probe -c C -s/-r` | 提交探测路径尚未接通；不可把帮助信息当作版本证据 |
| `epwflow budget status/reserve/attach/reconcile` | reconcile 读取 sacct 的分配终态与实际用时，未知状态保留预留，忽略 step 重复记录；完整提交账本与并发控制仍未接通 |
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

- **QE 解析**：已按 QE 原生 XML 读取 k_point 文本和相邻 eigenvalues，核对带数、点数与有限性，将 Hartree 能量转换为 eV。对三套已有 MgB₂ SCF XML 的只读解析均通过（8 个 k 点、12 带）；这不证明它们是 EPW 所需完整 NSCF 网格。
- **数据衔接**：输入生成器已统一使用配置 prefix；SCF 已消费捕获的 stdout 并调用共享完成检查。下游仍有硬编码保存文件路径、PH 快照与 EPW 日志衔接缺口，不能据此声称整个 B07 或数据链完成。
- **α²F**：`interpolation/scheduler.rs` 当前从频率公式构造 α²F，未从实际 EPW 谱读取；这些数值不能称为 MgB₂ 的真实电子声子谱。缺少独立质量参考时，当前代码还会跳过该项检查。
- **超导**：`superconductivity` 当前采用经验 Tc、归一化迭代和 BCS 型能隙；未实现原约定的两种原生 EPW Eliashberg 求解。both 使用 iso 的 Tc，不能当作两种独立结果。
- **验收阈值**：此前本页列出的 Tc 0.1 K、能隙 0.05 meV、λ 相对差 0.02 不是已确认的完整验收标准。正式标准分别在 [0010](../../specs/0010-epwflow-qe-data/spec.md)、[0012](../../specs/0012-epwflow-interpolation/spec.md)、[0013](../../specs/0013-epwflow-superconductivity-tc/spec.md)，当前实现尚未据此完成真实对照。

## 已有证据

2026-10-16 对同一源码运行 41 项测试，foundation 10、qe_data 12、interpolation 10、superconductivity 5、delivery 4 全部通过。其模拟程序、简化文件和数学函数测试覆盖不能替代实际 QE/EPW 格式与物理对照。

本地已有 MgB₂ 三种模式的 2×2×1 声子结果，各含 2 个不可约 q 点。2026-10-16 已在 `test_auto/MgB2/epwflow/` 下完成三种来源的原生 EPW 5.9 冒烟计算：Wannier 收敛、粗/细网格电子声子计算、500 点 α²F 及积分检查通过。这是独立测试脚本调用科学程序的证据，尚未证明公开 epwflow 入口能完成同样流程。

三套直接求和 λ 约为 0.651；index 相对 nosplit 仍有谱形差异，能量轴也非完全一致，严格数值对照未通过。未完成独立插值质量参考或原生 Eliashberg 求解。模式按实际输入核对，不能只根据历史目录名称判断。详见 [真实测试手册](../../../test_auto/MgB2/epwflow/README.md) 和 [0015 验收记录](../../specs/0015-epwflow-mgb2-real-validation/acceptance.md)；机器路径与原始日志留在忽略目录。

## 源码依据

公开命令见 `epwflow/src/foundation/cli.rs`；恢复与预算见 `snapshot.rs`、`budget.rs`；数据导入与 XML 见 `qe_data/reuse.rs`、`nscf.rs`；结果来源见 `interpolation/scheduler.rs`、`superconductivity/{scheduler,iso,aniso,gap}.rs`。模块存在不等于完整工作流可用。

## 与 qeflow 的实际复用关系

| 能力 | 共用实现与当前范围 |
| --- | --- |
| 结构解析 | epwflow 调用 qf-crystal 的 POSCAR 解析，不再维护独立解析器 |
| SCF/NSCF 输入 | qf-input 的 PW 渲染器；epwflow 适配严格配置、显式赝势和统一 prefix，质量来自共享原子质量表 |
| NSCF 网格 | qf-input 的完整均匀 crystal 网格生成器；EPW 5.9 适配设置 SYSTEM.nosym/noinv 为 false，保留晶体对称性元数据，完整性由显式点表和实际输出验证。真实测试检查了 32 点、14 带及归一化权重；生产链完整输出检查仍待接通 |
| PH 输入 | qf-input 的纯 namelist 渲染器，qeflow 原写文件接口也调用它；coords 坐标在 namelist 后，index 指定 start_q/last_q，子任务关闭 only_init |
| 完成检查 | qeflow 0006 的 checked_log 已提取到 qf-io，epwflow SCF 共用；JOB DONE 与数值收敛分别核验 |

快照、分 q 物化、归集和提交链尚未全部提取或接通。EPW 仍须验证响应势、patterns、完整电子态与原生矩阵元；不能用普通声子结果的成功标记跳过这些检查。当前显式赝势卡片要求绝对且无空白的路径，含空白路径的受控文件物化尚待补齐。

固定配置准备已改为 staging 后发布、重复准备不覆盖，执行前验证 input-init 哈希。空校验列表不再被视为有效清单，Wannier 的 Final State 不能覆盖明确未收敛信息。以上是局部防护，带有自述 passed 检查的伪数据仍有放行缺陷，原生谱与超导验证尚未完成。
