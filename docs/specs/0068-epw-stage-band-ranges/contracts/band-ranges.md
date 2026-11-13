# 能带范围图契约

| 阶段 | 科学来源 | 费米零点 | 派生目录 |
| --- | --- | --- | --- |
| scf | 1.qe-scf/snapshot/<prefix>.save/data-file-schema.xml | 自身 SCF | 1.qe-scf/plots |
| nscf | 2.qe-nscf/out/<prefix>.save/data-file-schema.xml | 自身 NSCF | 2.qe-nscf/plots |
| electronic | 3.qe-electronic/qe_eband/out/<prefix>.save/data-file-schema.xml | NSCF | 3.qe-electronic/qe_eband/plots |

图名分别 scf-band-ranges、nscf-band-ranges、electronic-band-ranges，提供 .png／.svg／.csv／.json。同源表保留绝对能量，图用相对能量。派生发布至 results/<阶段>/band-ranges/，与原有电子图 unit 不冲突。JSON 是图参与统计，不是第二份科学状态。

图例在主轴右侧，底注在图下，自动高度适应跨费米带数量。纵轴完整容纳全部能带和有限窗口。重合窗口用相位交错虚线、合并标签。PNG 300 DPI，固定宽布局，SVG 可供直接核对。

无新增科学 CLI 或配置参数。原三阶段命令自然触发绘图；已有结果重复执行时补图。绘图异常明确报派生失败，保留科学结果。纯能量超限不会导致诊断函数失败，但正式选窗仍拒绝。
