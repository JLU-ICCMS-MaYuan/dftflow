# Gitee Issue 记录

- 标题：将 qeflow 用 Rust 全量重写为单一静态二进制
- 类型：`type-feature`
- 目标：用 Rust 重写现有 Python 版 `qeflow`，产出单一静态二进制，在性能很差的机器上也能快速启动与运行，避免 Python 解释器/动态库导入与重载开销；Rust 二进制仅作为编排层，通过 shell 子进程调用外部 QE/VASP 等计算二进制，不内嵌 Fortran。
- Spec：本目录（编号 0004）；远端 Issue：Gitee 待创建（本机无 `glab` 亦无 Gitee token，需手动在 Gitee 创建后回填编号与链接）。
- 范围：兼容现有 `input.toml` 配置（递归覆盖语义）与 `workflow_status.json`/`task_status.json` 可恢复状态机、Slurm 两段提交、续算跳过 finished；全量覆盖输入生成、QE 文件解析、任务状态机、Slurm 编排、后处理（含 scflow 超导 Tc 计算与绘图），绘图用 Rust crate 替代 matplotlib。
- 范围外：不修改外部 QE/VASP 源码；不引入数据库或常驻 daemon；Darwin 仅作开发构建，发布目标为 Linux musl 静态二进制（`cargo build --release --target x86_64-unknown-linux-musl` + `strip`）。
- 依赖：由 `big-project-planer` 产出总体架构大纲与阶段路线；本 Spec 由 `big-project-spec-runner` 维护详细 Plan/Tasks。
- 追踪：Gitee Issue 编号回填后更新本文件与 `AGENTS.md`；Spec 位于 `docs/specs/0004-qeflow-rust/`。
