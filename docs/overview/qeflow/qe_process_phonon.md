# qe_process_phonon：声子后处理、收集与绘图

[返回 qeflow 索引](README.md)

## 功能与入口

在已有动力学矩阵基础上准备/执行 q2r 与两项 matdyn，也可单独收集和绘图。

等价入口：`qeflow qe-process-phonon`。

## 使用方法

```bash
qe_process_phonon -c input.toml -r
# 已有 QE 后处理结果时，按次序使用以下两条
qe_process_phonon -c input.toml --collect
qe_process_phonon -c input.toml --plot
```

`-c`、`-i` 及 -p/--poscar 别名；普通模式无 -r/-s 只准备，-r 直接运行，-s 提交。--collect / --plot 是立即执行的专用分支。

## 输入、输出与衔接

已有 dyn → q2r → matdyn_phband/matdyn_phdos → phband/phdos 归档 → CSV、PNG 与 phonon_combined.jpg。完整文件清单见 [声子工作流](phonon-workflow.md)。默认后处理工作目录是配置文件目录，可用 qe_phonon.work_dir 明确指定。

## 使用边界

--plot 优先于 --collect；不要合写成 --collect --plot 期待顺序执行。需提供 KPATH.in 及结构，保证配置 prefix 与结果一致。该入口不重算 SCF/PH；分 q 自动链不需再手工提交它。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[归档逻辑](../../../crates/qf-phonon/src/archive.rs)；[绘图实现](../../../crates/qf-phonon/src/plot.rs)。本页依据当前代码整理，本次未运行测试。

关联 [0002 坐标声明](../../specs/0002-matdyn-crystal-coordinates/spec.md)、[0003 联合图](../../specs/0003-phonon-band-dos-combined/spec.md) 与 [0006](../../specs/0006-qe-phonon-submit-chain/spec.md)。
