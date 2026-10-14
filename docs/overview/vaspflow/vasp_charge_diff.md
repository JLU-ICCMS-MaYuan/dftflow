# vasp_charge_diff：体积电荷差分

[返回 vaspflow 索引](README.md)

## 功能与当前行为

计算两个 VASP 格式体积场的 A−B。等价入口：vaspflow vasp-charge-diff。

## 使用方法

```bash
vasp_charge_diff CHGCAR_A CHGCAR_B -o delta_charge.vasp
```

A、B 是两个必填位置参数；-o/--output 默认 A_minus_B.vasp。

## 输入、输出与衔接

读取两份网格文件，校验网格尺寸，写出差分 VASP 体积文件。

## 使用边界

不自动重采样或对齐不同结构；用户需保证网格、晶胞和物理参考一致。输出差分不等于已完成可视化。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
