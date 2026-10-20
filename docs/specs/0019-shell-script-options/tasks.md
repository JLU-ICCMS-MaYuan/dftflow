# 技术任务

依赖顺序：T001 → T002／T003 → T004 → T005；本次顺序执行，无并行代理。US1 和 US2 均为本次最小交付范围。

- [x] T001 审计跟踪 shell 脚本、全部 flow 生成器和严格选项变体，记录到 docs/specs/0019-shell-script-options/acceptance.md。
- [x] T002 [US1] 修改 crates/qf-runner/src/lib.rs、crates/qf-phonon/src/split.rs、crates/df-substitution/src/workflow.rs、qeflow/src/、wannier90flow/src/init.rs 与 epwflow/src/ 生成逻辑，显式传播关键失败。
- [x] T003 [US2] 修改 scripts/、test_auto/MgB2/ 下跟踪入口及 Rust 测试中的脚本模板，移除严格选项并保留断言失败退出。
- [x] T004 [US1] 验证 epwflow/tests/three_programs.rs 的实际脚本初始化行为、相关 Rust 回归、shell 语法和安装失败保护，回写 docs/specs/0019-shell-script-options/acceptance.md。
- [x] T005 [US2] 按实际证据更新 docs/overview/README.md、docs/overview/epwflow/README.md 和本目录的文档检查、验收及 Issue 草稿。

完成依据：acceptance.md 中 308 文件审计、78 项 Rust 回归、19 份 shell 语法，以及实际脚本初始化、示例生成、环境入口和安装失败场景；线上链接核验不计入本地技术任务完成。
