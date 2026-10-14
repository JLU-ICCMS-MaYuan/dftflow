# 独立工具

[返回项目总览](../README.md)

## 功能边界

单独构建、安装和使用的辅助程序。目前收录 qf-gen-dyn0，主工作流不依赖它。

## 命令与功能导航

| 功能 | 类型 | 用途 |
| --- | --- | --- |
| [qf-gen-dyn0](qf-gen-dyn0.md) | CLI | 独立 Rust 不可约 q 网格工具 |

## 功能组成

```text
tools/
└── qf-gen-dyn0.md
```

## 使用关系

SCF XML → 独立算法 → dyn0/q_points.json。需要实际 QE 列表时，转用 qeflow 的 only_init 入口。
