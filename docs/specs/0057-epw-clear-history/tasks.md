# 0057 技术任务

Issue 负责协作状态，此文件只负责技术拆解。

- [x] T001 [US1]：在 `epwflow/tests/clear_history.rs` 建立三类历史、只读预览、链接、任务、引用、权限失败和幂等的公开 CLI 测试（FR-001～008、SC-001～005）。
- [x] T002 [US1]：实现 `epwflow/src/foundation/clear.rs` 与 `epwflow/src/bin/epw_clear.rs`，注册 Cargo 二进制和模块（全部 FR）。
- [x] T003 [US1]：更新 `scripts/product-layout.sh`、模块打包和安装／环境／自动发布回归，验证新增入口（SC-006）。
- [x] T004 [US1]：构建发布新入口，记录真实 MgB₂ 现用文件和账本哈希、只读状态，再执行预览／清理／核对／幂等检查，保存 `acceptance.md`（全部 SC）。
- [x] T005：回写根 README、模块说明与 Overview，审计任务路径及暂存字节后自动提交。
