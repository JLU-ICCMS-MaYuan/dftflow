# SiP₂：复用历史声子的电子计算输入

`epwinput.toml` 使用 schema 2，与支持 0059/0060 的程序配套。`POSCAR` 从历史 SCF 输入保留原始晶格和 24 个原子的分数坐标；`KPATH.in` 沿用历史 Wannier 的 12 段路径。赝势使用配置中已有文件的绝对路径。

| 参数 | 当前设置 |
| --- | --- |
| SCF / NSCF 均匀网格 | 2×3×6，零位移；NSCF 完整 36 点 |
| 密度 / 平滑 FFT | 216×128×60 / 120×72×32，SCF 与 NSCF 显式一致 |
| 截断能 | 60 / 720 Ry |
| NSCF 能带 / Wannier 函数 | 96 / 96，Si:sp3、P:sp3 |
| 细电子 / 细声子网格 | 8×12×24 / 4×6×12 |
| μ* / 温度 | 0.1 / 5、10、20 K |

细网格、温度和 μ* 是用户确认的起始设置，尚未做收敛验证。历史 Wannier 的迭代上限为 500；当前配置采用流程的 5000 上限和收敛检查。费米能从新 NSCF 读取，不固定历史数值。其余数值迭代和后处理设置也需要后续核验。

## 当前执行边界

外部声子来源为：

```text
/data/home/mayuan/work/SiP2/data/Pbam/exp_8K_relax_2026-09-29/SiP2_22.2GPa/qe_phonon_2x3x6_inspur/epw_phonon_extract/save
```

当前原始 `save/` 导入尚未接通，不能直接进入 Wannier 或超导阶段。配置中的第 4 阶段用于满足完整配置要求，运行它会重算声子；**不要运行默认声子入口或 `--stage phonon`，也不要提交全流程。** 同一 FFT 设置还需要在声子导入时与来源数据逐项核验。

下列命令通过仓库环境脚本加载正式 CLI，`epw_phonon` 应解析为 `/data/home/mayuan/code/dftflow/bin/epwflow/epw_phonon`。不再使用 `docs/local/0059-native-bin` 验证副本。SCF、NSCF 的首次提交步骤保留如下；已经完成计算时，直接从后面的“NSCF 完成后的操作”继续，不要重新执行首次提交步骤。

## SCF、NSCF 的首次计算

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
command -v epw_phonon
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 只读检查，不提交作业。
epw_phonon \
  --config "epwinput.toml" --stage nscf --check --json

# 第一步：用户提交 SCF。
epw_phonon \
  --config "epwinput.toml" --stage scf -s

# 等待 SCF 作业成功结束后，再运行同一命令登记完成状态。
epw_phonon \
  --config "epwinput.toml" --stage scf -s

# 第二步：SCF 完成状态登记成功后，用户提交 NSCF。
epw_phonon \
  --config "epwinput.toml" --stage nscf -s

# 等待 NSCF 成功结束后，登记完成并执行输入/输出证据校验。
epw_phonon \
  --config "epwinput.toml" --stage nscf -s
```

NSCF 配置只保存 `[2, 3, 6]`。Rust 按 `kmesh.pl` 的顺序和数值格式展开到 `nscf.in`，完成后比较 NSCF 输入、输出的完整点数和网格。若校验失败，应先保留报错证据，再考虑显式改用 `pw.x` 的 `calculation='bands'`；这是 `pw.x` 的运行模式，不是后处理程序 `bands.x`。不会自动切换模式或重试。

## NSCF 完成后的操作

### 1. 先登记已完成的 NSCF

2026-10-27 只读核查结果：

| 检查对象 | 当前证据 |
| --- | --- |
| NSCF 作业 | `1375900`，Slurm 为 `COMPLETED`，退出码 `0:0` |
| 科学程序日志 | `2.qe-nscf/nscf.out` 包含 `JOB DONE.` |
| NSCF 实际 XML | `nks=36`、`nbnd=96`，有 36 组电子态记录 |
| 新 SCF／NSCF 的实际 FFT | 密网格均为 `216×128×60`，平滑网格均为 `120×72×32` |
| 工作流登记 | SCF 已有完成回执；NSCF 尚无 `2.qe-nscf/completion.json`，原生任务记录仍为 `submitted` |

因此当前是“NSCF 科学程序已结束，工作流完成登记仍待执行”。保持本次计算的配置、输入和脚本不变，由用户运行：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 核对原作业，不提交计算。
sacct -j 1375900 --format=JobID,State,ExitCode

# 普通续接：校验原作业及产物，并登记 NSCF 完成。
# 不加 --resume；该命令含提交能力，若出现新的提交提示，应先核对原因。
epw_phonon --config "epwinput.toml" --stage nscf -s

# 登记成功后应存在此文件。
test -s "2.qe-nscf/completion.json"
```

这一步沿用原任务记录，正常情况下应复用已完成 SCF、核验已完成 NSCF。出现配置身份冲突、FFT 或 k 点校验失败时，保留报错，不删除回执或任务记录来强行继续，也不直接恢复重算。

`epw_phonon --config "epwinput.toml" --stage nscf --check --json` 已通过配置、输入文件、程序路径和并行设置检查；它返回的是 `configured`，不代替上述完成登记，也不证明声子数据已导入。

### 2. 把旧 save/ 复制为独立副本

**当前可以准备旧声子数据副本；复制完成不代表已经正式导入工作流。** 下面的命令由用户执行，本次 README 更新没有执行复制或读取全部大文件的校验。

旧声子 `save/` 与电子计算的 `Si8P16.save/` 是两种目录。这里复制的是包含响应势、动力学矩阵和 `Si8P16.phsave/` 的声子归档，不能拿它覆盖 `2.qe-nscf/out/Si8P16.save/`。

只读查看旧归档，已确认有 16 个 `Si8P16.dvscf_q*`、16 个 `Si8P16.dyn_q*`、`Si8P16.phsave/` 和 `ifc.q2r`，总量约 29 GiB；`control_ph.xml` 记录 q 网格为 `2×3×6`、16 个不可约 q 点。大响应文件的完整哈希尚未在本轮重算。

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

### 3. 正式导入目前尚无可执行入口

2026-10-27 已核对当前正式 CLI 的 `--help` 和仓库导入实现：

| 现有选项 | 实际接收的来源 | 是否适用于这份原始 save/ |
| --- | --- | --- |
| `--import-work` | 带 `run-binding.json`、`input-init.toml`、工作流记录和已验证 QE 清单的旧 epwflow 工作区 | 否；也不是向当前已有 SCF／NSCF 的目录追加原始 PH 的入口 |
| `--import <MANIFEST>` | 已有 `<source_work>/qe-data/manifest.json` 及其绑定的完整原生产物 | 否；不能把 `control_ph.xml` 或 save/ 路径当作该清单 |
| `--import-phonon` | 当前正式 CLI 未提供 | 尚不能使用 |

因此，**现在不能给出一条真实可用的“导入这份原始 save/”命令**。不要对该目录执行 `--import-work`，不要手工伪造 `completion.json` 或把清单状态改为 `valid`。仅将文件放进 `4.qe-phonon-nosplit/save/` 也不能建立可复用状态。

正式接入需要由原始声子导入能力完成以下核验和登记，当前均不能因复制完成而视为通过：

1. 从 PH 来源核对结构、原子顺序、赝势、截断能和实际 FFT；新 SCF／NSCF 两份 FFT 一致不能单独证明旧 PH 一致。
2. 绑定 `2×3×6` 网格、16 个不可约 q 点、响应势与 `patterns` 的编号映射、扰动基底和时间反演关系。
3. 识别旧 `dyn_q*` 的文本格式。当前原生声子交付按 XML 动力学矩阵处理，不能仅改扩展名。
4. 核对每份响应势的存储布局、尺寸和内容身份，并绑定本次新 NSCF 的实际输入及波函数。
5. 发布可校验的 QE 交付清单、PH 来源记录和阶段完成回执，保留新 SCF／NSCF 及旧声子来源。

现有入口的限制可从 [旧工作区导入实现](../../epwflow/src/foundation/workspace_import.rs)、[清单导入实现](../../epwflow/src/qe_data/cli.rs) 和 [QE 清单校验](../../epwflow/src/qe_data/manifest.rs) 核对。FFT 来源及读取路径见 [SiP₂ 错误分析](SiP2-errors-analysis.md)。

### 4. 正式导入通过后，才继续 Wannier 和粗网格

以下为导入能力接通、来源与交付验证通过后的操作顺序，**不是复制 save/ 后就能立即执行的命令**：

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/SiP2"

# 已通过正式声子导入与交付校验后，提交 Wannier 阶段。
epw_wannier --config "epwinput.toml" --stage wannier -s

# 等待作业完成，再运行同一命令核验和登记；确认本阶段完成后继续。
epw_wannier --config "epwinput.toml" --stage wannier -s

# Wannier 收敛及阶段交付通过后，提交粗网格 EPC 与比较阶段。
epw_wannier --config "epwinput.toml" --stage coarse -s

# 等待作业完成后核验和登记。
epw_wannier --config "epwinput.toml" --stage coarse -s
```

只有 Wannier 收敛、粗网格产物有限性和相应比较／交付核验通过，才继续细网格和超导阶段。粗网格还会涉及 [错误分析](SiP2-errors-analysis.md) 中记录的 EPW 双 FFT 分支问题；六个 FFT 参数匹配不等于该外部程序缺陷已经修复。

当前可执行到的范围是：**登记 NSCF 完成 → 独立复制并比较旧 save/ → 等待原始 PH 正式导入能力接通**。本次只更新操作说明，没有执行登记、复制、声子导入、后续提交或重新计算。
