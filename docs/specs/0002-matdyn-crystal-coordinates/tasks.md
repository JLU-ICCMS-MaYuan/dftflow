# 实施任务

输入：[spec.md](spec.md)、[plan.md](plan.md)、[research.md](research.md)、[契约](contracts/input.md)。

- [x] T001 建立 Issue IKHSA0 与本目录规格、计划、需求检查；确认 FR/SC 覆盖无缺项。
- [x] T002 [US1] 在 `test_auto/MgB2/test_matdyn_crystal_coordinates.py` 增加非立方路径和 DOS 输入契约测试，记录修复前失败。
- [x] T003 [US1] 修改 `qeflow/input_deck/matdyn.py` 的声子谱坐标声明，运行新增与已有声子回归。
- [x] T004 更新 `qeflow/README.md`、`docs/overview/qe-phonon-plot.md`、`skills/qe-phonon-plot/SKILL.md`、`AGENTS.md` 及本目录 `acceptance.md`，精确提交并回写 Issue。

T001 → T002 → T003 → T004，单故事串行实施，无并行必要。覆盖关系见 plan；未推送状态独立记录，Issue 不自动关闭。
