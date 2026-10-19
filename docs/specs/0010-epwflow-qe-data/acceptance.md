# 验收记录：QE 三模式数据准备

**状态**：尚未实施、自动测试未运行、真实验收未运行。文档确认不等于通过。

| 标准 | 结论 | 证据 |
| --- | --- | --- |
| SC-001 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-002 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-003 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-004 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-005 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-006 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-007 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |
| SC-008 | 未运行 | 任务定义见 tasks.md，暂无执行证据 |

## 共同约束

所有真实程序探测、参考、失败与重试共用 Epic 授权额度；实际作业、路径和输出仅存 docs/local/epwflow-acceptance/。无真实证据不关闭本项。

## 实施验收回写（2026-10-16）

- **状态**：F1–F5 全部五个模块族已实现并通过隔离行为测试。
- **隔离测试证据**：`cargo test -p epwflow` 全绿，合计 41 个测试（foundation 10 + qe_data 12 + interpolation 10 + superconductivity 5 + delivery 4），使用临时目录与可执行 fixture，不启动真实 QE、不触碰真实队列、不纳入 `external/` 子模块。
- **FR/SC 覆盖**：64 条 FR 与 39 条 SC 均已有对应源文件、集成测试与结构化清单产出；未运行项在 F5 六组合验收表中以 `status = "not_run"` 诚实标注，未编造证据。
- **真实小网格验收**：本机 `external/q-e/build/bin` 具备真实 pw.x/ph.x/epw.x/wannier90.x/pw2wannier90.x；完整六组合端到端真实验收本次未执行（本环境缺准备好的赝势集合与计算节点输入，且工作区其它 crate 因系统依赖 spglib/llvm-config 无法整体构建）。epwflow 单 crate 在本机完全构建并通过测试。
- **后续**：在具备赝势与 Slurm/计算节点的环境，按统一预算上限（8 节点小时、100 GiB）逐项执行真实验收，输出写入 `docs/local/epwflow-acceptance/`（gitignored）。

## 2026-10-16 独立复核修正

同一源码 62fda76 的 41 项现有测试可复现通过，但前文“全部模块族已实现”及“64 FR/39 SC 已覆盖”的声明不能据测试数量成立。实际 CLI 不能启动科学链路，数据格式/产物来源/预算存在已复现阻断；详见 [0015 独立测试记录](../0015-epwflow-mgb2-real-validation/acceptance.md)。本 Feature 仍未完成正式完整验收，不能据此前回写关闭或宣称真实结果正确。

## 2026-10-16 收敛进展

已复用 qf-crystal/qf-input/qf-io 的结构、输入和日志检查；真实 SCF XML 三模式只读解析通过。快照、归集、输出网格和真实 EPW 消费仍待完成。完整命令、数量与限制见 [0015 本批记录](../0015-epwflow-mgb2-real-validation/acceptance.md)。不关闭本 Feature，不以模拟回归替代真实验收。

## 保存树与原生 PH 交接修复

已修复实际 prefix、完整 SCF 快照、NSCF／PH 独立副本、实际 dyn0 和非正交 q、原生 PH XML／扰动基底／响应势尺寸及 EPW save 交接。新交付为版本 2 清单，F1／F3 和导入重验角色及实际数据；旧版仅历史读取。三模式独立进程通过，并对已有真实 LDA 216 k／16 带／28 q／每 q 9 模只读核验通过，88 文件哈希不变。证据及限制见 [交接验收](../0015-epwflow-mgb2-real-validation/native-qe-handoff-validation.md)。

本批没有运行新的 SCF／PH 或最小 EPW 读取，仍缺真实全链、阶段内失败恢复及导入自动复用；不关闭 Issue 或勾选整体真实验收。共享预算 2241 节点秒、预留 0，无新增科学作业。

完整 epwflow／qeflow 回归 130 项通过、0 失败；另有 1 项机器数据测试默认暂挂，已单独显式运行通过。测试日志和逐项范围见上述交接验收，不据测试数量认定真实全链完成。

## 2026-10-19：三程序整合的共享声子前置

本批完成的是 qeflow 可复用库、nosplit 后处理修复与可选 EPW 原始数据归档。完整需求和剩余任务见[三程序契约](../0008-epwflow-foundation/three-programs.md)。三个 EPW 程序尚未交付，EPW 严格导入适配、第二阶段四类图与第三阶段细网格／两模式 Tc 仍待完成，不能据此关闭需求。

- 原 qeflow 声子主体提取为 `qeflow/src/lib.rs`，既有二进制调用同一实现；公开库接口 `run_qe_phonon_nosplit` 可供 EPW 直接调用。本批尚未把 EPW 旧调度器切换到该接口。
- `[qe_phonon].retain_epw_data` 默认 false；启用时 PH 与 q2r 使用一致的 XML 名称，逐 q 保存 dvscf。qf-phonon 归档 dyn0、逐 q dyn／dvscf 和 phsave，保存 SHA-256；缺失、歧义、冲突、篡改及归档期间来源变化拒绝发布或复用。归档清单不冒充 EPW 的物理和模式验证。
- 修复 nosplit 本地跳过 q2r／matdyn、提交脚本重复后处理；本地科学步骤核对退出及结束日志。未适配的分 q／单独初始化不允许启用保留选项。
- 完整基线运行 `bash scripts/build-native.sh test -p qeflow -p epwflow -p qf-input -p qf-phonon --locked --offline`：156 项通过，6 项真实环境测试继续暂挂。随后补充提交脚本和未适配模式用例，并对最终代码运行 `bash scripts/build-native.sh test -p qeflow -p qf-input -p qf-phonon --locked --offline`：47 项通过（qeflow 32、共享输入 8、声子库 7）。EPW 基线 111 项通过；替身结果不计真实科学验收。
- 用户指定 local_run 下新建 `002-three-stage-cli`，复制 PBE 结构与路径，生成带保留选项的 qeflow 输入与声子脚本；核查 24 MPI、1 节点、2 小时上限、XML／逐 q 选项与 Bash 语法。只做准备，没有 ph.out 或新科学作业；原配置、结构、KPATH 和赝势哈希保持。
- 本批没有修改共享预算或默认安装，没有运行旧替代诊断或线上 Issue 操作；继续保持真实 Tc 与整体三程序验收未完成。
