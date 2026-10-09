# 当前功能总览

- [QE 声子绘图](qe-phonon-plot.md)：现有归档读取、独立 PNG/CSV 和共享频率轴联合 JPG。
- [scflow 超导后处理](scflow.md)：从 QE EPC 输出计算 A2F、McMillan/Allen-Dynes Tc 和 Eliashberg Tc。
- [QE 在 Apple M5 安装手册](qe-install-m5.md)：M5（arm64）上从 `external/q-e` 子模块编译安装 Quantum ESPRESSO，含依赖、CMake 配置、编译验证及本次真实踩坑记录。
- [qeflow Rust 全量重写](qeflow-rust.md)：将 Python 版 `qeflow` 重写为单一 musl 静态二进制，仅作编排层 spawn 外部 QE/VASP；当前为进行中，骨架/配置/IO/状态机/调度/垂直切片已落地。

本索引收录已核验的能力与安装经验，以及进行中的 qeflow-rust 重写当前边界；不表示仓库其他模块尚未实现。
