# qe_process_eband：整理投影能带

[返回 qeflow 索引](README.md)

## 功能与入口

解析 projwfc 投影、能带数据和晶胞，输出逐轨道与汇总投影表。

等价入口：`qeflow qe-process-eband`。

## 使用方法

```bash
qe_process_eband -i MgB2.projwfc_up -b elebanddata.dat --cell eband.in --prefix MgB2
```

`-i/--input` 必填；`-b/--bandfile` 默认 `elebanddata.dat`；`--cell` 默认 `eband.in`；`-o/--output`、`--prefix` 控制输出命名。

## 输入、输出与衔接

在已有 QE 电子能带文件基础上导出 `<prefix>_eleband_proj.dat`、`<prefix>_eleband_proj_sum.dat` 和能带 `_band.dat` 三份数据表，保留 k 路径距离与能量列；这些是数据整理产物。

## 使用边界

直接执行本地后处理，不接受 -r/-s，不启动 QE。输入必须是解析器支持的投影格式，不能把 POSCAR 作为 -i。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[投影解析与输出](../../../crates/qf-io/src/qe_proj.rs)；[黄金数据覆盖](../../../qeflow/tests/postprocess_golden.rs)。本页依据当前代码整理，本次未运行测试。
