# 0015 验收与问题记录

**状态**：初始检查与首批失败回归已执行；生产修复及真实验收尚未开始；Issue/Spec 草案待确认。

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

## 首批契约回归（2026-10-16）

用户“开始”后再次快进拉取，远端没有新实现提交；受测生产源码仍为 62fda76，工作树基点为 4640151。本轮新增测试，不修改生产实现。

- 默认 `cargo test -p epwflow --locked --offline`：原 41 项与新增 1 项 stdout 对照通过，8 项已知失败回归明确 ignored，不计入通过。
- 显式运行 `cargo test -p epwflow --locked --offline --test real_contracts -- --ignored --test-threads=1 --nocapture`：退出 101，0 通过、8 失败，复现结果如下。失败是待修复缺陷，不能据此声称验收通过。

| 用例 | 实际失败证据 | 对应缺陷 |
| --- | --- | --- |
| b01_run_and_submit_validate_missing_configuration | run 未加载实际配置就返回 not_implemented | B01 |
| b01_until_is_a_recognized_execution_boundary | --until 被参数解析拒绝 | B01 |
| b04_import_rejects_empty_manifest_despite_valid_label | 无任何数据、检查或角色却返回 imported | B04 |
| b05_scf_consumes_stdout_and_rejects_nonconvergence | 替身只写 stdout，阶段报缺少 scf.out，未进入收敛判断 | B05 |
| b06_unknown_scheduler_preserves_active_allocation | sacct/squeue 替身不可查询时仍清空 allocations | B06 |
| b07_qe_stages_use_bound_prefix | 配置 MgB2，SCF 实际写入 scf | B07 |
| b08_interpolation_rejects_empty_f2_manifest | 无电子态及 PH 的 F2 清单仍可绑定 F3 | B08 |
| b09_reads_qe_schema_text_coordinates_and_sibling_eigenvalues | 原生 schema 层级样本报缺少 kpoint_coord | B09 |

XML 样本根据已盘点的 QE 7.4.1 层级构造，数值为合成值，只验证格式，不是新的 MgB₂ 物理参考。预算用例通过独立 CLI 进程、隔离状态目录和不可用调度替身执行，未读写正式账本。原始输出、受测提交与测试文件哈希仅留在 `docs/local/0015-epwflow-smoke/real-contracts-known-failures.{json,log}`。

已知失败用例用 `#[ignore = "Bxx: ..."]` 明确暂挂，修复后应显式运行并取消对应 ignore；默认绿灯不能关闭缺陷。B02/B03 等完整科学来源回归仍待补齐，SC-001～004 均未因此转为通过。未运行真实 QE/EPW 或提交 Slurm，原有三套声子结果保持不动。

## 第二批回归与程序路径澄清（2026-10-16）

远端仍无新的生产修复。本轮只读核对用户后续指定的 QE 7.4.1 配套 pw.x、ph.x、epw.x：文件均存在并有执行权限。原 `.my_scriptrc.py` 仍声明旧 7.1 路径，本地算例环境已指定 7.4.1；遵循用户后续选择，不修改该配置。路径与文件检查结果保存在忽略的 `program-path-check.json`。没有执行科学程序，此检查不能替代计算节点启动、动态库或运行版本验证。

当前阻断出现在 epwflow 内部，不能归因于用户没有提供路径：公开 submit/run 在读取配置、调用外部程序前直接返回 not_implemented。即便绕过入口调用当前库函数，也存在下列可执行复现：

| 新用例 | 实际结果 | 缺陷 |
| --- | --- | --- |
| b02_rejects_interpolation_without_native_spectrum_or_quality_evidence | 无合法 F2、无原生谱/矩阵元/质量参考的简化替身仍得到 valid 插值清单，且 quality 为 None；α²F 由生产公式生成 | B02/B08 |
| b03_rejects_superconductivity_without_successful_native_solver | 配置了必定失败的 EPW 替身，生产代码未调用原生求解，仍返回 located/converged/valid 的 Tc 和能隙 | B03 |
| b08_wannier_final_state_does_not_override_nonconvergence | 合成日志明确 Convergence not reached，却因包含 Final State 和有限 Spread 返回 converged=true | B08 的数值质量门补充 |

新增样本均为检测错误放行的合成数据，绝不是 MgB₂ 结果或独立物理参考。插值用例在独立子进程使用隔离预算注册表，不修改正式账本。

当前 `real_contracts` 默认运行 1 通过、11 ignored；显式运行 ignored 用例退出 101，11 项已知失败全部复现。首批 8 项与本批 3 项分别保留记录；最新输出及测试哈希为本地 `real-contracts-second-batch.{json,log}`。ignored 不计通过，修复后取消对应标记。B01～B09 现均有相关行为回归，但树复制、指纹篡改、完整网格与未知状态恢复等覆盖尚未完成，T003 继续未完成。没有真实作业、原始数据或安装变更。
