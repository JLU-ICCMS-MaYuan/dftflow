# scflow 功能导航

[返回总览](README.md)

原链接保留，当前完整说明已归入 [scflow 家族](scflow/README.md) 与 [scflow CLI](scflow/scflow.md)。

```bash
scflow -c input.toml
```

当前生产 CLI 使用 Rust qf-scflow，读取已有 QE EPC 文件后计算分展宽 λ、频率矩与 Tc，输出 CSV/JSON/PDF。仅有 a2F.dat 不能使用完整 CLI；不要照搬早期单曲线或 Python 入口说明。普通声子矩阵也不能替代 EPC 输入。

实现与数值边界见 [核心库](../../crates/qf-scflow/src/lib.rs)、[0001](../specs/0001-scflow-qe-superconductivity/spec.md)、[0004](../specs/0004-dftflow-rust/spec.md)。本页不把历史物理样本对照扩展为任意材料的收敛保证。
