# 实施任务

- [x] T001 建立 scflow Spec、配置/输出契约和验证命令。
- [x] T002 为配置、SciPy 兼容和公开 QE 归档 CLI 写回归测试，并确认修复前失败。
- [x] T003 复制 A2F Python 模块，改为 scflow 包内导入。
- [x] T004 增加 TOML 配置、CLI、结果目录控制和 pyproject 注册。
- [x] T005 修复现代 SciPy、pymatgen、NumPy 标量兼容问题并通过公开归档测试。
- [x] T006 使用用户提供的真实 QE EPC 归档执行现场验收并记录 Tc；10 组数据已对照，SC-004 部分失败，详见 acceptance.md。
- [ ] T007 修复真实归档暴露的问题、通过 SC-004 后关闭 Issue；本次对照记录与失败原因另行回写并精确提交，不标记 Feature 完成。

依赖：T001 → T002 → T003 → T004 → T005 → T006 → T007。
