# qe_process_eledos：整理电子态密度

[返回 qeflow 索引](README.md)

## 功能与入口

读取已有总 DOS，合并原子、元素与轨道投影，输出 DOS/IDOS 表。

等价入口：`qeflow qe-process-el-edos`。

## 使用方法

```bash
qe_process_eledos -i qe_eledos --proj qe_eledos/MgB2.projwfc_up
```

`-i/--input` 默认当前目录，可指定 TDOS 文件或其目录；`--proj PATH` 指定用于解析壳层编号的投影文件；不传则在输入目录自动寻找。目录输入需要 *.tdos 文件。

## 输入、输出与衔接

读取同目录的 QE PDOS 文件，写出 prefix 对应的 `_dos_proj.dat`、`_dos_proj_sum.dat`、`_idos_proj.dat`、`_idos_proj_sum.dat`。

## 使用边界

直接后处理，无提交选项；须保留完整的 QE 投影文件及其命名，不能只提供绘图图片。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[DOS 解析与输出](../../../crates/qf-io/src/qe_proj.rs)；[黄金数据覆盖](../../../qeflow/tests/postprocess_golden.rs)。本页依据当前代码整理，本次未运行测试。
