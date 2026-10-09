# qeflow Rust 全量重写（单一静态二进制）

入口（规划）：`qeflow <subcommand>`；实现：`qeflow-rust/`（cargo workspace，分支 `qeflow-rust`）。

- 目标：用 Rust 重写现有 Python 版 `qeflow`，产出单一静态二进制（`cargo build --target x86_64-unknown-linux-musl` + `strip`），在弱机器上快速启动；仅作为编排层，通过 shell 子进程调用外部 QE/VASP，不内嵌 Fortran。
- 配置：`input.toml` 递归覆盖语义复刻 `config_templates._merge_dict_in_place`（`dict×dict` 递归，否则覆盖，`None` 不覆盖）。
- 状态：`workflow_status.json`/`task_status.json` 复刻三态（unstarted/finished/error）+ q 点 task 状态，续算跳过 finished。
- 调度：`std::process::Command` spawn 外部 QE/VASP；Slurm 两段提交（pre: scf→dyn0→split；post: merge→q2r→matdyn→plot）1:1 对齐现有 `qe_phonon_split_*`。

功能规格：[0004](../specs/0004-qeflow-rust/spec.md)；调研与架构：[`docs/qeflow-rust/`](../../qeflow-rust/../docs/qeflow-rust/)。
- 远端 Issue：[Gitee IKKGLP（编号 0004）](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKKGLP)。

- 已落地：cargo workspace 骨架与内部 crate `qf-config`/`qf-crystal`/`qf-input`/`qf-io`/`qf-phonon`/`qf-runner`/`qf-scflow`/`qf-state`；其中 `qf-config`（递归覆盖 TOML）、`qf-io`（QE 输出读取）、`qf-state`（状态机/续算）、`qf-runner`（spawn/Slurm 两段提交）已实现。
- 已验证：垂直切片 `qe-scf → qe-nscf` 与 `scflow` 单点已跑通；musl 静态二进制构建路径已搭通骨架；调研与架构文档已写入 `docs/qeflow-rust/`。
- 边界：全量迁移（`qf-crystal`/`qf-input` 全量输入 deck、`qf-phonon` split/merge 编排、`qf-scflow` 数值与 `plotters` 绘图、`qf-cli` 子命令）仍在计划中，尚未替代现有 Python `qeflow`；Darwin 仅作开发构建，发布目标为 Linux musl 静态二进制。
