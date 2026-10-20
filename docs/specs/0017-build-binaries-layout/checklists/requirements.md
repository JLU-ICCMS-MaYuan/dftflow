# 需求质量检查表

- [x] CHK001 是否逐字保存用户原始需求并将范围澄清单独记录？证据：`spec.md`「原始需求记录」包含需求原话与用户答复；`issue-draft.md` 保存同一记录。
- [x] CHK002 是否明确公开交付程序与测试 harness 的边界？证据：`spec.md` FR-003、FR-005、范围外事项；用户确认仅 DFTFlow 自身产品。
- [x] CHK003 是否定义所有家族与程序映射？证据：`spec.md` FR-002/003、`research.md`「程序映射基线」。
- [x] CHK004 是否覆盖构建、安装、测试和发布边界？证据：`spec.md` FR-001/004/005，`plan.md` 质量门与映射表。
- [x] CHK005 是否有可量化且可观测的成功标准？证据：`spec.md` SC-001～005 定义路径、可执行权限、启动、二次构建和打包验证。
- [x] CHK006 是否说明现有能力并避免重复建设？证据：`research.md` 记录 `install-bin.sh` 已有家族映射、兼容链接、入口验证与隔离安装回归。
- [x] CHK007 是否处理迁移测试程序可能被作业/脚本引用的风险？证据：`spec.md` FR-006，`research.md` 决策 3，Tasks T002/T009。
- [x] CHK008 FR/SC 是否映射到实施任务？证据：`tasks.md`「需求覆盖」表逐项映射 FR/SC。
- [x] CHK009 是否区分本地 Spec 与线上 Issue 状态？证据：`issue-draft.md`、`spec.md` 明确 Gitee Issue 未创建，未声称远程链接验证完成。
- [ ] CHK010 是否有真实构建、测试、安装和 Linux 发布结果证明实现通过？[实施门槛，SC-001～005；US3 已实现，其余构建/迁移未完成]
- [ ] CHK011 线上 Issue 是否已创建且远程 Issue→Spec 双向链接已验证？[外部协作缺口；当前环境没有 Gitee CLI/连接器，待可用时处理]
- [x] CHK012 是否逐字保留追加 PATH 需求并覆盖家族目录、重复加载和持久生效？证据：`spec.md` 原话、US3、FR-009/010、SC-006/007；`research.md` 决策 4；`tasks.md` T016/T017。
- [x] CHK013 是否提供明确的手动配置及作用边界？证据：`quickstart.md`「从任意目录使用命令」给出七家族配置、Bash/Zsh 启动文件、命令来源检查和批处理边界，不自动修改用户环境。
- [x] CHK014 PATH 入口是否经过真实 shell 验证并说明证据边界？证据：`scripts/tests/env.sh` 与 `acceptance.md` 记录 Bash/Zsh 各 55 入口、重复加载、含空格的仓库位置、原路径保留、临时 HOME 新终端加载；没有修改真实启动文件。
