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

## 2026-10-19：叠图需求检查

下列只检查新增文档，不代表实现／真实验收；核查证据统一见 [共同分析](../../0007-epwflow-superconductivity/documentation-analysis.md)。

- [x] 新增原话及共同有效能窗／QE 曲线加直接点选择已逐字保留；严格轨道与四图加总览选择不遗漏。
- [x] 新 FR/SC 对应 Plan、数据契约、任务与待验收记录，任务编号唯一、链接有效。
- [x] F3 唯一生产数据和投影，F5 消费绘图，0015 验证；无循环依赖或重复 QE 流程。
- [x] 共同能窗、DOS 核／单位／权重、真实原子投影与质量加权本征矢定义完整；缺项不替换成总量。
- [x] QE 实线先画、EPW 虚线最后画；四图／总览／三格式、独立重绘和仅插值诊断有验收任务。
- [x] 原科学阈值、6³ 失败记录、共享预算与源数据保护未改变；新增实现任务全部未完成。
