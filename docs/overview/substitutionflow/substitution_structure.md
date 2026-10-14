# substitution_structure：生成元素替换结构

[返回家族索引](README.md)

## 功能与入口

按 XA/XB 元素选择规则替换占位结构，生成高通量算例树与初始状态。

等价入口：`substitution structure`。

## 使用方法

```bash
substitution_structure -c input.toml -o cases
```

-c 默认 input.toml；-o/--output-dir 必填。[substitutionflow] 配置 XA、XB、结构来源、pressures、structure_filename；preview 默认 true。

## 输入、输出与衔接

structure_filename 指向配置文件旁的源结构（默认 structure.vasp），结构必须同时包含 XA 与 XB；读取该 VASP 结构，生成各元素组合目录、结构、input.toml、workflow_manifest.json 和初始状态 JSON；元素选择预览打印到终端。

## 使用边界

未设置 pressures 时当前默认 [20, 50, 100] GPa，建议显式填写自己的压力；只准备结构，不运行 VASP/QE；重新生成前应检查已有计算。

## 实现依据与关联

[CLI 定义](../../../substitutionflow/src/bin/substitution.rs)；[工作流实现](../../../crates/df-substitution/src/workflow.rs)；[结构生成](../../../crates/df-substitution/src/structure.rs)。本次只读核对实现，未重新运行程序测试。

关联 [Feature 记录](../../specs/0004-dftflow-rust/spec.md)。
