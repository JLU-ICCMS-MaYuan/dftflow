# wannier_plot：对比 QE 与 Wannier 能带

[返回家族索引](README.md)

## 功能与入口

绘制已整理的 QE 与 Wannier90 能带对比图。

等价入口：`wannier90 plot`。

## 使用方法

```bash
wannier_plot --qe qe_eband --w90 wannier90 --fermi 5.0 --out band_comparison.png
```

--qe/--w90 默认当前目录；--out 默认 band_comparison.png；--fermi 为 eV，示例 5.0 必须换成实际参考能量。

## 输入、输出与衔接

在两目录中寻找 *_band.dat；标签优先读 Wannier 目录的 *_band.labelinfo.dat，再回退 QE 目录。当前实际绘图路径读取分段的“路径距离 能量”数据，输出 PNG。

## 使用边界

多个匹配文件时排序取第一个并警告。默认费米能 23.3313 是历史值，应显式传入实际值。原生 bands.x 头部块格式应先整理成当前分段格式。

## 实现依据与关联

[CLI 定义](../../../wannier90flow/src/bin/wannier90.rs)；[配置模板](../../../wannier90flow/src/config.rs)；[绘图实现](../../../wannier90flow/src/plot.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
