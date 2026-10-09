# 实施计划：声子谱与态密度联合绘图

**Issue**：0003（Gitee：[IKHQNI](https://gitee.com/mayuan_JLUPHY/dftflow/issues/IKHQNI)）
**日期**：2026-09-22；**Spec**：[spec.md](spec.md)

## 目标与结构

只扩展 `qeflow/phonon/phonon_plot.py`：保持数据表构建与写出不变，在现有两张图之后新增联合图。
从独立图中提取两个小的轴绘制函数，联合图复用它们，避免复制曲线和标签处理。
`plt.subplots(1, 2, sharey=True, gridspec_kw={"width_ratios": [3, 1]})` 建立真正的共享轴。
先绘两类数据，再基于两类频率最小/最大值设置共同范围和少量边距；负频由原值保留。
输出 JPG 使用 300 dpi、白底；新图放在工作区根目录，语义上同时属于 band 和 DOS。

## 技术上下文

- Python 包声明 >=3.8；验证使用已有 Python 3.12、pytest、numpy、matplotlib/Pillow。
- 测试：`test_auto/MgB2/test_phonon_combined_plot.py` 新增单元与 CLI 集成测试；已有 `test_qe_phonon.py` 回归。
- 无数据库或新持久业务实体；仅新增可重复生成的图片，因此不新建 data-model.md。
- 对外输出契约：[contracts/plot.md](contracts/plot.md)。
- 功能规模：一张图片，一处绘图模块；不改变工作流状态机或归档检查器。

## 质量门

| 来源 | 要求 | 处理 |
| --- | --- | --- |
| 用户确认 | 不新增 CLI，不做大改动 | 原命令默认额外写图 |
| AGENTS 语言规则 | 本次沿用 Python 的明确例外 | 用户确认已记录于 Spec |
| FR-004 | 历史输出及状态兼容 | 不把 JPG 加入历史完成状态的必需文件集 |
| TDD | 先失败测试后实现 | 验证测试因联合图缺失而失败 |
| 数据安全 | 输入不可改动 | 单元/真实应用核验哈希 |

## 需求映射与实施顺序

1. 文档审查：FR-001–006、SC-001–004 全部映射到 [tasks.md](tasks.md)。
2. 先编写失败测试：JPG、共享轴、原曲线、负频范围、保留 PNG/CSV、缺文件、保存失败释放图对象、旧状态兼容。
3. 实现两个轴绘制 helper 与 `_plot_band_dos`；在 `plot_phonon_outputs` 末尾调用。
4. 运行聚焦测试和已有 QE 回归；检查 CLI `--help` 未引入参数。
5. 更新 `README.md`、`qeflow/README.md`、`docs/overview/qe-phonon-plot.md`、`skills/qe-phonon-plot/SKILL.md`、`AGENTS.md`。
6. 在独立工作区运行真实归档，生成联合 JPG，验证输入哈希与原频率/DOS 不变。
7. 精确暂存本功能文件并提交；Issue 回写本地验证与未推送状态，不把未推送文件称为远端可用。

## 验证命令

在仓库根目录运行（旧回归中的导入边界检查使用仓库相对路径）：

```bash
cd "/home/mayuan/code/dftflow"
PYTHONPATH="/home/mayuan/code/dftflow" "/home/mayuan/miniconda3/bin/python" -m pytest \
  "/home/mayuan/code/dftflow/test_auto/MgB2/test_phonon_combined_plot.py" \
  "/home/mayuan/code/dftflow/test_auto/MgB2/test_qe_phonon.py" -q
```

第一轮新增测试应因缺少联合绘图函数或文件失败；实现后应全部通过。
