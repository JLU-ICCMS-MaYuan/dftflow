# 0015 验收与问题记录

**状态**：初始检查已执行；新测试/修复及真实验收尚未开始；Issue/Spec 草案待确认。

## 已执行检查

- `git pull --ff-only --no-recurse-submodules origin master`：已是最新；HEAD 与 origin/master 均为 `62fda76621c17806f01b5831841fa44ed4239fb0`。
- `cargo test -p epwflow --locked --offline`：41 项通过（10+12+10+5+4）；未启动真实科学计算。
- 实际 CLI：submit/run 退出 1，提示科学阶段未接入；submit --until interpolation 退出 2，参数不存在。帮助检查通过不抵消运行失败。
- 三种来源各有相同的 2×2×1 dyn0、2 个不可约 q 点；SCF 日志显示收敛/JOB DONE，nosplit PH 或分 q 初始化正常结束；响应文件/保存树存在不代表 EPW 已接受。
- source inventory 记录 nosplit 13 项、index 29 项、coords 29 项文件及哈希。真实路径、CLI 输出和 sacct 查询仅保存在忽略的 docs/local/0015-epwflow-smoke/。
- 当前 squeue 查询无活动作业；近两日 sacct 有两项名字含 EPW 的 SiP2 作业，不因名称含 EPW 将其归入本次 MgB₂。未查到本次真实预算账本，历史消耗归属仍须核对，不能假定额度未使用。

## 阻断清单

| 编号/严重度 | 证据 | 影响 |
| --- | --- | --- |
| B01 HIGH | foundation/cli.rs submit/run 无条件 not_implemented，--until 缺失；snapshot.rs resume 未接入 | 一次入口不能运行 |
| B02 CRITICAL | interpolation/scheduler.rs 使用 0.5*(1-exp(-abs(w)/200)) 生成 α²F | 生成谱不来自真实电子声子矩阵元 |
| B03 CRITICAL | superconductivity/iso.rs、aniso.rs、gap.rs 使用经验 Tc/BCS，both 采用 iso Tc | 不是原 Spec 的两种原生 EPW Eliashberg 求解 |
| B04 HIGH | qe_data/reuse.rs 对 snapshot_dir、wavefunctions 用 fs::copy；仅比较复制时来源与副本哈希 | 目录不能导入，也未校验保存指纹；旧 qeflow 没有所需 manifest |
| B05 HIGH | process.rs 写 .epwflow/logs，阶段却要求 epw.out；tests fixture 自行造输出文件 | 真实 stdout 不会自动出现在阶段要求的位置 |
| B06 HIGH | foundation/budget.rs reconcile 不查 sacct，直接清 allocations/lanes | 活动作业/未知状态可被误结算，不能靠其证明资源受控 |
| B07 HIGH | interpolation/epw_config.rs 固定 prefix=scf，写 energy_reference；qe_config.rs SCF/NSCF/PH prefix 分别固定为 scf/nscf/ph | 输入与保存数据不衔接，必须按目标程序核验 |
| B08 HIGH | interpolation/scheduler.rs 无参考时跳过 quality_valid；测试使用空 F2 数据加 valid 标记 | 缺参考和数据仍可能标记可用 |

## 真实格式补充复现

通过临时 Rust 程序直接调用受测 crate 的 `qe_data::nscf::parse_xml`，输入三套来源的实际 SCF 保存 XML（同 QE 7.4.1 schema），均退出 1：`data_invalid: k 点缺少 kpoint_coord 坐标`。这证明当前 XML 格式适配失败；不是已做完 NSCF 的证据。实际坐标在 `<k_point weight="...">x y z</k_point>`，本征值为相邻 `<eigenvalues>`。

新增 B09（HIGH）：`qe_data/nscf.rs` 与真实 QE XML 不匹配，模拟测试嵌套结构掩盖了问题；修复需单位、权重、带数及网格完整性一起核对。对应回归纳入 T003/T004。

## 成功标准

| 标准 | 当前结论 |
| --- | --- |
| SC-001 | 初始审查完成；缺陷未修复，未通过 |
| SC-002 | 真实三模式 EPW 未运行 |
| SC-003 | 独立数值参考未运行 |
| SC-004 | 完整链路与资源恢复未通过 |

问题属于实测前的实现/验证缺口，不是队列尚未运行所能解释。未改变旧输入、运行目录或默认安装。

## 分工确认与实现交接

用户已确认“保持分工：其他模型按缺陷清单补齐实现，你继续测试和 debug（推荐）”。[交接清单](handoff.md) 按 B01～B09 列明修复顺序和逐批复测条件。当前仅文档交接完成，缺陷、三模式实际运行、数值对照均未因此转为通过；没有新增真实作业或修改科学实现。完整草案确认状态独立保留。
