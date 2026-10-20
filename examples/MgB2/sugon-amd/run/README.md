# MgB₂：sugon-amd 手动测试

QE／EPW 的当前工作目录是 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run`。日常只改这里的 `input.toml`，共用结构为 `POSCAR`、赝势为 `pp/`、路径为 `KPATH.in`。内部记录由程序维护，不需要设置 `CFG`、`W`、`CALC` 或 `LEDGER`。

## 当前应执行的命令

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run

epw_phonon --status --json
# 按需修改 input.toml，然后检查第 5 步的配置与并行：
epw_wannier --stage wannier --check --json
# 确认参数后，由你提交独立 Wannier：
epw_wannier --stage wannier --resume -s
```

`source` 把仓库 `bin/` 下各模块目录加入当前终端的 PATH。以后可以直接写程序名；只加 `bin/` 父目录不会递归找到子目录的程序。可用 `command -v epw_phonon epw_wannier epw_superconductivity` 核对位置。新终端需要再次 `source`；如需自动加载，可自行将这条 `source` 写入 Shell 配置。

`cd` 进入唯一日常配置所在目录。`--status` 按当前科学参数和文件哈希打印各阶段的 `reuse/update`；它不启动计算。`--check` 检查配置、文件路径和本次并行设置。`-s` 才提交任务：程序读取本次 `input.toml`，自动补齐必要依赖、复用已有成功结果。

旧数据导入后，SCF、NSCF、PH 及已有 q2r／matdyn 会复用。新第 3 步补充电子能带与总 DOS 参考，然后第 5 步拟合 Wannier；电子路径上的 `pw.x bands` 是新能带参考，不是重做原 SCF、均匀 NSCF 或 PH。相容的旧独立质量参考也有来源记录，供第 6 步复用。

## 本次短暂运行后失败的原因与恢复

2026-10-20 作业 1306688 已完成 `3.qe-electronic/qe_eband/eband.out` 中的 PW 能带计算（183 个 k 点、16 带），随后 `bands.err` 报 `there are processes with no planes`：32 MPI 的 slab FFT 分配出现空进程，尚未进入 Wannier。首次修复为 `bands.x` 增加 `-pd .true.`，保留配置的 32 MPI。

作业 1306749 因手工修改后的 `5.epw-wannier/submit.sh` 与冻结哈希不符而停止。删除脚本严格选项不能解决 FFT 错误；新版生成器已按 0019 移除自动严格选项，通过显式退出码处理关键失败。

后续作业 1307064 中 `bands.x` 已成功（零退出码和完整产物），但 `3.qe-electronic/qe_eledos/dos.err` 出现同一 FFT 空进程错误，DOS 退出 6；Wannier 仍未开始。0021 为 `dos.x` 同样补上 `-pd .true.`，保留当前 32 MPI。

1307068 的 Slurm 提交记录是 `sbatch submit.sh`，它重用了已绑定 1307064 的旧尝试，故报作业身份不一致。该保护保留；不要编辑脚本或内部清单，也不要重提已经执行过的旧脚本。恢复请在本页开头的 `run/` 重新执行公开命令：

```bash
epw_wannier --stage wannier -s
```

可加 `--resume`，两种写法都读取当前 `input.toml`，生成新尝试。相容且校验通过的 SCF、NSCF、PH、PW 能带和 bands 后处理复用，从 DOS 继续，再进入 Wannier 及电子对比。现场 PW 和 bands 收据的来源及产物哈希已只读核验；本次不代提交真实作业，DOS 修复后的科学运行仍待验证。证据见 [0021 验收](../../../../docs/specs/0021-qe-dos-pencil-resume/acceptance.md)。

`.epwflow/electronic/` 分别保存三个电子原生任务的完成证据。日志出现 `reuse: electronic/pw-bands` 表示旧 PW 已重验并复用；阶段状态仍为 `update` 可以只是后处理未完成。证据损坏或旧来源缺失会停止，不能仅凭 `JOB DONE` 跳过检查。不要重提修改过的旧脚本，也不要编辑内部哈希记录。

## 目录与结果

```text
run/
├── input.toml
├── POSCAR
├── pp/
├── KPATH.in
├── submit_all.sh
├── 1.qe-scf/                       # scf.in、submit.sh、scf.out、不可变快照
├── 2.qe-nscf/                      # nscf.in、submit.sh、nscf.out、完整均匀波函数
├── 3.qe-electronic/
│   ├── qe_eband/                   # eband.in、elebanddata.in、submit.sh、能带
│   └── qe_eledos/                  # eletdos.in、submit.sh、总 DOS
├── 4.qe-phonon-nosplit/             # 可选 coords/index；PH、q2r、matdyn
│   └── save/                      # 经校验的 dyn、dvscf、phsave 与清单
├── 5.epw-wannier/                  # 独立拟合、电子能带与总 DOS 对比
├── 6.epw-coarse/                   # 复用 ukk、粗 EPC、质量及四类对比
├── 7.epw-fine/                     # 分展宽计算、α²F、λ、细矩阵
├── 8.epw-superconductivity/
│   ├── iso/                       # 每次温度尝试包含 epw.in 和 submit.sh
│   └── aniso/
└── .epwflow/                      # 清单、提交快照、历史与迁移证据
```

原生输入和提交该计算的 `submit.sh` 在同一个实际目录，日志也在该目录。分 q、分展宽、分温度和独立参考遵守同样规则。依赖上游实际输出的输入在运行到对应步骤时生成，空目录不代表已计算。每个脚本会先进入自己的绝对工作目录，所以可以从其他目录 `sbatch /绝对路径/submit.sh`。

第 5 步电子图为 `electronic-bands.png`、`electronic-dos.png`，同时保存 DOS CSV 和规范化说明。第 6 步四类对比图与总览位于 `6.epw-coarse/results/figures/`，含 PNG／SVG／PDF；QE 实线先画，EPW 虚线后画。严格元素／轨道投影继续暂缓，不能把总 DOS 当作 PDOS。第 8 步成功求解后另在本阶段保存超导结果包。

## 第 5 步以后怎样运行

每次先等待当前作业结束，再执行下一步；`--status --json` 显示最近尝试的作业号和状态。

```bash
# 第 5 步完成后：粗网格 EPC、电子/声子对比及独立质量门。
epw_wannier --stage coarse -s

# 第 6 步质量通过后：细网格 EPC、α²F 和 λ。
epw_superconductivity --stage fine -s

# 第 7 步完成后：按配置的 iso/aniso 做原生求解。
epw_superconductivity --stage superconductivity -s
```

`epw_wannier -s` 默认到第 6 步；`epw_superconductivity -s` 默认到第 8 步。根目录 `bash submit_all.sh` 也提交到第 8 步，在一个有界分配内按依赖顺序推进，质量失败即停止。手动测试时优先逐阶段查看结果。

去掉 `-s` 只生成，不提交。修复失败原因后，修改根配置再执行同一命令即可，也可加 `--resume -s`；它同样读取当前配置。资源改变不会要求新建整套 QE 运行。若状态仍为运行中或调度器终态未知，程序拒绝重复提交。

## 参数怎样影响重算

| 根配置修改 | 预期行为 |
| --- | --- |
| `resources.mpi`、启动器 `-np`、EPW `-nk`、分区、时限 | 更新待执行任务资源；保留相容科学结果 |
| `interpolation.k_grid/q_grid`，对应 nkf/nqf | 更新第 7、8 步 |
| `wannier` 的能窗、函数数目、初始投影 | 更新第 5 步及依赖它的 EPW 阶段 |
| `phonon.q_grid`，对应粗 nq | 更新 PH、声子后处理和第 6～8 步；保留相容 SCF、NSCF、Wannier |
| 结构、赝势、截断能等 | 从最早受影响的上游更新 |
| 比较展宽 | 只更新对应 DOS 后处理与图像 |

修改 MPI 时必须同步修改 `resources.mpi`、`programs.launcher` 的 `-np` 和 `programs.epw.args` 的 `-nk`。当前 EPW 单 image 要求每个 pool 一个 MPI，且 pool 不超过 k 点数。当前 `4×4×2` 粗网格只有 32 个 k 点，不能配 64 pool；32 MPI／32 pool 是可用配置。若改为 64，会明确说明空 pool 风险并退出，程序不会自动改物理网格。细网格按本次 nkf 检查。

生成、提交前、脚本启动前都会预检；手工 `sbatch` 也不能跳过。完整原生矩阵文件还会核对格式、k/band 范围和原分片，不能因为 MPI 改了就把损坏或不相容文件当作可复用数据。

## 旧目录、work 与 scratch

`epwflow/work/` 是旧布局的工作根：含运行绑定、`workflow.json`、`calculations/<尝试号>/` 和旧原生输出。`epwflow/scratch/` 是旧配置的临时数据基址，用于程序运行时的电荷、波函数、响应等大文件；存在与否不表示科学成功。EPW 原生输入中的 outdir 指向实际临时数据，不能只看目录名猜测。不要删除或手改旧数据来“修复”续算。

新布局直接把计算数据放在编号目录：各阶段 `out/` 保存本阶段原生可重读数据，SCF 的不可变快照为其他阶段提供独立副本；`.epwflow/` 管理内部记录和历史。根配置的 `outdir` 保留兼容及预算基址语义，实际原生路径以生成输入为准。无需再手动创建 `epwflow/work` 或 `epwflow/scratch`，不会嵌套产生另一个 epwflow。

一次性迁移命令是：

```bash
epw_phonon --import-work epwflow/work
```

程序只读核验旧绑定、参数、清单及哈希后复制到新布局，并发布 `.epwflow/legacy-import.json`。已有 QE 不会因迁移重算；旧现场保持原样。相同来源重复导入会重验后返回 `reused`。这不是日常提交前要反复执行的步骤。

历史命令中的 `CALC` 仅表示旧 `workflow.json` 的 `calculation_dir` 字段，例如：

```bash
# 只用于理解/检查旧布局；新流程不需要这个变量。
CALC=$(python -c 'import json; print(json.load(open("epwflow/work/.epwflow/workflow.json"))["calculation_dir"])')
printf '%s\n' "$CALC"
```

`python` 读取旧 JSON，取出这次运行的绝对计算目录；`$(...)` 把输出赋给 Shell 变量；`printf` 打印实际值。它不是占位文字，也不能照搬别人的尝试号。现在直接在根目录执行公开命令，由程序定位阶段数据。

## 状态字段与预算

`operation_status = "ok"` 表示当前操作成功；对于 `--check`，检查的是 TOML 字段/范围、输入和程序文件可访问性，以及 MPI/pool/image。`workflow_status = "configured"` 表示配置或脚本已准备。它们不表示已提交、QE/EPW 已完成、数值收敛或插值质量通过。

`--status` 的 `reuse` 需要本阶段科学参数、实际上游产物和逐文件哈希一致；`update` 表示需要新计算。提交成功的作业号也只是调度回执。小网格流程测试与真实 MgB₂ 收敛结果分开记录，声子质量失败会阻止细网格和超导。

预算账本由根配置已有的 `budget.ledger` 指定，用于累计节点时间、并发预留和存储额度；不需要手工设置 `LEDGER` 变量，也不能另开账本重置旧消费。当前提交保存独立参数快照，排队后修改根配置不影响已提交任务；下一次提交再读新配置。旧工作区、历史绑定程序和来源记录保留，以便追溯。

实现、测试证据和科学验收边界见 [0018 验收](../../../../docs/specs/0018-epwflow-stage-workspaces/acceptance.md)。
