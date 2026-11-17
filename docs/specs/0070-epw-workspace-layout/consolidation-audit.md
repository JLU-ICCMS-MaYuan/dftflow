# 旧目录设计归并候选审计

这是文档清理候选清单，不是已完成的删除记录。所有设计归并说明只在本目录保存；不在旧文档留下跳转链接。

## 审查原则

1. 移入并统一的是 EPW 工作区目录层级、文件放置、数据拥有者和清理设计；不扩大到其它模块或全仓库构建布局。
2. 旧 Spec 的目录树／目录专属条款与非目录科学要求可能混合，必须逐项拆开，不能全文件删除或按“出现路径”批量删除。
3. 普通命令示例、源文件引用和历史实测来源不是自动删除对象；旧原话中的目录设计如需迁入，应连同完整原文迁入本目录，不能只删一句改变原意。
4. 总稿确认后，逐项记录迁入／删除的实际片段和保留的非目录规则；不把本清单当成清理完成。

## 候选位置

- `docs/overview/epwflow/README.md`：第 33 行 编号工作区的原生任务提交；第 93 行 编号工作区与当前配置续算；第 121 行 历史固定工作区与兼容行为；第 146 行 编号工作区的电子后处理续算
- `docs/overview/epwflow/epw_clear.md`：第 1 行 epw_clear：旧历史清理与新布局兼容
- `docs/overview/epwflow/workflow.md`：包含目录树。
- `docs/specs/0008-epwflow-foundation/research.md`：第 105 行 T050：目录锁的释放顺序
- `docs/specs/0008-epwflow-foundation/spec.md`：第 193 行 FR-003：路径与保存目录
- `docs/specs/0008-epwflow-foundation/tasks.md`：第 27 行 阶段 5：US1 — 路径与保存目录
- `docs/specs/0010-epwflow-qe-data/plan.md`：第 90 行 保存树与真实 PH 布局收敛
- `docs/specs/0010-epwflow-qe-data/research.md`：第 67 行 D08：修复主路径与目标格式，而非保留伪布局
- `docs/specs/0010-epwflow-qe-data/spec.md`：第 251 行 FR-009：导入、复用与旧目录
- `docs/specs/0010-epwflow-qe-data/tasks.md`：第 69 行 阶段 11：US4 — 导入、复用与旧目录
- `docs/specs/0012-epwflow-interpolation/acceptance.md`：第 103 行 2026-10-19：实际解纠缠外窗与编号运行目录
- `docs/specs/0012-epwflow-interpolation/research.md`：第 114 行 派生 PH XML 的记录布局与恢复身份；第 119 行 单 q 响应势目录
- `docs/specs/0013-epwflow-superconductivity-tc/tasks.md`：第 175 行 2026-10-21：当前目录 PBE 收敛任务
- `docs/specs/0014-epwflow-delivery/acceptance.md`：第 85 行 2026-10-19：实际解纠缠外窗与编号运行目录
- `docs/specs/0014-epwflow-delivery/spec.md`：第 483 行 2026-10-19：拟合外窗重绘与运行目录整理
- `docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md`：第 13 行 2026-10-21：当前目录 PBE 范围与只读诊断；第 326 行 2026-10-19：#15 外窗与目录子项的正式追踪补齐
- `docs/specs/0015-epwflow-mgb2-real-validation/checklists/requirements.md`：第 13 行 2026-10-21：当前目录 PBE 范围变更复核；第 31 行 实际外窗与编号目录的需求质量检查
- `docs/specs/0015-epwflow-mgb2-real-validation/contracts/verification.md`：第 31 行 FR-012/013、SC-006/007：外窗与目录验收契约
- `docs/specs/0015-epwflow-mgb2-real-validation/current-directory-pbe-tc.md`：第 1 行 当前目录 PBE 超导运行计划
- `docs/specs/0015-epwflow-mgb2-real-validation/handoff.md`：第 129 行 2026-10-19：实际外窗与编号目录的本轮交付；第 133 行 当前外窗与目录交接入口
- `docs/specs/0015-epwflow-mgb2-real-validation/issue-draft.md`：第 86 行 本次补充：实际外窗与编号目录；第 213 行 2026-10-19：自动入口缺口与测试目录整理；第 231 行 2026-10-19：实际外窗与编号目录的本轮交付；第 261 行 2026-10-21：当前目录 PBE 超导验收目标
- `docs/specs/0015-epwflow-mgb2-real-validation/plan.md`：第 102 行 2026-10-19：实际外窗与编号目录的本轮交付；第 106 行 US4：实际外窗与目录子项的实施基线
- `docs/specs/0015-epwflow-mgb2-real-validation/quickstart.md`：第 1 行 当前验证入口：实际外窗与编号目录
- `docs/specs/0015-epwflow-mgb2-real-validation/research.md`：第 107 行 实际外窗与编号目录的决策依据
- `docs/specs/0015-epwflow-mgb2-real-validation/spec.md`：第 234 行 2026-10-19：自动入口缺口与测试目录整理；第 252 行 2026-10-19：实际外窗与编号目录的本轮交付；第 256 行 当前外窗与目录子项的需求结论
- `docs/specs/0015-epwflow-mgb2-real-validation/tasks.md`：第 110 行 2026-10-19：入口与目录清理；第 115 行 2026-10-19：实际外窗与编号目录的本轮交付；第 121 行 外窗与目录子项的文档收敛；第 138 行 2026-10-21：当前目录 PBE 超导任务
- `docs/specs/0015-epwflow-mgb2-real-validation/temperature-points-validation.md`：第 1 行 当前目录 PBE 固定温度点运行验收
- `docs/specs/0015-epwflow-mgb2-real-validation/window-directory-analysis.md`：第 1 行 15 实际外窗与目录子项覆盖分析
- `docs/specs/0015-epwflow-mgb2-real-validation/window-directory-validation.md`：第 1 行 实际拟合外窗重绘与运行目录整理验收；第 18 行 目录与来源保护
- `docs/specs/0018-epwflow-stage-workspaces/approved-plan.md`：包含目录树。第 7 行 新需求：简化 QE／EPW 目录、读取当前配置续算、增加并行检查；第 15 行 2. 目录和计算阶段
- `docs/specs/0018-epwflow-stage-workspaces/issue-draft.md`：第 1 行 0018：简化 QE／EPW 目录、当前配置续算与并行预检
- `docs/specs/0018-epwflow-stage-workspaces/original-request.md`：包含目录树。
- `docs/specs/0018-epwflow-stage-workspaces/plan.md`：第 3 行 执行与目录
- `docs/specs/0018-epwflow-stage-workspaces/spec.md`：第 1 行 0018：QE／EPW 分阶段工作目录与当前配置续算
- `docs/specs/0025-epw-visible-run-state/issue-draft.md`：第 1 行 0025：移除 EPW 隐藏运行状态目录，支持删除阶段目录后重建
- `docs/specs/0025-epw-visible-run-state/plan.md`：第 3 行 目录与职责
- `docs/specs/0028-phonon-postprocess-checkpoint/issue-draft.md`：第 1 行 0028：编号声子目录 dyn0 自复制截断与后处理续算
- `docs/specs/0028-phonon-postprocess-checkpoint/spec.md`：第 1 行 需求规格：编号声子目录 dyn0 自复制截断与后处理续算
- `docs/specs/0049-epw-multitemperature-submit/contracts/config-cli.md`：第 1 行 配置、命令与目录契约
- `docs/specs/0050-epw-stage-results/acceptance.md`：第 87 行 2026-11-16 比较目录扁平化验收；第 105 行 已有 Ubuntu MgB₂ 目录迁移
- `docs/specs/0050-epw-stage-results/checklists/requirements.md`：第 37 行 2026-11-16 目录修订核对
- `docs/specs/0050-epw-stage-results/data-model.md`：第 84 行 版本和旧布局；第 89 行 2026-11-16 目录扁平化修订
- `docs/specs/0050-epw-stage-results/issue-draft.md`：包含目录树。第 83 行 数据复用与目录设计；第 252 行 2026-11-16 比较目录扁平化（当前有效）；第 277 行 2026-11-16 当前目录原位重绘
- `docs/specs/0050-epw-stage-results/plan.md`：包含目录树。第 43 行 拟交付目录；第 143 行 旧工作区兼容；第 166 行 2026-11-16 目录扁平化修订
- `docs/specs/0050-epw-stage-results/quickstart.md`：第 66 行 2026-11-16 扁平目录验证
- `docs/specs/0050-epw-stage-results/research.md`：第 142 行 2026-11-16 目录扁平化修订
- `docs/specs/0050-epw-stage-results/spec.md`：包含目录树。第 83 行 数据复用与目录设计；第 335 行 2026-11-16 比较目录扁平化（当前有效）；第 353 行 2026-11-16 当前目录原位重绘
- `docs/specs/0050-epw-stage-results/tasks.md`：第 11 行 第一阶段：统一数据和布局契约（US2、US4）；第 57 行 2026-11-16 目录扁平化修订
- `docs/specs/0055-electronic-plot-windows-pdos/contracts/electronic-results.md`：第 3 行 目录与文件
- `docs/specs/0056-epw-stable-workdir/issue-draft.md`：第 1 行 0056：EPW 重算归档保留阶段工作目录
- `docs/specs/0056-epw-stable-workdir/spec.md`：第 1 行 0056：EPW 重算归档保留阶段工作目录；第 25 行 US1：在原目录查看重算结果（P1）
- `docs/specs/0064-epw-readable-state-logs/acceptance.md`：第 129 行 SiP₂ 旧能带赝势目录兼容修复（2026-10-28）
- `docs/specs/0064-epw-readable-state-logs/issue-draft.md`：第 1 行 0064：EPW 八阶段可读日志与状态目录简化；第 182 行 原始需求记录追加：SiP₂ 旧赝势目录接续（2026-10-28）；第 190 行 原始需求记录追加：编号配置统一存入状态目录
- `docs/specs/0064-epw-readable-state-logs/research.md`：第 23 行 配置版本目录变更
- `docs/specs/0064-epw-readable-state-logs/spec.md`：包含目录树。第 1 行 0064：EPW 八阶段可读日志与状态目录简化规格；第 244 行 文件布局；第 407 行 原始需求记录追加：SiP₂ 旧赝势目录接续（2026-10-28）；第 417 行 原始需求记录追加：编号配置统一存入状态目录
- `docs/specs/0064-epw-readable-state-logs/tasks.md`：第 33 行 追加 SiP₂ 旧赝势目录兼容修复
- `epwflow/README.md`：包含目录树。第 37 行 编号工作区：原生任务与阶段交付；第 87 行 在其他目录指定工作区：；第 95 行 输入与布局；第 288 行 旧工作区导入
