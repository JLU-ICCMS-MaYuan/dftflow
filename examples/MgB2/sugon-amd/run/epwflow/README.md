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

按以下步骤逐步运行；确认上一阶段完成后再执行下一阶段。公开命令不会自动提交下一阶段。

### 第 1 步：SCF

执行自洽计算，生成后续阶段所需的电子基态。

```bash
epw_phonon -c "epwinput.toml" --stage scf -s
```

完成后查看 `1.qe-scf/`。

### 第 2 步：NSCF

执行非自洽计算，准备 Wannier 化所需的电子态。

```bash
epw_phonon -c "epwinput.toml" --stage nscf -s
```

完成后查看 `2.qe-nscf/`。

### 第 3 步：QE 电子

计算 QE 电子能带与 DOS，供独立查看和后续比较。

```bash
epw_phonon -c "epwinput.toml" --stage electronic -s
```

完成后在 `results/3.qe-electronic/` 查看 4 张单独 QE 图：两种范围的能带、TDOS＋Mg／B 元素 PDOS。

### 第 4 步：QE 声子

计算声子及其后处理结果。

```bash
epw_phonon -c "epwinput.toml" --stage phonon -s
```

完成后在 `results/4.qe-phonon-nosplit/` 查看声子谱、TDOS、Mg／B 元素 DOS。

### 第 5 步：Wannier

执行 Wannier 化并生成电子比较图，用于检查拟合与选窗。

```bash
epw_wannier -c "epwinput.toml" --stage wannier -s
```

完成后在 `results/5.epw-wannier/` 查看 4 张 QE／EPW 电子能带及总 DOS 对比图。

### 第 6 步：coarse

处理粗网格电声数据，生成声子比较与实空间衰减图。

```bash
epw_wannier -c "epwinput.toml" --stage coarse -s
```

完成后在 `results/6.epw-coarse/` 查看声子对比、四面板总览，在其中的 `decay/` 查看衰减图。

### 第 7 步：fine

执行细网格插值，生成原生 α²F／λ 数据。

```bash
epw_superconductivity -c "epwinput.toml" --stage fine -s
```

完成后在 `7.epw-fine/` 查看原生 α²F／λ 与清单。

### 第 8 步：gap

按配置中的方法与温度计算超导能隙。

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
```

完成后在 `results/8.epw-superconductivity/` 查看能隙、Z、温度曲线与数据。

### 阶段完成与执行顺序

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

0053 新版 CLI 生成的科学命令使用 `> xxx.out 2>&1`，例如 SCF 的正常输出和错误都在 `1.qe-scf/scf.out`，Slurm 日志设置保持原样。已有版本 1／2 任务仍读取其原分离日志；不要删除旧 `.err` 或修改已绑定的脚本。仓库与默认 EPW 入口已更新，本目录已实际完成一次新格式 SCF，见 [0053 验收](../../../../../docs/specs/0053-epw-merged-program-output/acceptance.md)。

需要重跑 SCF 并重新生成 `scf.sh` 时，无需删除 `1.qe-scf` 或 `completion.json`：

```bash
epw_phonon -c "epwinput.toml" --stage scf --resume -s
```

每次执行都会强制开始新批次，旧脚本及结果完整归档到 `epw-state/history/`；运行中或状态未知时拒绝重提。作业结束后，去掉 `--resume` 归集结果并保留后续复用：

```bash
epw_phonon -c "epwinput.toml" --stage scf -s
```

失败后先看日志，修正原因，再对同一目标加 `--resume -s`。成功科学检查点与相容后处理会复用；缺图或发布失败只补派生结果。活动或终态未知的作业拒绝重复提交，不手删状态规避核验。必要计算原因在比较目录的 `reuse-decision.json`／`dos-decision.json` 中。

## 重新绘制已有图片

在本目录执行以下命令，可以重新绘制第 3 步已发布的 QE 电子图：

```bash
epw_phonon --plot --results "./results"
epw_phonon --results-check --results "./results" --json
```

重绘读取结果包内已有数值和绘图参数，不加 `-s`，不提交 Slurm 作业，也不重新运行 QE／EPW。完整 `results/` 移动到其他目录后同样可以独立重绘。

第 3 步重绘后的四张图保存在：

```text
results/3.qe-electronic/
├── qe_eband/
│   ├── electronic-bands.png
│   └── electronic-bands-valence.png
└── qe_eledos/
    ├── electronic-dos.png
    └── electronic-dos-valence.png
```

两种能量范围见[电子图怎么看](#电子图怎么看)。当前命令会重绘整个 `results/` 中已有的阶段图片，尚不支持只选择第 3 步；重绘更新的是结果包中的图片。

前提是已有 `results/index.json` 和配套 `.plot.json`。旧结果缺少元素 PDOS 时，单纯重绘不会补算投影。

## 输入与目录

当前 SCF／NSCF 为 4×4×2、NSCF 16 带，PH 为 2×2×1，细 k／q 为 8×8×4／4×4×2，5 个 Wannier 函数；MPI 启动器为 `mpirun`，其 `-np 32`、MPI=32、EPW `-nk 32` 保持一致。超导迭代使用输入中的实际混合设置 `mixing_beta=-0.3`、历史长度 8，最多 1000 次、相对标准 1e-8；不自动调窗、扫描网格或修改温度。

阶段数据在本目录按 `1.qe-scf/` 至 `8.epw-superconductivity/` 生成，阶段状态在 `epw-state/`，最终图片与对应数据按阶段发布到同级 `results/`。原生科学文件继续保留在各阶段，续算时不要删除。

共享预算沿用 `docs/local/epwflow-campaign.json`，累计上限 28800 节点秒／100 GiB，同时最多两个作业。新建现场不会清零历史消费；预留不足时由程序明确拒绝。旧 examples 现场已归档到 `test_auto/MgB2/local_run/012-stage-results/example-before-manual/`，不作为本目录当前完成证据。

[图片与数据说明](RESULTS.md) · [测试脚本与复现](../../../../../test_auto/MgB2/epwflow/README.md) · [EPW 手册](../../../../../epwflow/README.md)

## 电子图怎么看

第 3／5 步各 4 张，共 8 张。默认文件 `electronic-bands.png`、`electronic-dos.png` 看费米能上下 5 eV；带 `-valence` 的文件看最低已计算 QE 占据态到费米能以上 5 eV。同一种预设的能带和 DOS 使用相同范围。

红线是费米能（E−E_F＝0），蓝线只表示 EPW／Wannier 插值。QE 是深灰线，Mg／B 投影与选窗边界使用其他颜色。每张 PNG 为 3000×2100、至少 300 DPI，配套数据在同目录，可直接独立重绘。第 3 步若缺原子投影，会从已有 NSCF 波函数副本补一次 projwfc；已完成的相容投影不重复计算。第 5 步 DOS 不画元素投影。详细验证见 [0055](../../../../../docs/specs/0055-electronic-plot-windows-pdos/acceptance.md)。
