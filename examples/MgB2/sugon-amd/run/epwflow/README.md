# MgB₂ EPW 手动测试

0046～0051 的合并版本已重新编译并完成联合验收。独立测试现场从 SCF 走通八阶段，iso／aniso 各一次 EPW 执行 20、30、40 K；结果与限制见 [联合验收记录](../../../../../docs/specs/0050-epw-stage-results/real-chain-validation.md)。本目录已清理历史运行文件，保留输入与说明，可从第 1 步开始手动测试。

## 开始前检查

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"
command -v epw_phonon epw_wannier epw_superconductivity
epw_phonon -c "epwinput.toml" --check --json
```

`--check` 只检查配置、文件和并行设置，不提交作业。所有命令读取当前 `epwinput.toml`，结构为 `../POSCAR`，赝势位于上级 `pp/`。当前为 PBE 小网格功能测试，图与数值供人工检查，不代表网格或选窗已经收敛。

## 八阶段操作

按下表逐步运行；确认上一阶段完成后再执行下一阶段。公开命令不会自动提交下一阶段。

| 步骤 | 提交命令 | 完成后查看 |
| --- | --- | --- |
| 1 SCF | `epw_phonon -c "epwinput.toml" --stage scf -s` | `1.qe-scf/` |
| 2 NSCF | `epw_phonon -c "epwinput.toml" --stage nscf -s` | `2.qe-nscf/` |
| 3 QE 电子 | `epw_phonon -c "epwinput.toml" --stage electronic -s` | `results/3.qe-electronic/` 单独能带与 DOS |
| 4 QE 声子 | `epw_phonon -c "epwinput.toml" --stage phonon -s` | `results/4.qe-phonon-nosplit/` 声子谱、TDOS、Mg／B 元素 DOS |
| 5 Wannier | `epw_wannier -c "epwinput.toml" --stage wannier -s` | `results/5.epw-wannier/` QE／EPW 电子能带及 DOS 对比 |
| 6 coarse | `epw_wannier -c "epwinput.toml" --stage coarse -s` | `results/6.epw-coarse/` 声子对比、四面板总览及 `decay/` 衰减图 |
| 7 fine | `epw_superconductivity -c "epwinput.toml" --stage fine -s` | `7.epw-fine/` 原生 α²F／λ 与清单 |
| 8 gap | `epw_superconductivity -c "epwinput.toml" --stage superconductivity -s` | `results/8.epw-superconductivity/` 能隙、Z、温度曲线与数据 |

**第 1／2 步**作业完成后，再执行相同命令核验完成记录；已完成的科学计算不会重跑。

**第 3～7 步**各提交一次即可，阶段脚本在作业内完成必要后处理、核验、绘图及发布。第 3 步能带和 DOS 各自完成即可出图；第 7 步没有另加独立最终图，已有谱在第 8 步结果包展示。无需再次调用公开命令触发绘图。

**第 8 步**当前 `task="gap"`、`mode="both"`、`mu_star=0.05`、温度 `[20,30,40]` K；执行一次命令提交两个独立方法作业，每个方法一次 EPW 处理全部温度。输出目录为 `iso-mu0.05-20-40/` 与 `aniso-mu0.05-20-40/`，全部有效温度齐备后在作业内发布最终结果。当前任务计算指定温度的能隙，不搜索 Tc。

为了尽早检查选窗，也可按 **1 → 2 → 3 → 5 → 4 → 6 → 7 → 8** 执行。第 5 步不依赖声子；能带由 EPW 内嵌 Wannier90 的 wdata 输出，只有电子 DOS 调用 postw90。图中四个窗口边界取实际拟合日志。第 6 步复用第 5 步电子数据，只新增必要的声子求值；声子比较只画 TDOS，元素 DOS 在第 4 步单独交付。

## 查看状态与处理失败

```bash
sacct -j 作业号 --format=JobID,State,ExitCode,Elapsed
epw_phonon -c "epwinput.toml" --status --json
epw_superconductivity --results-check --results "results" --json
```

`COMPLETED` 和退出码用于核对调度状态；还应检查阶段科学完成记录、根结果包及第 8 步各温度有效性。当前父脚本和合并日志位于对应阶段，如 `4.qe-phonon-nosplit/phonon.sh` 与 `slurm-phonon.log`；原生程序日志保留在实际任务目录。`results/index.json` 只导航已发布图片与数据，不代替科学完成记录。

失败后先看日志，修正原因，再对同一目标加 `--resume -s`。成功科学检查点与相容后处理会复用；缺图或发布失败只补派生结果。活动或终态未知的作业拒绝重复提交，不手删状态规避核验。必要计算原因在比较目录的 `reuse-decision.json`／`dos-decision.json` 中。

完整 `results/` 可移动到其他目录独立重绘：

```bash
epw_superconductivity --plot --results "results"
epw_superconductivity --results-check --results "results" --json
```

## 输入与目录

当前 SCF／NSCF 为 4×4×2、NSCF 16 带，PH 为 2×2×1，细 k／q 为 8×8×4／4×4×2，5 个 Wannier 函数；`mpirun -np 32`、MPI=32、EPW `-nk 32` 保持一致。超导迭代使用输入中的实际混合设置 `mixing_beta=-0.3`、历史长度 8，最多 1000 次、相对标准 1e-8；不自动调窗、扫描网格或修改温度。

阶段数据在本目录按 `1.qe-scf/` 至 `8.epw-superconductivity/` 生成，阶段状态在 `epw-state/`，最终图片与对应数据按阶段发布到同级 `results/`。原生科学文件继续保留在各阶段，续算时不要删除。

共享预算沿用 `docs/local/epwflow-campaign.json`，累计上限 28800 节点秒／100 GiB，同时最多两个作业。新建现场不会清零历史消费；预留不足时由程序明确拒绝。旧 examples 现场已归档到 `test_auto/MgB2/local_run/012-stage-results/example-before-manual/`，不作为本目录当前完成证据。

[图片与数据说明](RESULTS.md) · [测试脚本与复现](../../../../../test_auto/MgB2/epwflow/README.md) · [EPW 手册](../../../../../epwflow/README.md)
