# substitution_check_phonon_stability：批量声子稳定性概览

[返回家族索引](README.md)

## 功能与入口

从已有 DOS CSV 提取最低频率，绘制稳定性分类图。

等价入口：`substitution check-phonon-stability`。

## 使用方法

```bash
substitution_check_phonon_stability -d cases/12.XA_Mg -j 4
```

-d 可多个目录，不指定则自动发现；-j/--jobs 控制并行度。

示例的 12.XA_Mg 应换成实际生成的 XA 分组目录。

## 输入、输出与衔接

读 phdos/phonon_dos_elements.csv 的 frequency_cm-1，除以 33.35641 换算 THz；小于 -1 THz 为 unstable，读取失败为 unfinished，其余为 stable。输出 `phonon_stability_summary_<pressure>.png`。

## 使用边界

这是固定阈值的最低 DOS 频率分类，不等同于严格动力学稳定性判定；不重算声子。

## 实现依据与关联

[CLI 定义](../../../substitutionflow/src/bin/substitution.rs)；[工作流实现](../../../crates/df-substitution/src/workflow.rs)；[稳定性实现](../../../crates/df-substitution/src/check_phonon.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
