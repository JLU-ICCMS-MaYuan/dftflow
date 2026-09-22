# 技术决策

**Issue**：[IKHQNI](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHQNI)

| 决策 | 理由与现状证据 | 未采用方案 |
| --- | --- | --- |
| 在现有绘图末尾额外生成 JPG | `phonon_plot.py` 已同时持有 band_rows 与 dos_elements_rows | 新 CLI 重复已有读取和配置体系 |
| sharey=True；色散: DOS 宽度比 3:1 | 用户需要同一频率坐标；图例在较窄 DOS 面板 | 拼接 PNG 不能保证实际轴坐标对齐 |
| 提取轴绘制 helper | 独立 PNG 和联合 JPG 共用曲线及标签逻辑 | 重复实现容易出现分波或单位漂移 |
| 原 CSV 与 PNG 不变 | 保持用户已有脚本和数据导出稳定 | 替换旧输出会使现有消费方失效 |
| JPG 不加入历史完成判定 | `check_phonon_plot` 与 `archive.phonon_plot_files()` 保存旧完成契约 | 要求旧工作区新增文件会触发状态回退 |
| 保持异常传播，finally 关闭图 | 保存失败不会遗留 matplotlib 图对象 | 吞异常会把失败误报为成功 |

数据读取、结构顺序和路径重建均复用现有实现。本 Feature 不顺带修改既有解析容错或工作流逻辑。
