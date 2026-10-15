# 需求质量检查

以下只检查需求与设计文本，不是实现测试。2026-10-15 已逐项核对；具体证据见 [共同分析](../../0007-epwflow-superconductivity/documentation-analysis.md)。

- [x] CHK001 原始需求与完整确认正文一致，包含较新固定安装取舍。[spec.md 原始需求/确认记录]
- [x] CHK002 每个 US 有优先级、独立验收与失败场景。[spec.md 用户场景]
- [x] CHK003 全部 FR/SC 对应设计、准确文件任务与验证，无遗漏。[plan.md/tasks.md 映射]
- [x] CHK004 CLI、状态、路径和 manifest 与前后 Feature 一致。[contracts/interface.md]
- [x] CHK005 数值容差、复用边界及不可判定处理明确，不以模拟代替真实。[spec.md 成功标准]
- [x] CHK006 阶段依赖无环，共享修改串行，验证先于勾选。[tasks.md 依赖]
- [x] CHK007 预算、数据保护、固定安装与原始要求一致。[spec.md/plan.md]
- [x] CHK008 最终全文已经该项确认，无未解决的范围决策。[issue-draft.md 确认记录]
- [x] CHK009 本地 Issue/Spec 双向链接可读；用户指定本地草稿，远程门不适用，不伪造远程访问。[issue-draft.md/spec.md]

核查结果与具体依据汇总在 Epic 的 documentation-analysis.md；只有实际检查后更新勾选。
