# 0075 技术任务

协作状态以 [Issue #75](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/75) 为准；本文件只记录技术拆解。

依赖顺序为 T001 → T002 → T003 → T004 → T005。证据统一记录在 [acceptance.md](acceptance.md)。

- [x] T001 在 `epwflow/src/qe_data/phonon_state.rs` 实现状态、原子读写及并发／序列化测试（FR-005）。
- [x] T002 在 `epwflow/src/qe_data/phonon_parallel.rs` 生成 init／q／collect 脚本，复用 `phonon.rs` 的原生列表与 PH 执行；在 `scheduler.rs` 提供无 PH 重算的归集边界（FR-001～004；首次时序差异保留 T007）。
- [x] T003 在 `epwflow/src/foundation/workspace.rs`、`program.rs` 接入独立分配、严格回执和依赖，保留 nosplit 原路径（FR-003～005、007）。
- [x] T004 在 `epwflow/tests/support/phonon_parallel.rs` 验证公开 CLI 的失败 q 续算、collect 依赖更新、init／collect 失败和未知回执保护（FR-006、SC-001～005 的已实现部分；不包括 T007 时序确认）。
- [ ] T005 执行状态／脚本单元测试和进程回归，编译并覆盖固定 EPW 命令；用本机 MgB₂ 输入生成两种模式脚本，零提交。
- [x] T006 使用真实 Slurm 和原生 MgB₂ 完成 index／coords 计算及取消单 q 后续算的科学验收。2026-11-18：每 q 6 路 MPI，作业 90～101；失败阻断、只重提失败点、成功点不变、collect 原号更新依赖及原生后处理／绘图均通过，见 `acceptance.md` 的本次原生验收段。此项不表示网格收敛或 T007 时序问题已解决。
- [ ] T007 核对首次无 dyn0 的自动展开时序与 FR-003／SC-001 描述差异，取得需求确认后回写 Spec；不把默认实现取舍写成用户决定。
- [ ] T008 回写 `epwflow/README.md` 与 `docs/overview/epwflow/README.md`，保留真实验收边界；远程双向链接待有明确推送授权后核验。

T008 的本地文档已同步，远程链接尚未验证。扩展回归 21 个旧用例失败，代表性更新前复现和不提交原因见 `acceptance.md`，不将整体质量门标为通过。
