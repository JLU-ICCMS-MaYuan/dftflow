# epwflow

三个公开程序管理 QE 到超导的八个阶段。编号工作区使用根目录 `input.toml`，每次生成、提交或 `--resume` 都读取当前配置，按阶段核验参数和上游文件哈希，复用相容结果。

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run
# 修改 input.toml 后执行需要到达的阶段：
epw_wannier --stage wannier -s
```

MgB₂ 当前目录与数据导入说明见 [算例说明](../examples/MgB2/sugon-amd/run/README.md)。完整需求和验收见 [0018](../docs/specs/0018-epwflow-stage-workspaces/spec.md)；当前功能入口为 [Overview](../docs/overview/epwflow/README.md)。

## 输入与布局

新配置在原 EPW TOML 基础上设置：

```toml
layout = "numbered"
work_dir = "."
structure = "POSCAR"
kpath = "KPATH.in"
# pseudos、programs、resources、qe 等其余节仍然完整填写。
```

只有根 `input.toml` 需要日常维护；程序生成的原生输入、内部快照和清单由程序管理。相对路径按 TOML 所在目录解释，`work_dir = "."` 不再额外嵌套 epwflow。

```text
run/
├── input.toml、POSCAR、pp/、KPATH.in、submit_all.sh
├── 1.qe-scf/
├── 2.qe-nscf/
├── 3.qe-electronic/
│   ├── qe_eband/
│   └── qe_eledos/
├── 4.qe-phonon-nosplit/       # 也可为 coords 或 index
│   └── save/
├── 5.epw-wannier/
├── 6.epw-coarse/
│   ├── quality-sources/
│   ├── comparison-native/
│   ├── comparison/
│   └── results/figures/
├── 7.epw-fine/
├── 8.epw-superconductivity/
│   ├── iso/
│   └── aniso/
└── .epwflow/
```

每个实际计算目录包含原生输入、`submit.sh` 和日志。分 q、独立参考、细网格展宽及各温度尝试也在自己的目录生成提交脚本。依赖上游产物才能确定的输入和子任务在运行到该阶段时生成；未得到 NSCF 时不会伪造 EPW 选窗。脚本固定工作目录，能够从其他目录提交。根目录 `submit_all.sh` 将完整依赖交给一个有界 Slurm 分配顺序执行。

第 1～4 步复用 qeflow／共享输入、执行及后处理组件，SCF 只做一次并保留独立快照。NSCF 为 EPW 所需的完整均匀网格。`qe_eband` 使用 `pw.x bands` 和 `bands.x`，`qe_eledos` 使用 `dos.x`。PH 三种模式都严格校验 q 点、动力学矩阵、dvscf 和 phsave 后归集到第 4 步 `save/`。

第 5 步只拟合 Wannier；原生 `wannier90.x restart=plot` 和 `postw90.x` 生成电子能带、总 DOS 对比。程序优先读取 `[programs.wannier90]`、`[programs.postw90]`，缺省使用 `pw.x` 同目录的程序。两侧使用共同 Fermi 能、相同高斯展宽，非磁标量路径将 postw90 态密度乘两重自旋，原文件保持不变，规范化数据另存 CSV。实际能窗与高对称点一并标注。

第 6 步核验并复制第 5 步 `.ukk`，设置 `wannierize=false` 执行粗网格 EPC，生成电子、声子对比和独立质量报告。第 7 步生成细网格矩阵元、α²F、λ；第 8 步运行原生 iso／aniso 和有界 Tc 搜索。严格元素／轨道投影继续暂缓，图像不能替代独立质量门。

## 电子后处理失败后恢复

编号工作区分别记录 `pw.x bands`、`bands.x`、`dos.x` 的输入、真实上游与产物哈希，记录保存在 `.epwflow/electronic/`。后处理失败后，复用校验通过的 PW 保存树，只重试未完成的原生任务；MPI 数量变化不要求重跑成功的 PW。科学参数不相容时更新受影响步骤，证据损坏则停止并保留现场。

`bands.x` 使用当前 MPI 启动数量及自身的 `-pd .true.`，避免 slab FFT 出现空进程；不会继承 PW／EPW 的 pool 参数。该调整不改变物理网格、能窗或截断能。DOS 使用自身参数，不跟随添加 pencil 参数。

```bash
epw_wannier --stage wannier --resume -s
```

0018 旧现场的 PW 能带可在完整核验冻结来源、原生输入、零退出码、日志、XML 及全部 portable 波函数后接入；缺证据明确停止。失败日志移到 `.epwflow/history/`，成功结果保持原位。修改过的旧 `submit.sh` 仍会被哈希检查拒绝，重新执行上述公开命令生成新尝试，不修改旧清单。实现与验证边界见 [0020](../docs/specs/0020-qe-electronic-postprocess-resume/acceptance.md)。

## 命令

以下命令都在根 `input.toml` 所在目录执行。可用 `-c /绝对路径/input.toml` 从其他位置调用。

| 命令 | 到达的阶段 |
| --- | --- |
| `epw_phonon -s` | 第 1～4 步 |
| `epw_wannier --stage wannier -s` | 第 5 步 |
| `epw_wannier --stage coarse -s` | 第 6 步 |
| `epw_wannier -s` | 默认到第 6 步 |
| `epw_superconductivity --stage fine -s` | 第 7 步 |
| `epw_superconductivity --stage superconductivity -s` | 第 8 步 |
| `epw_superconductivity -s` | 默认到第 8 步 |

前置步骤相容时复用；缺失或过期时按依赖补齐。去掉 `-s` 只生成输入和脚本，不运行科学程序、不提交任务。`--resume -s` 也读取当前配置；无需手动寻找 `CALC` 或填写多份阶段 TOML。`-r` 只允许在已经获得的 Slurm 分配内执行。

```bash
epw_phonon --check --json        # 配置、文件路径与相应并行预检
epw_wannier --status --json      # 按当前配置判断各阶段 reuse/update
epw_phonon --all                # 仅生成完整依赖的目录和脚本
bash submit_all.sh               # 提交完整依赖
# 也可手动提交某一生成脚本；启动仍经过相同预检：
sbatch 5.epw-wannier/submit.sh
```

`operation_status=ok` 表示这次检查操作通过；`workflow_status=configured` 表示配置及脚本已准备。两者都不表示 QE／EPW 已运行、计算收敛、质量通过或 Tc 已得到。`--status` 的 `reuse` 同时要求科学键、上游产物键和本阶段输出哈希有效，`update` 表示当前请求需要执行。活动任务和终态未知任务会阻止再次生成或提交，以保护现场。

## 改参和复用

| 修改 | 更新范围 |
| --- | --- |
| MPI、pool、分区、时限 | 只更新后续提交资源，已有相容科学结果保留 |
| `interpolation.k_grid/q_grid`，即 nkf／nqf | 第 7、8 步 |
| Wannier 能窗、初始投影、函数数目 | 第 5～8 步 |
| `phonon.q_grid`，即粗 nq | PH、声子后处理及第 6～8 步；SCF、NSCF、Wannier 保留 |
| 结构、赝势、截断能 | 从最早受影响的上游开始 |
| 比较展宽 | 原生 DOS 后处理及重绘，不重做拟合、PH 或 EPC |

路径点数改变需要补齐相应路径参考；它不改变已有 Wannier 变换。失效现场移动到 `.epwflow/history/`，不会混合新旧数据。已成功的原生子尝试重验后复用，失败子尝试归档后重试。文件损坏、来源冲突及科学质量失败会明确停止，不凭完成标记跳过核验。

每次提交在 `.epwflow/attempts/` 保存本次完整配置和输入副本；排队期间改根配置不会改变已提交任务。后续提交从新的根配置建立新尝试，不再受旧 64 进程快照限制。当前整工作区串行，活动或未知任务不会重复提交。共享预算路径在配置中保留，用户无需手动配置 `LEDGER` 变量；累计消费不重置。

## 并行预检

生成、提交前和脚本启动时均校验 Slurm ntasks、MPI 数、pool 和 image。EPW 当前支持单 image，要求一个 MPI 对应一个 pool，且 pool 数不超过该次可分配 k 点数。粗网格读取实际 NSCF 点数，细网格使用本次 nkf。QE 采用自己的整除规则，允许每 pool 多个 MPI。

例如完整粗网格 `4×4×2=32` 点配 `MPI=64, pool=64` 会打印网格、32 点、64 MPI／pool 和空 pool 原因，然后非零退出。程序不会调整物理网格或并行参数来绕过错误。完整原生粗矩阵按 EPW 5.9 便携格式检查；`.ukk` 额外检查 k 点数和保留能带。失败的细矩阵重启现场不会带到不同资源的新尝试；完整细矩阵由原生求解器按原分片重新分配。

## 旧工作区导入

一次性导入前，在新根配置设置 `layout="numbered"`、当前目标 `work_dir`，保持要复用的科学参数与旧 QE 相容，资源可以不同：

```bash
epw_phonon --import-work epwflow/work
```

导入核验旧运行绑定、完整 QE 清单、文件集合与 SHA-256，复制 SCF、NSCF、三模式 PH 归集和已有声子后处理，建立新阶段清单和来源记录。旧来源不删除、不改写；已有 QE 不会重算。导入日志保存于 `.epwflow/legacy-import.json`，中断的受控物化可根据 `importing.json` 继续。目标已有未知同名目录或旧任务未结束时停止，不覆盖。

未设置 `layout` 的历史工作区仍保留原固定绑定行为，原有 `--resume -w 旧work --mpi ...` 只服务旧失败粗阶段。日常新流程使用根配置；历史接口细节见 [0008 三程序设计](../docs/specs/0008-epwflow-foundation/three-programs.md)。

## 构建和验证

```bash
bash scripts/build-native.sh build -p epwflow --locked
# 先准备与计算节点兼容的原生运行制品，再仅安装本模块：
bash scripts/install-bin.sh --module epwflow /兼容制品目录
bash scripts/build-native.sh test -p epwflow -p qeflow --locked
```

`bin/` 已被 Git 忽略，按 `epwflow`、`qeflow`、`scflow`、`substitutionflow`、`vaspflow`、`wannierflow` 分组。`scripts/env.sh` 将模块路径加入当前终端 PATH，不要求在测试目录寻找可执行文件。

公开进程及脚本测试使用隔离替身，证明调度、布局、复用和数据防护，不认证科学数值。真实 MgB₂ 全链和原生 iso／aniso Tc 仍需真实科学验收；本需求不代用户提交真实任务。质量、预算与来源保护保持原约束，见 [0015 验收](../docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md)。
