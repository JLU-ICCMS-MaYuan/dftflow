# scflow：通过 qeflow 进行超导后处理

[返回 qeflow 索引](README.md)

## 功能与入口

调用同一 qf-scflow 后处理库，从已有 QE EPC 数据计算 λ、频率矩与 Tc。

等价入口：`qeflow scflow`。

## 使用方法

```bash
qeflow scflow -c input.toml
```

-c 默认 input.toml；无 -r/-s。

## 输入、输出与衔接

配置、输入和输出与 [独立 scflow 命令](../scflow/scflow.md) 相同。

## 使用边界

这是读取 EPC 结果的后处理；普通声子 dyn0 和动力学矩阵不足以计算 Tc。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[超导核心](../../../crates/qf-scflow/src/lib.rs)。本页依据当前代码整理，本次未运行测试。
