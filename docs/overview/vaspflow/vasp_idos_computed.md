# vasp_idos_computed：能窗内积分 DOS

[返回 vaspflow 索引](README.md)

## 功能与当前行为

在给定能量范围积分总 DOS 和选定投影 DOS。等价入口：vaspflow vasp-idos-computed。

## 使用方法

```bash
vasp_idos_computed -d vasp_eledos --low=-5 --up=0 -e "B:p" --skip-vaspkit
```

--low、--up 必填；--scf-outcar 默认 ../vasp_scf/OUTCAR；--doscar 默认 DOSCAR；--skip-vaspkit 使用已有 DAT；--per-volume 体积归一；--out-prefix 默认 idos_computed。

## 输入、输出与衔接

从 SCF OUTCAR 读取费米能，原地更新 DOSCAR 的费米能字段，再按选项调用 VASPKIT 导出 TDOS/PDOS；读取 POSCAR 体积并积分，结果写 idos_computed.csv（或指定前缀）。即使 --skip-vaspkit 也仍会更新 DOSCAR，需要该文件和可读 POSCAR。

## 使用边界

能窗相对参考能量，需核对 SCF OUTCAR 与 DOS 数据属于同一体系；本命令不是计算新的电子态。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
