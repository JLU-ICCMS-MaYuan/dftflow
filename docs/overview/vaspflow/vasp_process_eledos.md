# vasp_process_eledos：投影 DOS 表与图

[返回 vaspflow 索引](README.md)

## 功能与当前行为

从 TDOS.dat、PDOS_元素.dat 汇总指定轨道，导出 DOS 与费米能处数值。等价入口：vaspflow vasp-process-eledos。

## 使用方法

```bash
vasp_process_eledos -d vasp_eledos -e "Mg:s,p" "B:p"
```

-d 默认 .；-e/--elements 可多个；--per-volume 启用体积归一；--out-prefix 默认 pdos_merged。

## 输入、输出与衔接

在数据目录输出汇总 CSV、*_at_ef.csv 和 PNG；按体积归一时应提供可读取的 POSCAR；当前体积读取失败会回退为 1.0，不能把这种结果当作已正确按体积归一。

## 使用边界

输入 DAT 应已导出；当前 CLI 不接受旧参数 --elements-total 或 --skip-vaspkit。精细 d 轨道使用 dx2-y2。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
