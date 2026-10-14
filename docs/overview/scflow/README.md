# scflow：QE EPC 超导后处理

[返回项目总览](../README.md)

## 功能边界

对已有 QE 电子声子耦合数据执行数值后处理。独立 scflow 与 qeflow scflow 共用 Rust qf-scflow 库；不启动第一性原理计算。

## 命令与功能导航

| 功能 | 类型 | 用途 |
| --- | --- | --- |
| [scflow](scflow.md) | CLI | QE 电子声子耦合超导后处理 |

## 功能组成

```text
scflow/
└── scflow.md
```

## 使用关系

EPC 文件与 q 权重 → 分展宽 direct/interp → λ/频率矩/Tc → CSV/JSON/PDF。普通声子结果没有耦合数据，不能代替 EPC 输入。
