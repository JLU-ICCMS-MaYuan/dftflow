# qe_q2r：动力学矩阵转力常数

[返回 qeflow 索引](README.md)

## 功能与入口

生成输入并单步调用 q2r.x。

等价入口：`qeflow qe-q2r`。

## 使用方法

```bash
qe_q2r -c input.toml
qe_q2r -c input.toml -r
```

`-c` 默认 input.toml；`-i/--input` 可指定结构；无标记只准备，-r 直接执行，-s 提交单步作业，二选一。

## 输入、输出与衔接

输入：已齐全且同一网格的 prefix.dyn0 与各动力学矩阵。读取 `[qe_phonon]` 对应子段，在工作目录输出 q2r.in、q2r.out 和实空间力常数。后续用 qe_process_phonon --collect / --plot 整理。

## 使用边界

不自动补齐前序计算，也不为单独提交的多个步骤自动建立依赖。请顺序执行或使用 split 全链路。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[后处理输入生成](../../../crates/qf-input/src/q2r.rs)。本页依据当前代码整理，本次未运行测试。
