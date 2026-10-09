# 实施计划

Issue：[IKHSA0](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHSA0)；规格：[spec.md](spec.md)。

## 技术与边界

沿用 Python、pytest、NumPy。只在 `qeflow/input_deck/matdyn.py` 的声子谱 namelist 写出 `q_in_cryst_coord = .true.`。不改模板配置、KPATH 转换、DOS writer、CLI、绘图或工作流；无新数据模型。

## 顺序

1. 创建 Issue、Spec、契约、需求检查与任务，执行覆盖检查。
2. 在 `test_auto/MgB2/test_matdyn_crystal_coordinates.py` 增加不等轴正交和斜晶胞测试；实际调用路径与输入生成函数，不 mock 掉被测逻辑。
3. 观察旧实现因缺声明失败；新增一行声明后运行回归。
4. 更新 qeflow README、绘图 Overview 的输入生成边界、项目 skill、AGENTS；记录验收并精确提交。

## 映射与质量门

| 来源 | 设计与验证 | 任务 |
| --- | --- | --- |
| FR-001 / SC-001 / US1 | namelist 声明；两类非立方晶胞，插值开关两种值 | T002、T003 |
| FR-002 / US1 | 逐项比对坐标、权重、标签索引和倒格子距离 | T002、T003 |
| FR-003 / SC-002、SC-003 | 固定 DOS 输入契约，旧测试及生产 diff 边界 | T002–T004 |
| FR-004 | 文档、验收与 Issue 回写 | T001、T004 |

用户已确认范围与 Python 修复；无未决设计项，未引入额外复杂度。需求检查表全部完成后才能实现。

测试命令见 [quickstart.md](quickstart.md)；技术依据见 [research.md](research.md)；接口见 [contracts/input.md](contracts/input.md)。
