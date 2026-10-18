# epwflow 当前入口与验证边界

## 当前结论

截至 2026-10-19，epwflow 已复用 qeflow 的 QE 基础库，并能只读检查真实 EPW 5.9 谱和独立插值参考。公开 `submit/run/resume` 仍未接通科学流程，原生 Eliashberg/Tc 尚未交付。MgB₂ 讲义参数的独立声子质量已实测失败，不能把原生冒烟或解析回归当作完整验收。

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
| `epwflow interpolation check -w W [--json]` | 原生目录检查退出、结束计时、Wannier 和谱；有 quality-reference.json 时核对来源及电子/声子误差，超差非零退出；没有该参考则仅报告 native_epc_only |
| `epwflow interpolation import -c C --source M` | 读取自身插值清单；完整实际文件角色和重读能力仍待实现 |
| `epwflow superconductivity check/import` | 读取自身求解清单；清单中的数值来源受下述限制 |
| `epwflow interpolation export-comparison --source S --output D` | 只读导出已有 MgB₂ 原生路径／均匀网格与总 DOS；不提交科学计算；严格投影尚缺 |
| `epwflow collect -w W [--output R]` | 校验并收集 interpolation/comparison，不要求 F4；当前仅交付独立比较包 |
| `epwflow plot --results R [--output D] [--formats png,svg,pdf]` | 四张独立叠图及 2×2 总览，QE 实线先画、EPW 虚线后画；缺投影明示，默认三格式 |
| `epwflow results check --results R [--json]` | 只读检查比较表及默认图目录清单／哈希；操作成功与完整性／质量状态分开 |
| `epwflow delivery collect/results/budget` | 从已有结构化结果收集、读取及显示；操作成功不等于科学结果正确 |

先按本机平台构建说明加载 native 环境和配置，再核查受测入口。下面以构建目录变量为例，实际路径不能用默认安装路径替代：

```bash
"$DFTFLOW_BUILD_BIN/epwflow" --help
"$DFTFLOW_BUILD_BIN/epwflow" qe-data --help
"$DFTFLOW_BUILD_BIN/epwflow" interpolation --help
"$DFTFLOW_BUILD_BIN/epwflow" interpolation check -w "$NATIVE_RESULT" --json
```

当前没有经真实验收的“一次提交续算 EPW”使用命令。安装脚本已列入六程序/52 入口；这不证明当前机器默认入口已安装受测制品，本次未切换安装。

## 数据与科学结果的限制

- **QE 解析**：已按 QE 原生 XML 读取 k_point 文本和相邻 eigenvalues，核对带数、点数与有限性，将 Hartree 能量转换为 eV。对三套已有 MgB₂ SCF XML 的只读解析均通过（8 个 k 点、12 带）；这不证明它们是 EPW 所需完整 NSCF 网格。
- **数据衔接**：输入生成器已统一使用配置 prefix；SCF 已消费捕获的 stdout 并调用共享完成检查。下游仍有硬编码保存文件路径、PH 快照与 EPW 日志衔接缺口，不能据此声称整个 B07 或数据链完成。
- **原生输入**：区分 NSCF/PH 粗网格与 nkf/nqf 细网格，正确处理 meV/eV、冻结窗单独上界、Fermi 能窗换算和短 dvscf_dir；全 q 诊断写显式选择文件。已按 Mori MgB₂ 讲义参数验证渲染，不表示旧 Wannier/coarse/fine 编排已全部替换。
- **α²F**：F3 调度代码已经移除公式造谱，使用原生谱表的能量轴和 α²F 列，保留 λ_integral 与 λ_sum；兼容目标版本两种谱列布局。缺少质量参考或超差会阻断，不能跳过。原生矩阵元、完整首次/重读及自动调度仍未完成。
- **质量**：真实 QE 电子参考与 EPW 能量做有序一一匹配，不删能窗内参考态或拟合平移；声子保留虚频符号、比较全部 3N 模。质量来源清单重新核对指纹，输出超差报告。点的独立性、冻结计划及物理来源仍须验收核查，单份自述清单不是自动认证。
- **超导**：F4 已关闭经验 Tc/BCS 的调度发布路径。原生两模式执行未接通时明确报错，不创建有效能隙/Tc 清单。旧数学辅助函数的测试不能证明 Eliashberg 求解；当前没有通过验收的原生能隙/Tc 入口。
- **验收阈值**：此前本页列出的 Tc 0.1 K、能隙 0.05 meV、λ 相对差 0.02 不是已确认的完整验收标准。正式标准分别在 [0010](../../specs/0010-epwflow-qe-data/spec.md)、[0012](../../specs/0012-epwflow-interpolation/spec.md)、[0013](../../specs/0013-epwflow-superconductivity-tc/spec.md)，当前实现尚未据此完成真实对照。

## 已有证据

2026-10-18 epwflow 与 qeflow 回归合计 95 通过、2 项 B01 明确暂挂；新原生格式/输入/只读质量用例 12 项通过。B02/B03 已验证拒绝无原生证据的错误科学交付，不代表对应科学执行能力已经实现。模拟和旧数学辅助测试均不能替代真实数值对照。

本地已有 MgB₂ 三种模式的 2×2×1 声子结果，各含 2 个不可约 q 点。2026-10-16 已在 `test_auto/MgB2/epwflow/` 下完成三种来源的原生 EPW 5.9 冒烟计算：Wannier 收敛、粗/细网格电子声子计算、500 点 α²F 及积分检查通过。这是独立测试脚本调用科学程序的证据，尚未证明公开 epwflow 入口能完成同样流程。

三套旧输入直接求和 λ 约为 0.651；index 复用同一 SCF 快照重算 PH 后谱形诊断显著改善，但轴仍不同，严格同轴对照未通过。模式按实际输入核对，不能只根据历史目录名称判断。

按用户指定 Mori 讲义新建的 LDA 输入实际完成 SCF、3³ PH 六点、6³ NSCF、原生 Wannier/粗网格及独立六 k/四 q 参考。当前采样的电子误差最大/RMS 为 0.013028/0.008787 eV；声子为 48.530593/10.610753 cm⁻¹，未通过 5/2 标准。相同 ASR 下粗网格点误差小于 0.001 cm⁻¹，主要差异在网格外。电子采样窗内态覆盖有限，不代表整个费米面已验证。

用户批准的单次 6³ 粗 q 对照已于 2026-10-19 正常完成 28 个不可约 PH 点与原生 Wannier/粗网格 EPC，复用同一 LDA SCF/NSCF，其他物理参数不变。相同四点参考的声子最大/RMS 降至 9.042613/1.840342 cm⁻¹；最大误差仍超过 5，实际 Rust 检查输出 quality_failed、退出 1。剩余最大误差在网格外 (1/12,0,0)，未追加扫描或越过质量门。固定四点在新网格下仅余一个网格外点，完整独立覆盖仍不足；本次结果不是 F3 质量通过或自动入口成功的证据。

40³/20³ 细网格仅有配置适配，没有越过失败质量门实际执行；原生 Eliashberg、Tc 和六组合仍未完成。详见 [真实测试手册](../../../test_auto/MgB2/epwflow/README.md)、[参数与检查契约](../../specs/0012-epwflow-interpolation/contracts/interface.md) 和 [0015 验收记录](../../specs/0015-epwflow-mgb2-real-validation/acceptance.md)；机器路径与原始日志留在忽略目录，默认安装未切换。

## 源码依据

公开命令见 `epwflow/src/foundation/cli.rs`；恢复与预算见 `snapshot.rs`、`budget.rs`；数据导入与 XML 见 `qe_data/reuse.rs`、`nscf.rs`；原生输入/谱/质量见 `interpolation/{epw_config,native,native_quality}.rs`；阶段边界见两项 scheduler。模块存在不等于完整工作流可用。

## 与 qeflow 的实际复用关系

| 能力 | 共用实现与当前范围 |
| --- | --- |
| 结构解析 | epwflow 调用 qf-crystal 的 POSCAR 解析，不再维护独立解析器 |
| SCF/NSCF 输入 | qf-input 的 PW 渲染器；epwflow 适配严格配置、显式赝势和统一 prefix，原子质量可显式指定，否则使用共享表；EPW 按结构元素顺序使用相同质量 |
| NSCF 网格 | qf-input 的完整均匀 crystal 网格生成器；EPW 5.9 适配设置 SYSTEM.nosym/noinv 为 false，保留晶体对称性元数据，完整性由显式点表和实际输出验证。真实测试检查了 32 点、14 带及归一化权重；生产链完整输出检查仍待接通 |
| PH 输入 | qf-input 的纯 namelist 渲染器，qeflow 原写文件接口也调用它；coords 坐标在 namelist 后，index 指定 start_q/last_q，子任务关闭 only_init |
| 完成检查 | qeflow 0006 的 checked_log 已提取到 qf-io，epwflow SCF 共用；JOB DONE 与数值收敛分别核验 |

快照、分 q 物化、归集和提交链尚未全部提取或接通。EPW 仍须验证响应势、patterns、完整电子态与原生矩阵元；不能用普通声子结果的成功标记跳过这些检查。当前显式赝势卡片要求绝对且无空白的路径，含空白路径的受控文件物化尚待补齐。

固定配置准备已改为 staging 后发布、重复准备不覆盖，执行前验证 input-init 哈希。空校验列表不再被视为有效清单，Wannier 的 Final State 不能覆盖明确未收敛信息。这些是局部防护；完整输入来源、实际矩阵元角色和自动工作流仍有缺口，未通过整体验收。

## 比较诊断图的当前边界

比较数据模型由 F3 拥有，F5 直接消费。原生导出限定 MgB₂／QE 7.4.1／EPW 5.9，解析真实 XML、band.eig、QE matdyn 和 EPW phband.freq，核对坐标、完整均匀网格、单位和指纹。总 DOS 在相同能量／频率轴使用共同高斯核及物理权重，保留虚频；电子图使用绑定的共同 E_F 和有效窗。

独立电子／声子 DOS 图有元素与轨道分面位置，可靠数据缺失时明确标记。当前原生适配没有生成严格原子投影和 EPW 声子本征矢，真实元素／轨道叠图仍未完成；键心 Wannier 函数和声子模式序号不能充当元素。总览的 DOS 面板展示总量，完整投影以独立分面为准。

collect／plot 可在没有 F4 时导出部分诊断；结果包移动后可重绘，不调用科学程序，表格与原始输出内容不变。图像先暂存、登记哈希再发布，未知文件冲突拒绝。质量失败显示 failed、缺项显示 PARTIAL RESULTS，workflow_complete=false；图像与样式回归不改变原科学质量门。完整自动入口、六组合和原生能隙／Tc 未完成。
