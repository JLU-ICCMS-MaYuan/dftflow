# QE 声子绘图导航

[返回总览](README.md)

原页面保留以兼容既有链接；当前 Rust 实现与完整用法已归入 qeflow：

- [qe_process_phonon：后处理、收集与绘图](qeflow/qe_process_phonon.md)。
- [三种声子模式与产物目录](qeflow/phonon-workflow.md)。
- [matdyn 声子谱](qeflow/qe_matdyn_phband.md)、[matdyn DOS](qeflow/qe_matdyn_phdos.md)。

已有 matdyn 结果时依次执行：

```bash
qe_process_phonon -c input.toml --collect
qe_process_phonon -c input.toml --plot
```

--collect 与 --plot 分开调用。当前输出为 phband/phonon_band.png、phdos/phonon_dos.png 以及工作目录的 phonon_combined.jpg，伴随 CSV；联合图共享频率轴并保留虚频。不再引用迁移前已删除的 Python 绘图模块。

实现见 [Rust 绘图](../../crates/qf-phonon/src/plot.rs) 和 [归档](../../crates/qf-phonon/src/archive.rs)；历史需求见 [0003](../specs/0003-phonon-band-dos-combined/spec.md)，自动串联验收见 [0006](../specs/0006-qe-phonon-submit-chain/acceptance.md)。
