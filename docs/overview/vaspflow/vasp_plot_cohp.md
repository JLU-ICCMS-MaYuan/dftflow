# vasp_plot_cohp：COHP 与积分成键分析

[返回 vaspflow 索引](README.md)

## 功能与当前行为

读取 COHPCAR.lobster，绘制平均与投影 COHP 并分析指定能窗的 ICOHP。等价入口：vaspflow vasp-process-cohp。

## 使用方法

```bash
vasp_plot_cohp -d lobster_result --emin=-10 --emax=5 --lower_energy=-5 --upper_energy=0
```

-d 默认 .；-f/--file 默认 COHPCAR.lobster；--interaction 可选相互作用；--lower_energy/--upper_energy 为积分窗（别名 --low/--up），--emin/--emax 为绘图范围；--out-prefix 改名。

## 输入、输出与衔接

默认生成 average_cohp.dat/png 与 proj_cohp.dat/png，并报告积分分析。

## 使用边界

绘图范围与积分能窗是不同参数。必须已有 LOBSTER 结果；命令不会运行 VASP 或 LOBSTER。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
