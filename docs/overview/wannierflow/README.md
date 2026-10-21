# wannierflow：Wannier90 接口与分析

[返回项目总览](../README.md)

## 功能边界

按用户要求将文档归入 wannierflow/；源码目录仍为 wannier90flow/，家族程序名仍为 wannier90。文档目录名不会创建 wannierflow 可执行文件。外部计算程序是 wannier90.x、pw2wannier90.x、postw90.x。

## 命令与功能导航

| 功能 | 类型 | 用途 |
| --- | --- | --- |
| [wannier_init](wannier_init.md) | CLI | 初始化 Wannier90 流程 |
| [wannier_plot](wannier_plot.md) | CLI | 对比 QE 与 Wannier 能带 |
| [wannier_post](wannier_post.md) | CLI | Wannier90 DOS 后处理 |
| [wannier_window](wannier_window.md) | CLI | 分析 Wannier 能量窗口 |

## 功能组成

```text
wannierflow/
├── wannier_init.md
├── wannier_plot.md
├── wannier_post.md
└── wannier_window.md
```

## 使用关系

window 与 init 共用指定上游默认的能量冻结＋投影解纠缠策略；init 直接从配置中的真实 NSCF bands_file 生成窗口及来源记录。window 的 -n／-e 保留只读分析，旧 --auto 已弃用。post 读取计算结果，plot 比较已有能带。init 之前需完成匹配的 QE 电子计算；实际程序能力与科学收敛单独检查。

init／post 的默认 Slurm header 保留 `slurm.out`／`slurm.err` 简短文件名，增加 `#SBATCH --chdir` 固定日志目录。正文使用生成时确定的实际计算目录，避免 Slurm spool 副本的路径影响执行。用户自定义 header 原样保留；跨目录日志及 spool 目录回归见 [0022 验收](../../specs/0022-slurm-log-filenames/acceptance.md)。

```mermaid
flowchart LR
    A[已有电子能量] --> B[wannier_window]
    B --> C[核对共享选窗结果与投影配置]
    C --> D[wannier_init]
    D --> E[wannier_post]
    D --> F[wannier_plot]
```
