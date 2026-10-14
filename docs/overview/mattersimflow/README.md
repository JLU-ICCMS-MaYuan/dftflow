# mattersimflow：MatterSim 预优化

[返回项目总览](../README.md)

## 功能边界

保留的 Python 工具家族，公开入口只有 mattersim_opt；不属于五个 Rust 家族安装制品。

## 命令与功能导航

| 功能 | 类型 | 用途 |
| --- | --- | --- |
| [mattersim_opt](mattersim_opt.md) | CLI | 机器学习势结构预优化 |

## 功能组成

```text
mattersimflow/
└── mattersim_opt.md
```

## 使用关系

输入结构与阶段配置 → 生成 Python runner → MatterSim/ASE 优化 → CONTCAR，可再交给 VASP/QE 或替换工作流。
