# QE 声子绘图当前能力

入口：`qe_process_phonon --plot`；实现：`qeflow/phonon/phonon_plot.py`。

- 输入：既有 phband/phdos 归档、路径标签、POSCAR 与配置。
- 读取一次 band/DOS 数据表，导出原有四个 CSV 和两个独立 PNG。
- 额外生成根目录 `phonon_band_dos.jpg`：共享频率轴，左色散、右总/元素 DOS，300 dpi 白底。
- 独立图与联合图复用轴绘制 helper；两类频率范围并集控制联合轴，虚频完整保留。
- 数据读取与计算调度边界不变；单项输入缺失继续失败，不自动补计算。
- 旧工作流完成状态不要求新 JPG；需要联合图的消费方应直接检查该文件或执行绘图命令。

## 上游声子谱输入约定

`qeflow/input_deck/matdyn.py` 按 KPATH 晶体分数坐标写出 `q_in_cryst_coord = .true.`；与标签的倒格子距离约定一致。该修复不改变 DOS 输入或绘图层，不新增命令。旧输入与旧计算结果不会自动迁移，错误路径的频率不能靠重新绘图修复。

修复规格：[0002](../specs/0002-matdyn-crystal-coordinates/spec.md)。

使用说明：[qeflow README](../../qeflow/README.md#8-qe_process_phonon)。
功能规格：[0003](../specs/0003-phonon-band-dos-combined/spec.md)。
