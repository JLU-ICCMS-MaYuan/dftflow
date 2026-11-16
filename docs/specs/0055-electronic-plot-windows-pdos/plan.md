# 实施计划

## 设计与职责

1. 配置层增加带独立默认值的 PlotConfig，通过当前全部 TOML 布局与检查点序列化；禁止将显示范围加入科学身份。
2. workspace_plot 负责现有原生数值适配和审计表，combined_plot 统一渲染自包含图参；电子与声子共用左右对齐的图面板。
3. 第 3 步保留路径即时逐带范围诊断，组合图在 DOS 数据就绪后生成。第 4 步复用既有元素聚合与平滑。
4. 第 5 步复用原生能带和 DOS，不重新拟合；全范围只读取 QE 路径极值。第 6 步消费既有比较表，不再调用四图与总览生成器。
5. 阶段 plot/ 与 results 发布清单共同更新；使用当前事务发布机制移除旧清单登记图片，不删除未知或科学文件。

## 契约

- [plot].electronic_energy_min_ev / electronic_energy_max_ev：相对 E_F 的有限递增边界，默认 −3、＋3。
- 电子图片：electronic.png 与 electronic-from-lowest-to-highest.png；自包含图参同名 .plot.json。
- 声子图片：phonon.png，自包含图参 phonon.plot.json；纵轴 0 至路径最高频率。
- 电子全范围值记录于 electronic-display.json，明确来源为 QE 路径。
- 原始 CSV 和归一化／指标数据保持独立，不因显示范围重算或截断。

## 失败与兼容

科学成功、绘图失败分开保存；补图只读已有数值。历史独立结果包仍保留有界旧格式读取，新生成端不再产生旧分离图。现场 --plot 使用新组合图图参，--results 使用包内图参，不访问原工作区。
