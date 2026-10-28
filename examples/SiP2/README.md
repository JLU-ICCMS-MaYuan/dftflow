# SiP₂：单命令导入历史声子并接续 Wannier

`epwinput.toml` 使用 schema 2 平层配置，与整合 0060/0061 的正式程序配套。`POSCAR` 从历史 SCF 输入保留原始晶格和 24 个原子的分数坐标；`KPATH.in` 沿用历史 Wannier 的 12 段路径。赝势使用配置中已有文件的绝对路径。

| 参数 | 当前设置 |
| --- | --- |
| SCF / NSCF 均匀网格 | 2×3×6，零位移；NSCF 完整 36 点 |
| 密度 / 平滑 FFT | 216×128×60 / 120×72×32，SCF 与 NSCF 显式一致 |
| 截断能 | 60 / 720 Ry |
| NSCF 能带 / Wannier 函数 | 96 / 96，Si:s,p、P:s,p（此前为 Si:sp3、P:sp3） |
| 细电子 / 细声子网格 | 8×12×24 / 4×6×12 |
| μ* / 温度 | 0.1 / 5、10、20 K |

当前 EPW 与 Wannier90 的迭代上限均为 500，投影为 `Si:s,p`、`P:s,p`。细网格、温度和 μ* 仍需收敛验证，费米能读取实际 NSCF。以下保留调整前的运行说明，其中 5000 轮已由当前 500 轮设置替代：

> 细网格、温度和 μ* 是用户确认的起始设置，尚未做收敛验证。历史 Wannier 的迭代上限为 500；当前配置采用流程的 5000 上限和收敛检查。费米能从新 NSCF 读取，不固定历史数值。其余数值迭代和后处理设置也需要后续核验。

## 编号目录与操作顺序

前三个电子计算阶段的说明保留在这里，供首次计算、查看结果和失败排查时查阅；后续声子导入、Wannier 接续和旧数据复制步骤继续保留在下文。**已有有效完成回执的阶段直接复用，首次计算示例不需要在当前目录重新执行。**

| 目录 | 用途 | 主要结果与后续关系 |
| --- | --- | --- |
| `1.qe-scf/` | 计算自洽电荷密度 | 为 NSCF、电子能带与历史声子兼容核验提供基础 |
| `2.qe-nscf/` | 在完整均匀 k 网格上计算电子态 | 为 DOS、Wannier 和 EPW 提供电子态及费米能 |
| `3.qe-electronic/` | 计算 QE 电子能带和 DOS，并独立出图 | `qe_eband/` 放能带，`qe_eledos/` 放 DOS；不依赖声子计算 |
| `4.qe-phonon-nosplit/` | 接入历史 PH，执行 q2r、matdyn 并出图 | 本例通过 `--import-phonon` 接续已有声子 |
| `5.epw-wannier/` | Wannier 拟合与本阶段交付 | 复用已验证的电子和声子数据 |

首次准备按 `SCF → NSCF` 顺序进行。之后可单独运行第 3 步查看电子结果；历史声子导入不要求先完成第 3 步。进入 Wannier 时，程序会补齐尚缺的电子参考。因此，目录编号不表示电子出图必须等待声子计算完成。

以下命令均在本例根目录执行：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
command -v epw_phonon
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 只读检查配置、输入和程序设置；configured 不代表计算已经完成。
epw_phonon --config "epwinput.toml" --stage nscf --check --json
```

### 1.qe-scf：自洽电子计算

本阶段使用 `pw.x` 的 `calculation='scf'`，得到自洽电荷密度和电子保存数据。当前使用零位移 `2×3×6` 网格、`60/720 Ry` 截断能，并显式固定密度和平滑 FFT。

仅在新工作区尚未完成 SCF 时执行：

```bash
# 首次提交 SCF。
epw_phonon --config "epwinput.toml" --stage scf -s
```

等待作业结束后，再执行普通命令校验产物并登记完成：

```bash
epw_phonon --config "epwinput.toml" --stage scf -s
test -s "1.qe-scf/completion.json"
```

重点查看 `1.qe-scf/scf.in`、`scf.out`、`slurm-scf.log`，以及 `1.qe-scf/out/Si8P16.save/` 中的 XML、电荷密度和波函数。`completion.json` 是工作流完成回执；仅有 `JOB DONE.` 或 Slurm 成功状态仍需完成工作流核验。

普通完成登记不加 `--resume`。显式 `--stage scf --resume -s` 会归档旧证据并强制启动新 SCF，不能把它当作“再查一次状态”的命令；当前已有 SCF 无需重算，也无需删除目录。

### 2.qe-nscf：完整均匀网格电子态

本阶段复用 SCF 电荷密度，使用 `pw.x` 的 `calculation='nscf'`，计算完整 `2×3×6=36` 个 k 点、每点 96 条能带。配置中的 `K_POINTS = [2, 3, 6]` 由 Rust 按 `kmesh.pl` 的顺序和数值格式展开到 `nscf.in`，无需手工列出 36 点。SCF 可通过对称性只计算不可约点，不能把它的点数直接当作 NSCF 的完整点数。

仅在 SCF 完成且尚未完成 NSCF 时执行：

```bash
# 首次提交 NSCF。
epw_phonon --config "epwinput.toml" --stage nscf -s
```

等待作业结束后，校验输入、输出的完整点数和网格，并登记完成：

```bash
epw_phonon --config "epwinput.toml" --stage nscf -s
test -s "2.qe-nscf/completion.json"
```

主要文件为 `2.qe-nscf/nscf.in`、`nscf.out`、`slurm-nscf.log`，以及 `2.qe-nscf/out/Si8P16.save/`。其中 `data-file-schema.xml` 保存实际 k 点、能带数、费米能及 FFT 信息，`wfc*.dat` 为后续使用的波函数；这些数据需要保留。

如果完整网格、FFT 或计算身份校验失败，先保留报错和现场，不删除回执强行接续。历史讨论中的 `calculation='bands'` 是显式改用 **`pw.x` 的运行模式**，不是运行后处理程序 `bands.x`，也不是第 3 步沿高对称路径的能带计算。当前使用 `nscf`，程序不会自动切换模式或重试。

### 3.qe-electronic：QE 电子能带与 DOS

这一步提供可以直接查看的 QE 电子结果。能带沿 `KPATH.in` 的高对称路径重新调用 `pw.x`，再用 `bands.x` 整理；DOS 用 `dos.x` 读取均匀网格电子态。NSCF 的 36 点用于均匀采样，不能直接替代高对称路径能带。

SCF、NSCF 已完成，且没有其他活动任务占用该工作区时，可单独提交：

```bash
# 一次提交完成电子参考、后处理、绘图及正式交付。
epw_phonon --config "epwinput.toml" --stage electronic -s
```

该阶段不调用 PH，不需要等待声子导入或 Wannier。也可以由下文的 Wannier 命令补齐；已有兼容任务正在运行时，应先等待该任务结束，避免重复提交。下表记录既有安装和现场的文件位置；0064 源码布局改为八份中文阶段日志与计算目录 `.jobs`，本轮没有转换这个真实现场。

| 位置 | 内容 |
| --- | --- |
| `3.qe-electronic/qe_eband/` | 能带输入、脚本、日志与后处理数据 |
| `3.qe-electronic/qe_eband/plots/electronic-bands.png` | QE 独立电子能带图，横轴标注高对称点 |
| `3.qe-electronic/qe_eledos/` | DOS 输入、脚本、日志与后处理数据 |
| `3.qe-electronic/qe_eledos/plots/electronic-dos.png` | QE 独立电子 DOS 图 |
| 两个子目录的 `plots/` | 相应的绘图数据 CSV 和绘图完成记录 |
| `3.qe-electronic/receipts/` | PW 能带、bands 和 DOS 的原生任务校验记录 |
| `3.qe-electronic/completion.json` | 整个电子阶段的完成回执 |
| `results/3.qe-electronic/` | 正式发布的电子阶段结果 |

能带和 DOS 以已核验 NSCF 的费米能为共同零点；每个电子子任务校验成功后即可生成自己的图，不需要先设置 Wannier 拟合能窗。后处理失败时保留已成功的 PW 电子数据，通过报错确定恢复步骤，不手动删除整个第 3 阶段。

### 电子计算的核查记录与当前状态

早先的 2026-10-27 核查记录为：NSCF 作业 `1375900` 的 Slurm 状态为 `COMPLETED`、退出码 `0:0`，`nscf.out` 有 `JOB DONE.`，XML 为 `nks=36`、`nbnd=96`；当时 NSCF 完成回执尚未登记。**这是历史检查时点，不能继续作为当前“尚未登记”的结论。**

本次补文档时再次只读检查，`1.qe-scf/completion.json` 与 `2.qe-nscf/completion.json` 均已存在；实际 XML 中，SCF 为 16 个不可约 k 点，NSCF 为 36 个完整 k 点、96 条能带，两者密度 FFT 均为 `216×128×60`，平滑 FFT 均为 `120×72×32`。这里只核对回执存在和 XML 数值，未重新读取全部波函数核验哈希，也不据此声明第 3 步或 Wannier 已完成。

日常只读查看流程状态可执行：

```bash
epw_phonon --config "epwinput.toml" --status --json
```

配置检查、科学日志、完成回执和结果图表分别回答不同问题，应结合查看。历史 FFT 与 EPW 报错的详细分析继续保留在 [SiP₂ 错误分析](SiP2-errors-analysis.md)；六个 FFT 参数一致不能单独证明历史外部程序问题已经修复。

## 已完成电子计算后的接续

2026-10-27，本目录已通过正式入口完成历史声子导入（作业 1379059，COMPLETED/0），声子谱、TDOS、Si/P 元素 DOS 和组合图已生成并查看。原 1074 个来源文件及 SCF/NSCF 的 126 项科学产物经完整哈希复核均保持不变。下面的导入命令可重复执行以核验并复用结果。

本目录的电子能带/DOS 参考也已完成。1379339 暴露的内嵌采样输入容量问题已修复并发布正式 CLI；恢复作业 1380018 使用原配置执行了首次真实 Wannier 拟合，**达到 5000 轮仍未收敛，流程已正确停止，未发布有效的 Wannier 阶段交付**。查看 `5.epw-wannier/Si8P16.wout` 与 `slurm-wannier.log`。无需重算声子、SCF、NSCF 或电子参考；继续拟合需要另行研究 Wannier 设置，本轮未修改或扫描参数，不能据此进入 EPC 或超导。

当前目录已有 SCF、NSCF 完成回执。不要重新提交这两项，也不要用默认声子计算替代导入。已复制到 external-phonon/save 的旧声子目录保持只读。

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
command -v epw_phonon
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 一次提交；作业内完成核验、转换、q2r、两次 matdyn、绘图与完成登记。
epw_phonon --config "epwinput.toml" --import-phonon "external-phonon/save" -s
```

正式命令应来自 /data/home/mayuan/code/dftflow/bin/epwflow，不再使用 docs/local 中的验证副本。源码中的新导入流程保留用户 TOML 原文，在配置旁生成递增的 0000-config.toml 等版本；旧嵌套配置仅在显式导入路径受控读取，不再改写用户输入或创建历史目录。外部来源与转换信息保存在 epw-state/imports/，续算进度写入八份中文阶段日志。本说明不表示当前安装和已有 SiP₂ 现场已经转换。

来源约 29 GiB，提交前和作业内均需读取完整文件哈希；命令开始后短时间没有新输出通常是在核验大文件。一次提交完成本阶段，无需重复命令推进。可查询：

```bash
epw_phonon --config "epwinput.toml" --status --json
```

完成应同时满足 workflow_complete=true、声子 completion.json 可验证、results 中已有图表。Slurm 的 COMPLETED 或日志的 JOB DONE 单独不等于工作流交付完成。

结果位于：

- 4.qe-phonon-nosplit/plots/phband/phonon_band.png 和色散 CSV。
- 4.qe-phonon-nosplit/plots/phdos/phonon_dos.png、总量/逐原子/Si/P 元素 DOS 表格。
- 4.qe-phonon-nosplit/plots/phonon_combined.jpg。
- results/4.qe-phonon-nosplit/：供查看和搬运的正式交付。
- epw-state/imports/：外部来源、转换记录及实际检查范围；epw-state/4.qe-phonon-nosplit.log 展示当前导入与后处理状态。各计算目录中的同名 .jobs 文件保存子任务和 Slurm 作业号。

导入核对完整 q 星、结构与原子顺序/质量、扰动基底、响应势有限性/尺寸、当前 SCF/NSCF 的密度及平滑 FFT；来源缺少历史 SCF 赝势、截断能或平滑 FFT 记录时不会声称验证过这些历史信息。历史赝势和截断能一致性由输入准备者负责。旧 ifc.q2r 留在来源目录，新的力常数从动力学矩阵重新计算。

失败后保留现场、修正明确报错，再执行一次：

```bash
epw_phonon --config "epwinput.toml" --import-phonon "external-phonon/save" --resume -s
```

导入路径绝不运行 SCF、NSCF、PH 或 PH 初始化；失败也不会退回声子重算。相同来源的已完成计算复用，缺图时仅补图；来源内容变化或现有证据损坏会拒绝接续。可选只读检查使用同命令加 --check 并去掉 -s；不带 -s 只准备脚本。

## 导入完成后运行 Wannier

```bash
epw_wannier --config "epwinput.toml" --stage wannier -s
```

第 5～8 步恢复纯原生脚本与手动推进：每次 `-s` 校验上一任务并最多提交下一项原生任务；脚本直接调用 `epw.x` 或必要的原生后处理程序，不嵌套 `epw_wannier`、`epw_superconductivity`。Wannier 的实际输入、脚本和输出位于 `5.epw-wannier/`，分别为 `epw_wannier.in`、`epw_wannier.sh`、`epw.out`，模型日志为 `Si8P16.wout`。若依赖尚未完成，命令先提交对应依赖，按返回的实际脚本和目录查看进度。

等待每项作业结束，再运行同一命令核验、出图或提交下一项，直到报告 `workflow_complete: true`：

```bash
epw_wannier --config "epwinput.toml" --stage wannier -s
```

失败后先修正报错，再显式使用 `--resume -s`。不要连续重复提交仍在队列中的任务；只读状态使用 `--status --json`。`nbnd`、窗口、网格和已有有效科学结果不因脚本修复而改变。第 8 步 gap 的 iso/aniso 仍为两个独立原生作业，结束后同样由公开命令在作业外归集和发布。

修复版本新生成或覆盖的 EPW `.sh` 均设置 `0755` 执行权限；绿色显示还取决于终端配色。2026-10-27，本次修复已更新三个默认 EPW 入口，作业 1380018 已以未收敛失败结束，确认终态后旧包装脚本已备份并替换为直接 EPW 调用。上述逐次推进约定用于更新后的程序；安装与回归状态见 [0026 验收](../../docs/specs/0026-epw-direct-native-submit/acceptance.md)。以下保留旧版本说明，供追溯；其中“作业内自动完成第 5 步”的描述已由上述新约定替代：

> 该命令一次提交，在作业内补齐尚缺的电子能带/DOS 参考、拟合、核验和本阶段图表，已有 SCF、NSCF、PH 复用。通过收敛及交付检查后才算完成；未收敛时保留日志和现场，不自动扫描参数。本轮真实验收到 Wannier，粗网格 EPC、细网格和超导不在本轮执行范围。

## 500 轮、原子 s/p 投影与未收敛诊断图

当前源配置的 EPW 与 Wannier90 `num_iter` 均为 `500`，初始投影为 `Si:s,p`、`P:s,p`。省略迭代数时，源码默认也是 500；显式设置仍按用户值执行。旧计算目录中的输入与日志保留当次实际参数，不把历史 5000 轮结果改写成 500 轮。

拟合未收敛、原生程序失败或仍有部分日志时，也应利用已有数据生成诊断图，不在收敛检查处跳过绘图。诊断保存在 `5.epw-wannier/diagnostics/`，包括 `wannier-fit.png`、`wannier-fit-near-fermi.png`、`wannier-spread.png`，相应 CSV、图参和 `diagnostic.json`。尚无完整原生能带时，先出已有 spread 曲线并记录缺失原因，不编造能带。

图中 QE 为深灰实线，Wannier 为蓝色虚线，费米线为红色；外窗和冻结窗边界读取该模型实际 `.win`，以 E−E_F 标注。未收敛图明确标识 `UNCONVERGED`，不生成成功科学回执；诊断出图不表示拟合收敛，也不据此放行 EPC。

修复版本可只读取当前现场重新出图，不运行 QE、EPW 或新的拟合：

```bash
epw_wannier --config "epwinput.toml" --plot
```

2026-10-27 已从作业 1380018 的原始输出生成上述三张诊断图；这三张图仍属于旧的 `sp3 / 5000` 模型。实际输出为 96 条带、96 个 Wannier 函数，没有额外能带用于解纠缠选子空间；窗口标线用于人工检查，不应把输入中的窗口线当成原生日志确认采用了能量筛选。新 `s,p / 500` 配置尚未重跑。

## 6.epw-coarse：粗网格耦合与插值检查

第 6 步复用已导入的声子响应势和第 5 步的 Wannier 模型，计算粗网格电子—声子耦合，再生成 QE/EPW 声子谱、声子总 DOS 对比及总览。当前粗电子网格和粗声子网格均为 `2×3×6`，`nbndsub=96`，`wannierize=false`；这里复用已有模型，不重新拟合。第 7 步才使用 `8×12×24` 电子细网格和 `4×6×12` 声子细网格。

### 提交前确认第 5 步可复用

先进入本例目录，并只读查看状态：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
epw_wannier --config "epwinput.toml" --stage wannier --status --json
```

重点看 `steps.wannier` 是否为 `reuse`，并确认第 4 步声子及电子前置阶段也可复用。`--status` 不推进任务；顶层 `workflow_complete` 可能包含其他阶段的状态，不能单独用它判断第 5 步。第 5 步应已有可验证的 `5.epw-wannier/completion.json`、后处理记录及 `results/5.epw-wannier/stage-manifest.json`，不能只看拟合图或 `All done`。

本次只读核查时，第 5 步尚无完成回执，进度记录仍为 `failed`，现有 `.wout` 是此前 `sp3 / 5000` 的未收敛结果；当前 TOML 已改为 `s,p / 500`。因此，这份旧诊断图即使视觉上吻合，也不能直接当作当前配置的成功模型。若状态仍为 `update`，先处理第 5 步；不要直接运行粗网格命令，因为它会补齐缺失或不相容的依赖，可能重新启动 Wannier 拟合。

如果只是已成功的原生任务尚未归集，按上节重复第 5 步命令核验和推进；如果明确报未收敛，则需先解决拟合问题。不要删除回执、手工改哈希或把诊断图当作成功登记。

### 提交并手动推进第 6 步

确认前置阶段可复用且没有活动作业后执行：

```bash
epw_wannier --config "epwinput.toml" --stage coarse -s
```

每次调用最多提交一项原生任务。记录命令返回的 `job_id`、`script` 和 `directory`，用返回的实际作业号查询队列；已有任务仍在运行时不要重复提交。当前主计算输入为 `6.epw-coarse/epw_coarse.in`，主要输出为该目录的 `epw.out`，脚本直接调用 `epw.x`。

**每项作业结束后，重复同一命令**，让 CLI 核验结果、登记、提交下一项后处理或出图任务：

```bash
epw_wannier --config "epwinput.toml" --stage coarse -s
```

正常链路依次处理粗网格主计算、必要的声子比较采样及绘图发布。比较时可能调用 `q2r.x`、`matdyn.x` 或读取已有模型的 `epw.x`；这些属于后处理，不重新计算 SCF、NSCF 或 PH。第 5 步的电子能带/DOS 对比直接复用，不为第 6 步新增电子参考 PW 计算。

仅查看状态时执行：

```bash
epw_wannier --config "epwinput.toml" --stage coarse --status --json
```

### 查看结果并判断本阶段完成

主要查看以下位置：

- `6.epw-coarse/epw.out`：粗网格原生计算日志。
- `6.epw-coarse/comparison-native/`：声子比较的原生输入、后处理日志和采样来源。
- `6.epw-coarse/decay/`：哈密顿量、动力学矩阵和耦合的实空间衰减图及数据。
- `results/6.epw-coarse/figures/`：QE/EPW 声子谱、声子总 DOS 对比及总览，含 PNG/SVG/PDF。
- `results/6.epw-coarse/comparison/`：比较表格、来源与清单。
- `6.epw-coarse/completion.json`、`post-completion.json` 和 `results/6.epw-coarse/stage-manifest.json`：科学计算、派生交付与结果发布记录。

以推进命令报告 `workflow_complete: true`，且上述记录和图表均可验证，作为第 6 步流程完成判断。只有 Slurm 的 `COMPLETED`、主计算结束或科学回执存在，还不能证明全部比较图已经发布。之后人工查看声子谱/DOS 是否吻合、实空间衰减是否合理；单次粗网格结果不等于网格已收敛，也不是超导结果。

失败后先保留现场，依据日志解决明确报错，再恢复同一目标：

```bash
epw_wannier --config "epwinput.toml" --stage coarse --resume -s
```

恢复复用相容的已完成产物；不要为后处理或出图失败清空整个阶段，更不要重新提交声子。若提示上游不相容或数据损坏，先解决该问题，不强行进入下一阶段。

## 第 6 步确认后：细网格与超导

以下是后续目标的操作入口，先完成并检查第 6 步，再逐阶段执行。它们不会由粗网格命令自动提交。

第 7 步复用粗网格数据，在当前起始细网格上插值并计算细网格耦合、α²F 与 λ：

```bash
epw_superconductivity --config "epwinput.toml" --stage fine -s
```

每项原生作业结束后重复上面的 `fine -s` 命令核验并推进，直到本目标报告完成。数据保存在 `7.epw-fine/`；当前不承诺该阶段新增独立图包。遇到失败先检查日志，修复后使用同目标的 `--resume -s`。

第 7 步完成并确认结果后，第 8 步按当前 `task="gap"`、`mode="both"`，在 `5/10/20 K`、`μ*=0.1` 下分别运行 iso/aniso 能隙求解：

```bash
epw_superconductivity --config "epwinput.toml" --stage superconductivity -s
```

iso/aniso 是两个独立原生方法作业；结束后继续执行同一命令归集、核验和发布。按返回的实际目录查看日志，最终结果位于 `results/8.epw-superconductivity/`。这组指定温度的能隙计算不等于自动得到 Tc；细网格、展宽、温度和 μ* 的收敛仍需另行验证。

## 尚未复制旧数据时的准备步骤

下列步骤仅供新建工作区参考；当前目录用户已完成复制，不必重复。

### 把旧 save/ 复制为独立副本


复制完成后使用上面的 --import-phonon 命令登记，无需手工改名或制作 manifest。

旧声子 `save/` 与电子计算的 `Si8P16.save/` 是两种目录。这里复制的是包含响应势、动力学矩阵和 `Si8P16.phsave/` 的声子归档，不能拿它覆盖 `2.qe-nscf/out/Si8P16.save/`。

只读查看旧归档，已确认有 16 个 `Si8P16.dvscf_q*`、16 个 `Si8P16.dyn_q*`、`Si8P16.phsave/` 和 `ifc.q2r`，总量约 29 GiB；`control_ph.xml` 记录 q 网格为 `2×3×6`、16 个不可约 q 点。导入命令会自动核验全部文件的内容哈希。

```bash
(
  set -eu
  cd "/data/home/mayuan/code/dftflow/examples/SiP2"
  PH_SOURCE="/data/home/mayuan/work/SiP2/data/Pbam/exp_8K_relax_2026-09-29/SiP2_22.2GPa/qe_phonon_2x3x6_inspur/epw_phonon_extract/save"
  PH_COPY="$PWD/external-phonon/save"

  test -d "$PH_SOURCE"
  mkdir -p "$PH_COPY"

  # 复制完整目录，保留文件名与相对结构；来源只读。
  # 若传输中断，保留副本，重新执行相同命令继续复制。
  rsync -a --info=progress2 -- "$PH_SOURCE/" "$PH_COPY/"

  # 只读比较源和副本的文件内容；会完整读取两边的大文件。
  # 正常应无差异输出；-n 表示只预览，不改写任何文件。
  rsync -rcn --itemize-changes -- "$PH_SOURCE/" "$PH_COPY/"
)
```

`external-phonon/save/` 专用于这份独立副本。校验有差异时先处理复制问题，不把数据移动到生产阶段目录，不重命名原始文件，也不把 `dvscf` 转成软链接冒充独立副本。上述比较覆盖来源中的文件，不会删除副本目录中额外存在的文件。

## 旧 epw-state 转换为八份阶段日志（0064）

本节用于仍有 `layout.json`、`native-active.json`、`history/` 等旧状态的工作区，替代上文用旧 JSON 回执判断当前状态的操作。不要先清空 `epw-state/`，也不要为整理目录重新计算 SCF、NSCF 或声子。

使用已发布的新版 CLI，在本例目录执行：

```bash
cd "/data/home/mayuan/code/dftflow/examples/SiP2"
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
epw_wannier --config "epwinput.toml" --migrate-state
```

这条命令不加 `-s`、`-r` 或 `--resume`，不提交新计算。程序先确认旧作业已退出，核验相容的现有科学结果，把当前状态转换为八份中文阶段日志，并保留外部声子的 `epw-state/imports/`。配置原文按 `0000-config.toml` 起递增保存到输入文件同目录。

**“撤下旧状态”就是删除，不是移动或备份。** 转换会删除固定范围内的旧 JSON、`history/`、`attempts/`、`native-jobs/`、`native-progress/`、`requests/`、`qe-data/` 等旧状态，以及被新日志替代的阶段完成回执和电子子步骤回执。`history/` 内若含旧拟合计算副本，也会随该历史目录一起删除；若仍需留存，应在转换前另存。当前八个编号计算目录、当前科学输入输出、`results/`、`external-phonon/save/` 与 `imports/` 导入资料保留。

旧可见状态在命令入口自动转换，因此结尾可能提示“状态已采用八阶段日志，无需转换”。完成后查看实际文件：

```bash
ls -1 "epw-state"
cat "epw-state/5.epw-wannier.log"
cat "epw-state/4.qe-phonon-nosplit.log"
```

本例第 4 步仍为已导入声子的 `nosplit` 模式；其他算例按实际模式使用 `4.qe-phonon-coords.log` 或 `4.qe-phonon-index.log`。应有八份阶段 `.log` 和保留的 `imports/`，旧状态子目录不再作为续算依据。任务号查看各计算目录内同名 `.jobs`；已有终态旧作业不一定保留为当前新任务。

若出现活动作业、查询失败、无法确认的提交状态或未知文件，停止并保留报错，不要手工删除状态或日志。检查大文件可能耗时，转换期间不要同时运行其他 EPW 操作。某阶段显示“旧结果需要检查”时，按该日志中的具体原因处理，不能手工改为完成。

仅转换和查看状态不会重新拟合。需要调整拟合时修改当前 `epwinput.toml`，再显式恢复：

```bash
epw_wannier --config "epwinput.toml" --stage wannier --resume -s
```

作业结束后用普通命令核验和推进后处理：

```bash
epw_wannier --config "epwinput.toml" --stage wannier -s
```
