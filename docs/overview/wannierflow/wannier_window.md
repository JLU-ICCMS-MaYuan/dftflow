# wannier_window：分析 Wannier 能量窗口

[返回家族索引](README.md)

## 功能与入口

从能带能量文件报告带号/能量范围，或估计自动解缠结窗口。

等价入口：`wannier90 window`。

## 使用方法

```bash
wannier_window EIGENVAL -n 1 8
wannier_window data-file-schema.xml -e -5 5
wannier_window EIGENVAL --auto 1 8 2 --auto-delta 0.02
```

能量文件为必填位置参数；-n 给 1 或 2 个带号，-e 给 1 或 2 个能量；--auto 给 nbnd1 num_wann nbnd3；三种模式选一种。

## 输入、输出与衔接

可读 EIGENVAL 或解析器支持的 QE/FLEUR XML，在终端输出能窗信息；结果可填写到 wannier_init 的 dis_win/dis_froz 参数。

## 使用边界

不会改写 .win 或执行 Wannier90。--auto-delta 默认 0.02 eV；自动估计仍需结合轨道投影检查。

## 实现依据与关联

[CLI 定义](../../../wannier90flow/src/bin/wannier90.rs)；[配置模板](../../../wannier90flow/src/config.rs)；[能窗实现](../../../wannier90flow/src/window.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
