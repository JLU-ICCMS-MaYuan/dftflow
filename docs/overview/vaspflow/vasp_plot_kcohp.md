# vasp_plot_kcohp：能带分辨 pCOHP

[返回 vaspflow 索引](README.md)

## 功能与当前行为

解析 KspaceCOHPBand*.lobster，把成键投影映射到能带。等价入口：vaspflow vasp-plot-k-cohp。

## 使用方法

```bash
vasp_plot_kcohp -d lobster_result --emin=-5 --emax=5 -o kcohp
```

--interaction 选择相互作用；--fermi 设置能量参考；--vmax 默认 1；-o/--output-prefix 控制输出文件前缀。

## 输入、输出与衔接

从 k 分辨 LOBSTER 文件读取能量与投影，输出 DAT 和 PNG。

## 使用边界

需要此前启用 k-space COHP 并完成计算；普通 COHPCAR 不能替代这些输入。

## 实现依据与关联

[CLI 定义及别名映射](../../../vaspflow/src/bin/vaspflow.rs)；[核心实现](../../../crates/df-vasp/src/process.rs)；[VASP 库覆盖](../../../crates/df-vasp/tests/vasp.rs)、[后处理覆盖](../../../crates/df-vasp/tests/process.rs)。覆盖文件的存在不代表本次重新验收。

关联 [0004 Rust 迁移](../../specs/0004-dftflow-rust/spec.md)。
