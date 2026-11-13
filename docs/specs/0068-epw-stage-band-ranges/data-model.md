# 数据模型

报告保存 schema_version、renderer_version、stage、sample_kind、prefix、fermi_ev／fermi_source、实际 nk／nbnd、num_wann、四个可选绝对窗口、window_source／window_error、band_ranges 和逐 k 点 outer_counts／energy_frozen_counts。

每个 band 保存原始编号（1-based）、min_ev、max_ev、crosses_fermi、excluded；crosses_fermi 为 min≤E_F≤max（含边界）。统计在排除指定带之后执行；能量冻结不包含原生 AMN 投影冻结。

派生缓存键绑定来源 XML 身份、有效 Wannier 配置、阶段、E_F 来源、渲染版本。绘图完成记录由现有阶段日志保存；PNG／SVG／CSV／JSON 损坏或缺失可仅补派生输出。科学 XML／输入和 raw payload 不改写。
