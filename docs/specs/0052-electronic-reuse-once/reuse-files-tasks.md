# 按实际文件接续：技术任务草案

输入：[需求](reuse-files-spec.md)、[计划](reuse-files-plan.md)。以下均未实施；不作为线上协作状态副本。

- [ ] T101：恢复第 1～8 步逐项讨论，确认文件用途、来源、直接检查和输出，确定快照处理；写入本目录配套交接说明。阻断全部依赖精确文件范围的代码修改。
- [ ] T102（US1）：在 epwflow/src/foundation/stages.rs、native_workspace.rs 移除全祖先展开／递归复查和无关刷新，缺直接输入停止；FR-001/002/004 → SC-001。
- [ ] T103（US1）：在 qe_data/save.rs、direct.rs 与 interpolation/{manifest,adapter,direct,scheduler,restart}.rs 解除清单／指纹／登记的持久和内存依赖，检查全部消费者，不返回伪成功；FR-005/008 → SC-003/005。
- [ ] T104（US3）：在 interpolation/{epw_config,coarse,scheduler}.rs 移除后续窗口复查和强制上游费米能／本征值加载，保留 Wannier 化与显式原生设置；FR-006/007 → SC-004。
- [ ] T105（US2）：在 foundation/readable_state.rs 和文件交接点输出实际来源分组与完整路径，7 空格续行；FR-003 → SC-002。
- [ ] T106（US1/US2/US3）：在 epwflow/tests/ 的相关公开进程／原生阶段回归中覆盖无关旧阶段不可用、缺直接输入、过期清单、真实操作与输出一致、费米能与窗口、调度和失败边界；FR-001～008 → SC-001～005。
- [ ] T107：运行相关回归，release 编译并覆盖固定模块命令，不留版本副本；不启动未授权科学计算。
- [ ] T108：使用 overview-maintainer 回写当前已落地行为及验证边界，补充验收记录；仅提交本任务变更，不推送，不自动关闭或更新线上 Issue。

顺序：T101 与完整草案确认、明确实施授权在前；T102～T105 按消费者依赖协调，T106 验证所有修改，T107 成功后 T108。旧 T001～T004 的完成记录只对应历史实现，不视为上述新任务完成。
