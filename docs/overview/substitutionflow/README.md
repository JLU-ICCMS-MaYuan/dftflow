# substitutionflow：元素替换与批处理

[返回项目总览](../README.md)

## 功能边界

生成 XA/XB 元素组合，并组织 MatterSim、VASP 和 QE 命令。Rust 程序名为 substitution。当前部分跨 CLI 调用参数仍不兼容，详见 substitution 与 substitution_continue 页，不能视为已完整验收的自动链。

## 命令与功能导航

| 功能 | 类型 | 用途 |
| --- | --- | --- |
| [substitution](substitution.md) | CLI | 批量组织替换计算 |
| [substitution_check_phonon_stability](substitution_check_phonon_stability.md) | CLI | 批量声子稳定性概览 |
| [substitution_continue](substitution_continue.md) | CLI | 更新状态与续算 |
| [substitution_plot_eledos](substitution_plot_eledos.md) | CLI | 批量 DOS 总览图 |
| [substitution_prepare_potcar](substitution_prepare_potcar.md) | CLI | 准备 VASP 赝势库 |
| [substitution_prepare_qe_pseudo](substitution_prepare_qe_pseudo.md) | CLI | 准备 QE 赝势库 |
| [substitution_structure](substitution_structure.md) | CLI | 生成元素替换结构 |

## 功能组成

```text
substitutionflow/
├── substitution.md
├── substitution_check_phonon_stability.md
├── substitution_continue.md
├── substitution_plot_eledos.md
├── substitution_prepare_potcar.md
├── substitution_prepare_qe_pseudo.md
└── substitution_structure.md
```

## 使用关系

structure 准备案例，prepare_* 准备赝势库，substitution/continue 组织阶段与状态，plot_eledos/check_phonon_stability 汇总已有输出。配置 phonon_mode 使用 nosplit/split_index/split_coords。

```mermaid
flowchart LR
    A[元素选择与占位结构] --> B[substitution_structure]
    C[prepare_potcar / prepare_qe_pseudo] --> D[substitution / continue]
    B --> D
    D --> E[外部计算及状态文件]
    E --> F[DOS / 声子稳定性总览]
```
