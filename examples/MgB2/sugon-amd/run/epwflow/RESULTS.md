# MgB₂ 的逐阶段图片与数据

以下对应 0050 的新版源码。默认安装、已经生成的冻结脚本和现有计算目录不会随源码提交自动更新；本次未部署或改动真实计算数据。

先将计算目标设为第 5 步：

```bash
epw_wannier -c "epwinput.toml" --stage wannier -s
```

正常作业完成后，直接查看 `results/5.epw-wannier/`：`electronic-bands.png`、`electronic-dos.png` 及对应 CSV、绘图参数、采样和来源清单。电子能带来自 EPW 内嵌 Wannier90 的 wdata 输出；只有 DOS 使用 postw90。该目标不等待第 4 步声子，也不自动提交第 6 步。

电子图显示本次实际四个能窗边界，可结合最大差、未配对态和 DOS 的重合程度判断选窗。它不自动给出质量评级，也不证明粗网格已收敛。

需要声子比较时：

```bash
epw_wannier -c "epwinput.toml" --stage coarse -s
```

作业内部完成必要声子后处理并发布到 `results/6.epw-coarse/`，图在 `figures/`，自包含比较表在 `comparison/`；同时复用第 5 步电子数据生成四面板总览。QE 参考不重复做电子 PW；声子参考只用当前配置所选模式。必要 q2r／matdyn 与 EPW 插值求值在执行前记录原因，成功结果缓存复用。第 6 步声子 DOS 仅比较 TDOS。

其他结果位置：

| 目录 | 已有图片与配套数据 |
| --- | --- |
| `results/3.qe-electronic/qe_eband/` | QE 单独电子能带，完成即发布 |
| `results/3.qe-electronic/qe_eledos/` | QE 单独电子 DOS，完成即发布 |
| `results/4.qe-phonon-<模式>/` | QE 单独声子谱、TDOS、元素 DOS 和 CSV |
| `results/5.epw-wannier/` | 两张电子比较图、窗口、差值和来源 |
| `results/6.epw-coarse/` | 声子比较及四面板总览 |
| `results/8.epw-superconductivity/` | 当前求解模式已有的超导图与数据 |

第 7 步当前没有独立最终图，不额外增加绘图任务。`results/index.json` 为已发布单元导航；它不代替阶段的科学完成回执。

正常作业无需第二次公开命令来触发出图。出错时先检查日志和 `reuse-decision.json`／`dos-decision.json`；修复后用相同目标加 `--resume -s`。图片失败保留科学检查点，已成功的拟合、PW／PH 和相容后处理不会因为补图重复运行。活动作业或终态未知时拒绝重提。

```bash
epw_superconductivity -c "epwinput.toml" --status --json
epw_superconductivity --results-check --results "results" --json
epw_superconductivity --plot --results "results"
```

完整 `results/` 可移动后独立重绘，不调用 QE／EPW 或 Slurm。包内完整性核验与原始来源是否仍可访问是两件事。旧工作区只从可核验的旧采样整理派生结果；缺少真实点位或来源时明确报缺项，不能为目录更名重新拟合。

[流程图与数据来源](../../../../../docs/overview/epwflow/workflow.md) · [0050 验收记录](../../../../../docs/specs/0050-epw-stage-results/acceptance.md)。

## 第 6 步实空间衰减图（0051）

0051 新版源码在上述粗网格目标的原生核验成功后，先独立发布 `results/6.epw-coarse/decay/`，声子比较尚未完成或失败也不阻塞这组图。阶段内同份派生结果位于 `6.epw-coarse/decay/`；既有冻结脚本和默认安装需另行更新，提交源码不会自动补当前算例目录。

先看 `decay-overview.png`：左上 Hamiltonian、右上动力学矩阵、左下电子方向电声矩阵元、右下声子方向电声矩阵元。四张独立图分别为 `decay-H`、`decay-dynmat`、`decay-epmate`、`decay-epmatp`，每张及总览均有 PNG／SVG／PDF。坐标按原生单位 Å／Ry，紫色散点、对数纵轴，不合并相同距离，也不给出自动收敛结论。

包内包含本次回执绑定的四份原始副本和绘图数据、参数、来源。可移动完整 `results/` 后用前文命令重绘；缺图恢复不重跑科学，缺原生文件明确列项而不重算 EPW。本轮已将该算例现有数据只读出图到独立测试目录，未修改算例科学文件，见 [0051 验收与实图位置](../../../../../docs/specs/0051-epw-real-space-decay/acceptance.md)。
