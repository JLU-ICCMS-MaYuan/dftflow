# qe_matdyn_phband：力常数插值声子谱

[返回 qeflow 索引](README.md)

## 功能与入口

生成输入并单步调用 matdyn.x。

等价入口：`qeflow qe-matdyn-phband`。

## 使用方法

```bash
qe_matdyn_phband -c input.toml
qe_matdyn_phband -c input.toml -r
```

`-c` 默认 input.toml；`-i/--input` 可指定结构；无标记只准备，-r 直接执行，-s 提交单步作业，二选一。

## 输入、输出与衔接

输入：q2r 力常数及 KPATH.in。读取 `[qe_phonon]` 对应子段，在工作目录输出 matdyn_phband.in/out、prefix.phband.freq 与路径标签。后续用 qe_process_phonon --collect / --plot 整理。

## 使用边界

不自动补齐前序计算，也不为单独提交的多个步骤自动建立依赖。请顺序执行或使用 split 全链路。 能带路径以晶体坐标写入，声明 q_in_cryst_coord=true。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[后处理输入生成](../../../crates/qf-input/src/matdyn.rs)。本页依据当前代码整理，本次未运行测试。
