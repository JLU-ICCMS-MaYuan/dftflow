# vasp_process_eband：投影能带表与图

[返回 vaspflow 索引](README.md)

## 功能与当前行为

从 VASPKIT 风格 BAND.dat、PBAND_元素.dat 整理投影能带。等价入口：vaspflow vasp-process-eband。

## 使用方法

```bash
vasp_process_eband -d vasp_eband -e "Mg:s,p" "B:p" --out-prefix pband_merged
```

-d/--directory 默认 .；-e/--elements 接收元素与轨道表达式；--fermi 指定能量参考；--out-prefix 默认 pband_merged。

## 输入、输出与衔接

读取已有 BAND.dat/PBAND 文件，在 -d 目录生成 CSV 与 PNG；需先由相应工具导出这些 DAT。

## 使用边界

直接后处理，没有 -r/-s，也没有在此 CLI 自动启动 VASPKIT 的开关。精细 d 轨道使用 dx2-y2，不使用 dx2。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
