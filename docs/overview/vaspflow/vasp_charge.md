# vasp_charge：电荷密度平面平均

[返回 vaspflow 索引](README.md)

## 功能与当前行为

读取 CHGCAR 体积网格，沿指定方向求平面平均。等价入口：vaspflow vasp-process-charge。

## 使用方法

```bash
vasp_charge -d calculation -f CHGCAR --axis 2
```

-d 默认 .；-f/--file 默认 CHGCAR；--axis 为 0/1/2，默认 2；--out-prefix 默认 charge。

## 输入、输出与衔接

输出 `charge_planar_average.dat` 与同名 PNG，位于数据目录。

## 使用边界

不运行 VASP；axis 是数组方向编号，不是任意空间法向。此独立别名不生成电荷计算输入；准备计算请使用 [vaspflow vasp-charge](vaspflow-vasp-charge.md)。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
