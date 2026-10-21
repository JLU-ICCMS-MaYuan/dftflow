# 0044 实施任务

输入：[规格](spec.md)、[计划](plan.md)、[设计](research.md)。无新增公共接口或持久模型。

- [x] T001 保存 docs/specs/0044-mpi-submit-command/ 全部需求与设计产物，核对批准范围。
- [x] T002 [US1] 新增 crates/qf-runner/tests/mpi_command.rs 的前缀及实际脚本执行测试。
- [x] T003 [US1] 在 crates/qf-runner/src/mpi.rs 实现共享转换，并接入 QE／VASP／Wannier／EPW 生成边界。
- [x] T004 [US1] 更新 epwflow/tests/support/direct_submit.rs，补充公开入口 PATH／失败回归并运行相关和工作区测试。
- [x] T005 [US1] 回写 docs/overview/README.md、模块说明、配置示例及 acceptance.md，显式路径提交。
- [x] T006 [US2] 从提交构建安装，迁移未冻结现用脚本，核对受保护哈希并记录 deployment.md。
- [x] T007 [US2] 安装版回归、最终收敛和部署证据提交，保持线上链接未完成。

依赖顺序 T001→T002→T003→T004→T005→T006→T007。FR／SC 映射见 plan.md；T002–T005 覆盖 US1，T006–T007 覆盖 US2。
