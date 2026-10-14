# substitution_plot_eledos：批量 DOS 总览图

[返回家族索引](README.md)

## 功能与入口

按元素组合和压力汇总费米能处 DOS，绘制周期表式总览。

等价入口：`substitution plot-eledos`。

## 使用方法

```bash
substitution_plot_eledos -d cases/12.XA_Mg
```

-d/--directory 可多个分组目录；不指定时自动发现当前目录下可处理的分组。

示例的 12.XA_Mg 应换成实际生成的 XA 分组目录。

## 输入、输出与衔接

读取 manifest、状态及 *_at_ef.csv，按压力写 `eledos_summary_<pressure>.dat/png`。

## 使用边界

每个结果目录须能唯一确定 *_at_ef.csv；不补跑 DOS。

## 实现依据与关联

[CLI 定义](../../../substitutionflow/src/bin/substitution.rs)；[工作流实现](../../../crates/df-substitution/src/workflow.rs)；[汇总实现](../../../crates/df-substitution/src/plot_eledos.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
