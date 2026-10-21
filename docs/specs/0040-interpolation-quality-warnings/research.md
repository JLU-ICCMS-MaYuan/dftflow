# 调研与决策

当前 1314972 已完成原生粗 EPC／比较，电子 RMS 0.022388185 eV、声子最大／RMS 184.512305／67.118317 cm⁻¹，独立报告真实失败。硬阻断散布在 interpolation::scheduler、superconductivity::scheduler、foundation::workflow、interpolation::cli 和通用 quality 函数。仅删除首个 ensure 无法覆盖细网格和续算。

采用默认告警策略，避免额外开关和分叉；数值报告仍 failed，阶段可用摘要 warning，二者职责分离。保留质量参考和不可变数据检查，避免把损坏当作普通误差。相比提高阈值，本方案不改变理想目标和任何历史结论。超导原生执行仍可能因真实数值问题失败，该失败须继续 debug 或如实记录。

这是用户明确改变质量策略的新 Feature，不作为旧缺陷的偷偷放宽，也不重写早期验收中的阈值阻断事实。
