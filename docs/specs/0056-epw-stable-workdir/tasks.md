# 0056 技术任务

Issue 负责协作状态，此文件只负责技术拆解。

- [x] T001 [US1]：在 `epwflow/src/foundation/workspace_archive_tests.rs` 与 `epwflow/tests/three_stage_execution.rs` 验证目录 inode、驻留 shell、打开日志和归档错误（FR-001～005、SC-001～005）。
- [x] T002 [US1]：修改 `epwflow/src/foundation/workspace.rs` 的公共阶段归档，保留根目录并逐项移动旧内容（FR-001～006）。
- [x] T003 [US1]：执行 library、three_stage_execution、native_interpolation 回归，覆盖 SCF 显式重算与上游复用（全部 SC）。
- [x] T004 [US1]：完成制品构建和实际入口检查，在 `acceptance.md` 区分进程契约与未新增科学计算的事实（SC-004）。
- [x] T005：回写根 README、模块手册和 `docs/overview/epwflow/README.md`，按任务路径审计并自动提交。
