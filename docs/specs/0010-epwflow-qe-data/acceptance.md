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
