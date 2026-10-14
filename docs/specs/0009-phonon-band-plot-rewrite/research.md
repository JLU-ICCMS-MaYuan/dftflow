# 0009 调查与决策

## 已查明

plot.rs 两处 labels.get(qi) 将 183 点频率错误映射到 12 条端点；CSV 已有插值函数，但缺少索引/数量校验。联合图 DOS 仍画频率横轴且用频率最大值作 DOS 横轴上限，两面板不是实际共享频率；固定零下限裁虚频。选择重写共享数据与绘图层，拒绝仅调字体或继续单独复制插值。

## Python 与 VASPKit

Python 的 kpath.py 读取成对端点、合并连续段；结束/跳段端点权重为 1，普通段为 kpath_points。倒格矢为 2π inv(lattice).T；跳段距离不增加。matdyn 显式 q_in_cryst_coord=true。Python 绘图读取插值后的 CSV 数据并使用端点刻度，联合图约 3:1、共享完整频率范围。Python 将跨段同距离点连成竖线、标签重叠的问题不照搬，按确认 Spec 拆线与合并标签。

## 实现取舍

plotters 使用像素坐标绘制标签置于轴下，保留浮点路径坐标；不使用数值刻度的近似 formatter，避免高对称点落不到刻度。单图和联合图共用同一绘制器及数据。保留现有文件名；不依赖 Python 绘图运行时。测试/参考调用 Python 仅用于对照。

投影聚合使用 CLI 已解析结构的 elements/counts 传入绘图，不在绘图层重新读取固定名称 POSCAR，保留 -i 指定任意结构文件的行为。CLI 回归特意使用 selected.vasp 且工作目录无 POSCAR。
