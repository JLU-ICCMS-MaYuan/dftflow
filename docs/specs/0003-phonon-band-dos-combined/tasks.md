# 实施任务：联合声子图

输入：[spec.md](spec.md)、[plan.md](plan.md)、[research.md](research.md)、[契约](contracts/plot.md)。

## 准备

- [x] T001 创建并核验 Issue 0003，完成本目录 Spec/Plan/Research/Contracts 与需求检查表。

## US1：共享频率轴 JPG（P1，MVP）

- [x] T002 [US1] 在 `test_auto/MgB2/test_phonon_combined_plot.py` 编写并运行失败测试，覆盖 JPEG、共享轴、曲线数值、双范围负频、原输出、原状态、CLI、缺文件、保存失败释放图。
- [x] T003 [US1] 修改 `qeflow/phonon/phonon_plot.py`，提取共用轴绘制函数并增加联合 JPG，保持读取和旧输出不变。
- [x] T004 [US1] 运行新增测试及 `test_auto/MgB2/test_qe_phonon.py`，核验实际归档绘图和输入哈希。

## 完善与交付

- [x] T005 更新 `README.md`、`qeflow/README.md`、`docs/overview/qe-phonon-plot.md`、`skills/qe-phonon-plot/SKILL.md`、`AGENTS.md`。
- [x] T006 在本目录记录验收结果，精确范围提交；回写 Gitee Issue 验证结果与未推送边界。

依赖：T001 → T002 → T003 → T004 → T005 → T006；同一绘图模块串行处理。

| 来源 | 覆盖任务 |
| --- | --- |
| FR-001、FR-002、SC-001、SC-002 | T002–T004 |
| FR-003、FR-005、SC-004 | T002–T004 |
| FR-004、SC-003 | T002–T004 |
| FR-006 | T002、T005–T006 |

当前仅一个用户故事，无额外增量或并行开发需要。
