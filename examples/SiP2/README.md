# SiP₂：单命令导入历史声子并接续 Wannier

`epwinput.toml` 使用 schema 2 平层配置，与整合 0060/0061 的正式程序配套。`POSCAR` 从历史 SCF 输入保留原始晶格和 24 个原子的分数坐标；`KPATH.in` 沿用历史 Wannier 的 12 段路径。赝势使用配置中已有文件的绝对路径。

| 参数 | 当前设置 |
| --- | --- |
| SCF / NSCF 均匀网格 | 2×3×6，零位移；NSCF 完整 36 点 |
| 密度 / 平滑 FFT | 216×128×60 / 120×72×32，SCF 与 NSCF 显式一致 |
| 截断能 | 60 / 720 Ry |
| NSCF 能带 / Wannier 函数 | 96 / 96，Si:sp3、P:sp3 |
| 细电子 / 细声子网格 | 8×12×24 / 4×6×12 |
| μ* / 温度 | 0.1 / 5、10、20 K |

细网格、温度和 μ* 是用户确认的起始设置，尚未做收敛验证。历史 Wannier 的迭代上限为 500；当前配置采用流程的 5000 上限和收敛检查。费米能从新 NSCF 读取，不固定历史数值。其余数值迭代和后处理设置也需要后续核验。

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

该阶段不调用 PH，不需要等待声子导入或 Wannier。也可以由下文的 Wannier 命令补齐；已有兼容任务正在运行时，应先等待该任务结束，避免重复提交。

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

2026-10-27，本目录已通过正式入口完成历史声子导入（作业 1379059，COMPLETED/0），声子谱、TDOS、Si/P 元素 DOS 和组合图已生成并查看。原 1074 个来源文件及 SCF/NSCF 的 126 项科学产物经完整哈希复核均保持不变。下面的导入命令可重复执行以核验并复用结果。真实验收作业 1379339 已完成所需电子参考，但在 EPW 启动前被内嵌采样输入的 200 条上限拒绝；正在验证保持全部目标坐标的生成修复，恢复时将复用现有电子参考。

当前目录已有 SCF、NSCF 完成回执。不要重新提交这两项，也不要用默认声子计算替代导入。已复制到 external-phonon/save 的旧声子目录保持只读。

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
command -v epw_phonon
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 一次提交；作业内完成核验、转换、q2r、两次 matdyn、绘图与完成登记。
epw_phonon --config "epwinput.toml" --import-phonon "external-phonon/save" -s
```

正式命令应来自 /data/home/mayuan/code/dftflow/bin/epwflow，不再使用 docs/local 中的验证副本。首次导入自动核验旧电子记录，将旧嵌套 TOML 转为平层格式，保留全部科学参数；原配置、旧回执及接续报告保存在 epw-state/history/flat-import-*，不删除或伪造原记录。

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
- epw-state/qe-data/manifest.json、epw-state/qe-data/external-source.json：来源哈希、转换记录及实际检查范围。

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

该命令一次提交，在作业内补齐尚缺的电子能带/DOS 参考、拟合、核验和本阶段图表，已有 SCF、NSCF、PH 复用。通过收敛及交付检查后才算完成；未收敛时保留日志和现场，不自动扫描参数。本轮真实验收到 Wannier，粗网格 EPC、细网格和超导不在本轮执行范围。

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

  # 本地复制全部内容（包括隐藏文件），保留目录结构、权限和时间；来源只读。
  cp -a -- "$PH_SOURCE/." "$PH_COPY/"

  # 只读比较源和副本的文件内容；会完整读取两边的大文件。
  # 正常应无差异输出；-n 表示只预览，不改写任何文件。
  rsync -rcn --itemize-changes -- "$PH_SOURCE/" "$PH_COPY/"
)
```

`external-phonon/save/` 专用于这份独立副本。校验有差异时先处理复制问题，不把数据移动到生产阶段目录，不重命名原始文件，也不把 `dvscf` 转成软链接冒充独立副本。上述比较覆盖来源中的文件，不会删除副本目录中额外存在的文件。
