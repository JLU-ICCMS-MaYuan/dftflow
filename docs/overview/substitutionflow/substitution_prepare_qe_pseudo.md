# substitution_prepare_qe_pseudo：准备 QE 赝势库

[返回家族索引](README.md)

## 功能与入口

按替换元素范围，从已有赝势库选择文件并准备本地库。

等价入口：`substitution prepare-qe-pseudo`。

## 使用方法

```bash
substitution_prepare_qe_pseudo -c input.toml --pp /path/to/pseudopotentials
```

-c 默认 input.toml；--pp 必填，指向已有库。 --pp 也支持 --pseudo-dir。

## 输入、输出与衔接

读取配置与赝势源，生成 qe_pseudo_lib；遇到多个候选时交互选号。

## 使用边界

不联网下载赝势；批处理前在可交互环境完成选择，并核对泛函与价态。

## 实现依据与关联

[CLI 定义](../../../substitutionflow/src/bin/substitution.rs)；[工作流实现](../../../crates/df-substitution/src/workflow.rs)；[赝势准备](../../../crates/df-substitution/src/prepare.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
