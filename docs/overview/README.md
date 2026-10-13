# 当前功能总览

- [QE 声子绘图](qe-phonon-plot.md)：现有归档读取、独立 PNG/CSV 和共享频率轴联合 JPG。
- [scflow 超导后处理](scflow.md)：从 QE EPC 输出计算 A2F、McMillan/Allen-Dynes Tc 和 Eliashberg Tc。
- QE 在 Apple M5 安装手册：已移至本地留档 `docs/local/qe-install-m5.md`（含机器路径与软件配置，按 Issue 0005 要求不入 Git）。
- [dftflow Rust 全量迁移](dftflow-rust.md)：将 dftflow 五个 Python 流包（qeflow/vaspflow/wannier90flow/substitutionflow/scflow）全量迁移为 Rust 家族二进制；当前为进行中，骨架/配置/IO/状态机/调度/垂直切片已落地。

本索引收录已核验的能力与安装经验，以及进行中的 dftflow Rust 迁移当前边界；不表示仓库其他模块尚未实现。
