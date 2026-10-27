# MgB₂ EPW 手动测试

本目录采用按阶段填写原生参数的版本 2 配置。2026-10-28 已用全仓库重新编译发布的新 CLI，清空旧计算／状态／结果后，从 SCF 完成八阶段重算；iso／aniso 的 20／30／40 K 六点均收敛且数据有效，结果来源与图表检查通过。当前结果属于 [0064 验收](../../../../../docs/specs/0064-epw-readable-state-logs/acceptance.md)；旧现场未归档保留，[0059 验收](../../../../../docs/specs/0059-epw-native-stage-config/acceptance.md) 仅作历史证据。

当前为小网格功能测试，比较图仍有明显数值偏差，不是 Tc 或网格收敛验收。公开命令重复核验上游数据仍较慢：本轮完整流程约 49 分钟，科学分配合计仅 228 节点秒，具体限制见 0064。

## 开始前检查

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"
command -v epw_phonon epw_wannier epw_superconductivity
epw_phonon -c "epwinput.toml" --check --json
```

2026-10-28 已通过全工作区 release 构建发布当前 CLI，`scripts/env.sh` 指向仓库 `bin/`。0064 的真实重算与八阶段日志验收见 [0064 验收记录](../../../../../docs/specs/0064-epw-readable-state-logs/acceptance.md)。配置原文在内容变化后的首次 CLI 调用保存为下一份 `NNNN-config.toml`，不再生成参数来源 JSON。配置格式见 [0061](../../../../../docs/specs/0061-epw-flat-stage-config/spec.md)。

`--check` 只检查配置、文件和并行设置，不提交作业。所有命令读取当前 `epwinput.toml`，结构为 `../POSCAR`，赝势位于上级 `pp/`。当前为 PBE 小网格功能测试，图与数值供人工检查，不代表网格或选窗已经收敛。

## 按阶段修改配置

当前 `epwinput.toml` 仅使用 `schema_version = 2`。例如 SCF 的截断能在 `stages."1.qe-scf".ecutwfc`，NSCF 在 `stages."2.qe-nscf".ecutwfc`；两处分别填写，单位均为 Ry。PH 初始化和逐 q 计算、电子后处理以及两个超导方法也分别配置。第 7 步 `temps=[20]` K 独立于第 8 步的 20／30／40 K。

科学段内不再细分 pw、SYSTEM 或 ELECTRONS。能带 PW、bands.x、DOS、PDOS 分别使用 `3.qe-bands/3.qe-bands-post/3.qe-dos/3.qe-pdos`。自动绑定项不写入 TOML，由程序内部检查。自动选窗、输入来源及调度保持在独立工作流配置；科学输入里的数组和卡片保留原生名称，不能再填写旧别名。完整对应关系见[参数说明](../../../../../docs/overview/epwflow/native-stage-config.md)。

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

完成后在 `results/4.qe-phonon-coords/` 查看声子谱、TDOS、Mg／B 元素 DOS。

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

**第 3／4 步**在阶段作业内完成计算、后处理和发布；结束后重复相同命令核验并复用。**第 5～7 步**需要在作业结束后重复相同公开命令，核验科学输出并推进必要后处理与发布；如果又提交了后处理作业，待其结束再重复，直到提示阶段完成。第 7 步没有另加独立最终图，已有谱在第 8 步结果包展示。

**第 8 步**当前工作流 `task="gap"`、`mode="both"`；`stages."8.epw-iso"` 与 `stages."8.epw-aniso"` 分别显式设置 `muc=0.05`、`nstemp=3`、`temps=[20,30,40]` K；执行一次命令提交两个独立方法作业，每个方法一次 EPW 处理全部温度。输出目录为 `iso-mu0.05-20-40/` 与 `aniso-mu0.05-20-40/`，两个作业结束后重复同一公开命令，检查全部温度并发布最终结果。当前任务计算指定温度的能隙，不搜索 Tc。

为了尽早检查选窗，也可按 **1 → 2 → 3 → 5 → 4 → 6 → 7 → 8** 执行。第 5 步不依赖声子；能带由 EPW 内嵌 Wannier90 的 wdata 输出，只有电子 DOS 调用 postw90。图中四个窗口边界取实际拟合日志。第 6 步复用第 5 步电子数据，只新增必要的声子求值；声子比较只画 TDOS，元素 DOS 在第 4 步单独交付。

## 查看状态与处理失败

```bash
sacct -j 作业号 --format=JobID,State,ExitCode,Elapsed
epw_phonon -c "epwinput.toml" --status --json
epw_superconductivity --results-check --results "results" --json
```

`COMPLETED` 和退出码用于核对调度状态；还应检查阶段科学完成记录、根结果包及第 8 步各温度有效性。当前父脚本和合并日志位于对应阶段，如 `4.qe-phonon-coords/phonon.sh` 与 `slurm-phonon.log`；原生程序日志保留在实际任务目录。`results/index.json` 只导航已发布图片与数据，不代替科学完成记录。

当前科学命令使用 `> xxx.out 2>&1`，例如 SCF 的正常输出和错误都在 `1.qe-scf/scf.out`，Slurm 日志单独保留。程序、输入和完成回执均以本轮新生成记录核验。

需要重跑 SCF 并重新生成 `scf.sh` 时，直接使用公开命令：

```bash
epw_phonon -c "epwinput.toml" --stage scf --resume -s
```

每次执行都会强制开始新批次，直接覆盖本阶段旧计算内容，保留阶段根目录及打开的 Slurm 日志；运行中或状态未知时拒绝重提。续算信息集中在 `epw-state/` 的八份中文阶段日志，任务号位于计算目录的同名 `.jobs`。配置原文内容变化时在输入旁新增 `NNNN-config.toml`。作业结束后，去掉 `--resume` 归集结果并保留后续复用：

```bash
epw_phonon -c "epwinput.toml" --stage scf -s
```

失败后先看日志，修正原因，再对同一目标加 `--resume -s`。成功科学检查点与相容后处理会复用；缺图或发布失败只补派生结果。活动或终态未知的作业拒绝重复提交，不手删状态规避核验。必要计算原因在比较目录的 `reuse-decision.json`／`dos-decision.json` 中；续算子任务写入所属阶段日志，任务号从各阶段 `.jobs` 读取。

## 重新绘制已有图片

在本目录执行以下命令，可以重新绘制第 3 步已发布的 QE 电子图：

```bash
epw_wannier --plot --results "./results"
epw_wannier --results-check --results "./results" --json
```

重绘读取结果包内已有数值和绘图参数，不加 `-s`，不提交 Slurm 作业，也不重新运行 QE／EPW。完整 `results/` 移动到其他目录后同样可以独立重绘。

当前 `epw_phonon` 入口不开放 `--plot`／`--results-check`，因此统一使用 `epw_wannier` 处理结果包。这两个命令不要求第 5 步已完成，也不会启动 Wannier 计算；只有第 3 步结果时同样可用。

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

当前 SCF／NSCF 为 4×4×2、NSCF 16 带，PH 为 2×2×1，细 k／q 为 8×8×4／4×4×2，5 个 Wannier 函数；MPI 启动器为 `mpirun`，其 `-np 32`、MPI=32、EPW `-nk 32` 保持一致。超导迭代使用输入中的实际混合设置 `broyden_beta=-0.3`、历史长度 8，最多 1000 次、相对标准 1e-8；不自动调窗、扫描网格或修改温度。

阶段数据在本目录按 `1.qe-scf/` 至 `8.epw-superconductivity/` 生成，阶段状态在 `epw-state/`，最终图片与对应数据按阶段发布到同级 `results/`。原生科学文件继续保留在各阶段，续算时不要删除。

共享账本沿用 `docs/local/epwflow-campaign.json`，保留历史消费与最多两个作业的登记约束；时间和存储旧额度不再约束新任务。DFTFlow 不再额外限制计算时长和存储空间，集群自身规则仍由 Slurm 管理。源配置须删除 `resources.wall_seconds`、`resources.storage_bytes`；含旧字段会明确报错。旧冻结快照和已提交脚本不自动改写；本次已明确执行构建发布，平时修改源码后仍需重新构建，见 [0058 验收](../../../../../docs/specs/0058-epw-remove-resource-limits/acceptance.md)。

[图片与数据导航](results/index.json) · [测试脚本与复现](../../../../../test_auto/MgB2/epwflow/README.md) · [EPW 手册](../../../../../epwflow/README.md)

## 电子图怎么看

第 3／5 步各 4 张，共 8 张。默认文件 `electronic-bands.png`、`electronic-dos.png` 在第 3 步看费米能上下 5 eV，第 5 步看实际 `dis_win_min−E_F` 到 `dis_win_max−E_F`；带 `-valence` 的文件看最低已计算 QE 占据态到费米能以上 5 eV。同一种预设的能带和 DOS 使用相同范围。QE 参考为深灰实线，EPW／Wannier 为蓝色虚线，费米线为红色。

红线是费米能（E−E_F＝0），蓝线只表示 EPW／Wannier 插值。QE 是深灰线，Mg／B 投影与选窗边界使用其他颜色。每张 PNG 为 3000×2100、至少 300 DPI，配套数据在同目录，可直接独立重绘。第 3 步若缺原子投影，会从已有 NSCF 波函数副本补一次 projwfc；已完成的相容投影不重复计算。第 5 步 DOS 不画元素投影。详细验证见 [0055](../../../../../docs/specs/0055-electronic-plot-windows-pdos/acceptance.md)。
