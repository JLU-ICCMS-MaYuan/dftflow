# epwflow

编号工作区新原生脚本使用 `<功能>.sh`（如 `scf.sh`、`bands.sh`、`epw_wannier.sh`），Slurm 两路日志合并到 `slurm-<功能>.log` 并覆盖写入。公开命令在实际计算目录调用 sbatch；脚本只在正文 cd，不设置或传入 --chdir。手工 sbatch 时先进入脚本所在目录，使 Slurm 日志也留在那里。历史冻结脚本保持原字节，细节见 [0046 验收](../docs/specs/0046-epw-submit-simplification/acceptance.md)。

## MPI 启动器

配置使用 `launcher = ["mpirun", "-np", "32"]`，并在 `programs.environment_script` 指定已有环境脚本。新作业先加载环境，再普通调用 `mpirun`；即使旧配置含 MPI 目录，新脚本也使用 PATH 查找，不增加 `exec`。登录节点无需先激活 MPI。科学程序路径和原参数仍按配置；其他自定义启动器沿用原规则。

历史提交脚本、固定配置和回执保持原字节；新原生任务不再生成 submit.sh 阅读副本。[0044 规格与部署](../docs/specs/0044-mpi-submit-command/spec.md)。

## 编号工作区逐任务提交（0026）

对 `layout='numbered'` 的工作区，`-s` 核验上一任务并且最多提交一次新的原生调用。实际作业脚本直接执行当前配置中的 QE、Wannier90／postw90 或 EPW；准备输入、交接文件、校验产物及推进均由作业外公开命令负责。作业完成后重复同一个命令推进，无后台控制器。

```bash
epw_phonon -c input.toml -s
epw_wannier -c input.toml -s
epw_superconductivity -c input.toml -s
```

每条命令分别推进其目标；在当前目标显示 `workflow_complete: true` 后切换下一条。原生失败后修复配置，再用对应命令 `--resume -s`；排队、运行中或回执未知时拒绝重提。Tc 的单模式、单温度、单初值分别提交，保留原搜索规则及次数上限。

输出给出实际提交的功能脚本、作业号与工作目录。QE／EPW 科学输出直接写指定 `.out`，错误写 `.err`，计算运行时即可查看，不等待复制 Slurm 日志；Wannier／postw90 保留种子参数接口。新脚本的命令类似：

```bash
if ! cd -- "/path/to/1.qe-scf"; then exit 1; fi
source "/path/to/oneapi/setvars.sh" --force
mpirun -np 32 /path/to/pw.x < scf.in > scf.out 2> scf.err
```

输入及命令身份记录在可见的 `epw-state/native-jobs/`；独立 W90 后处理使用 `5.epw-wannier/native-post/`，不重写拟合来源。source 按配置直接执行，没有 if 或自动严格选项；科学命令失败仍返回非零。普通安全参数直接显示，含空格或 Shell 特殊字符时保留必要引用。

`epw_phonon --all` 只准备下一就绪脚本与 `native-plan.json` 待完成清单，零提交，不生成自动全流程总脚本。`--status [--json]` 只读区分运行中、已结束待核验、失败及已核验。

可手工修改脚本并 `sbatch` 独立 debug，但它不产生正式完成记录；准备或活动任务的脚本被修改时，正式命令仍拒绝接管。版本 2 任务结束并完成核验后，覆盖功能脚本或 Slurm 日志不影响科学复用；输入、来源和科学输出仍严格校验。版本 1 历史任务保持原校验及日志读取。旧封存脚本和默认安装不随源码修改自动更新。旧非编号布局的新 `-s` 拒绝继续提交包装脚本；使用新的编号工作目录并通过 `--import-work` 显式导入。`-r` 的既有分配执行路径保留。

以下早期脚本／自动串联说明属于旧路径；编号工作区以本节为准。[0026 设计与验收](../docs/specs/0026-epw-direct-native-submit/plan.md)。

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
│   ├── comparison-native/
│   ├── comparison/
│   └── results/figures/
├── 7.epw-fine/
├── 8.epw-superconductivity/
│   ├── iso/
│   └── aniso/
└── epw-state/
```

每个实际计算目录包含原生输入、`submit.sh` 和日志。分 q、细网格展宽及各温度尝试也在自己的目录生成提交脚本。依赖上游产物才能确定的输入和子任务在运行到该阶段时生成；未得到 NSCF 时不会伪造 EPW 选窗。脚本固定工作目录，能够从其他目录提交。根目录 `submit_all.sh` 将完整依赖交给一个有界 Slurm 分配顺序执行。

第 1～4 步复用 qeflow／共享输入、执行及后处理组件，SCF 只做一次并保留独立快照。NSCF 为 EPW 所需的完整均匀网格。`qe_eband` 使用 `pw.x bands` 和 `bands.x`，`qe_eledos` 使用 `dos.x`。PH 三种模式都严格校验 q 点、动力学矩阵、dvscf 和 phsave 后归集到第 4 步 `save/`。

第 5 步只拟合 Wannier；原生 `wannier90.x restart=plot` 和 `postw90.x` 生成电子能带、总 DOS 对比。程序优先读取 `[programs.wannier90]`、`[programs.postw90]`，缺省使用 `pw.x` 同目录的程序。两侧使用共同 Fermi 能和相同高斯展宽。非磁标量路径显式设置 postw90 每态电子数为 2，并核对原生日志；原生总 DOS 已计入自旋，CSV 和图不再乘二，规范化记录同时保存原始 DOS 与日志哈希。实际能窗与高对称点一并标注，断开的路径分段绘制，共享横坐标的端点名合并显示。依据见 [0031](../docs/specs/0031-postw90-dos-spin-normalization/acceptance.md)、[0032](../docs/specs/0032-disconnected-electronic-band-paths/acceptance.md)。

独立拟合输入不声明粗网格插值使用的 `filkf/filqf`。后处理自动从校验后的同一 NSCF 提取晶格、原子、网格、原 k 点顺序及排除后的能带，补齐独立 W90 的 `.win/.eig`；无需用户维护另一份输入。原生 DOS 使用 `dos_smr_type = gauss`。除退出码外，还检查本次日志和 `.werr`，避免 Fortran 报错却零退出时错误放行。后处理失败保留拟合 `.wout/.chk/.ukk` 及收据，下次公开命令只重试缺失后处理；旧后处理日志和产物归档。依据见 [0023 验收](../docs/specs/0023-epw-wannier-native-start/acceptance.md)。

`[wannier] guiding_centres = true` 可显式启用原生 W90 的中心相位引导；`false` 显式关闭。不设置时保留原生默认值和旧科学身份。程序同时核对原生日志中的实际采用值；修改此项只使 Wannier 及依赖它的阶段更新，完整相容的 QE／PH 继续复用。它用于局域化数值控制，不代替原生收敛和数据完整性检查，也不保证任意材料收敛。依据见 [0030](../docs/specs/0030-wannier-guiding-centres/acceptance.md)。

独立 Wannier 交付同时校验并绑定 `.bvec/.mmn` 的网格、排除后带空间及完整有限数记录；粗网格只复制同一收据绑定的变换与配套数据。独立绘图关闭 `write_bvec`，保护已绑定文件。旧收据未绑定配套数据时只将第 5 步及其下游判为需要更新，原现场归档，QE／PH 保留；已绑定文件损坏仍报错。见 [0033](../docs/specs/0033-wannier-velocity-handoff/acceptance.md)。

Slurm 日志属于运行输出；旧原生输入记录中的标准 `slurm-数字.out/err` 项不作为固定科学输入核验。两种阶段归档共用当前日志同 inode 入口维护，见 [0035](../docs/specs/0035-native-retry-slurm-logs/acceptance.md)。EPW 会改写粗网格自己的 `out/prefix.wfcN` 工作副本；只从已冻结且校验通过的原生输入识别前缀与 `./out`，保留初始身份用于追溯，失败后归档并从已校验来源重建。原始 NSCF、portable SAVE、响应及其余输入仍严格核验，见 [0036](../docs/specs/0036-epw-mutable-wavefunction-scratch/acceptance.md)。

第 6 步核验并复制第 5 步 `.ukk`，设置 `wannierize=false` 执行粗网格 EPC，生成电子、声子对比及四项常规数据最大差值。第 7 步生成细网格矩阵元、α²F、λ；第 8 步根据 `superconductivity.task` 运行指定温度能隙或既有 Tc 搜索。严格元素／轨道投影继续暂缓，图表和差值只展示事实，科学合理性由用户判断。

## 声子后处理与作业日志

编号声子目录的 q2r／matdyn 失败后，重新执行 `epw_phonon -c INPUT -s`（或依赖声子的后续阶段公开命令），程序先重验已发布的 QE 数据清单；科学身份相容且 save 完整时复用 PH，只继续后处理。q2r 从清单绑定的 save 副本恢复 dyn0，不把原文件复制到自身。缺少交付检查点时不能仅凭 PH 日志中的 JOB DONE 复用，损坏清单或文件明确停止。见 [0028](../docs/specs/0028-phonon-postprocess-checkpoint/acceptance.md)。

阶段归档保留当前作业 Slurm 日志的硬链接，Slurm 已打开的句柄继续写入同一文件，阶段目录中可持续查看日志；历史文件不删除。见 [0029](../docs/specs/0029-active-slurm-log-archive/acceptance.md)。

## 电子后处理失败后恢复

编号工作区分别记录 `pw.x bands`、`bands.x`、`dos.x` 的输入、真实上游与产物哈希，记录保存在 `3.qe-electronic/receipts/`。后处理失败后，复用校验通过的 PW 保存树，只重试未完成的原生任务；MPI 数量变化不要求重跑成功的 PW。科学参数不相容时更新受影响步骤，证据损坏则停止并保留现场。

`bands.x` 和 `dos.x` 各自使用当前 MPI 启动数量及 `-pd .true.`，避免读取保存数据时 slab FFT 出现空进程；不会继承 PW／EPW 的 pool 参数。其他程序参数保持独立，不改变物理网格、能窗或截断能。

```bash
epw_wannier --stage wannier --resume -s
```

0018 旧现场的 PW 能带可在完整核验冻结来源、原生输入、零退出码、日志、XML 及全部 portable 波函数后接入；缺证据明确停止。失败日志移到 `epw-state/history/`，成功结果保持原位。修改过的旧 `submit.sh` 仍会被哈希检查拒绝，重新执行上述公开命令生成新尝试，不修改旧清单。PW 粒度恢复见 [0020](../docs/specs/0020-qe-electronic-postprocess-resume/acceptance.md)，DOS 参数与新尝试恢复见 [0021](../docs/specs/0021-qe-dos-pencil-resume/acceptance.md)。已绑定作业的旧脚本不能用 `sbatch` 重提；重新执行公开 `-s`（可加 `--resume`）会创建独立尝试，复用成功的 PW 和 bands，从 DOS 等未完成任务继续。

正常入口与阶段执行已移除 `epw-state/control.lock`、`epw-state/execution.lock` 及阶段 `execution.lock` 的获取和恢复。旧残留不需要手动删除，也不会因为它们存在而拒绝重复命令；同时写同一工作区不再受这些锁互斥。顺序重复调用仍依据作业记录处理，活动或提交回执未知时禁止重提。预算账本、注册表及其锁和其他用途的锁保持原样。

同一阶段的独立产物最多使用 8 路完整哈希核验，流式读取使用 1 MiB 堆缓冲；减少共享盘串行等待，不省略文件或跨调用缓存。缺失与损坏仍分别处理，见 [0041](../docs/specs/0041-stage-output-verification-io/spec.md)。

## 常规对比最大差值（0043）

不再生成 `quality-sources`、独立参考 NSCF／直接 PH／dynmat、抽样点、质量报告或绑定清单；`[interpolation.quality]` 已删除，旧配置中的该节会被拒绝。粗 EPC 不再输出抽样 band_plot，常规绘图重读仍保留。

`comparison/metrics.json` 的 `differences` 分别保存电子路径、电子 DOS 网格、声子路径、声子 DOS 网格最大绝对差值。电子使用共同 Fermi 零点和实际 Outer 显示窗，较少一侧全部匹配较多一侧的有序子集，使平方差之和最小，记录配对与未配对态数；这种能量配对不代表同一本征态。声子以 q2r/matdyn 为 QE 对照，在相同 q 点比较全部 3N 个排序频率，统一 cm⁻¹ 并保留虚频符号。

每项包括坐标、带／排序模式编号和两侧原值；路径最大值标紫，DOS 图注明最大值的网格坐标。无配对时最大值为 null，终端显示“无可配对数据”。差值没有阈值、RMS、评级或告警，也不影响退出码、发布和下游执行。文件、来源、单位、坐标、有限数、预算、原生执行和求解收敛检查继续有效。

新比较包使用版本 2，四张独立图和总览仍可自包含移动重绘；版本 1 历史包可只读重绘，不补算四项差值。历史工作区、冻结记录和旧报告保持；已有 SCF／NSCF／PH 可通过 `--import-work` 导入新编号工作区，导入不复制抽样参考。默认安装不随源码更新。设计及验证见 [0043](../docs/specs/0043-epw-comparison-max-differences/acceptance.md)，取代 [0040](../docs/specs/0040-interpolation-quality-warnings/spec.md) 的独立核验要求。

指定温度能隙使用以下超导段设置，保留原 TOML 的其他物理和程序参数：

```toml
[superconductivity]
task = "gap"
mode = "both"
gap_temperatures_k = [20.0, 30.0, 40.0]
initial_gap_mev = 10.0
mixing_beta = -0.3
```

仍执行 `epw_superconductivity -c input.toml --stage superconductivity -s`，作业结束后重复同一命令。每个模式和温度仅运行一次非线性 EPW，不做 Tc 搜索或双初值分类。`mode` 可选 `iso`、`aniso`、`both`；单初值缺省 10 meV。温度列表不得为空、重复或超出 0.01～999.99 K，温度精度为 0.01 K；频率截断须容纳首个 Matsubara 频率。负混合系数为原生线性混合、正值为 Broyden，绝对值须在 `(0,1]`。

指定温度清单为 schema 3，独立记录 `execution_complete` 与 `data_complete`。正常结束但未收敛／能隙无效的点仍继续后续温度，有效点才进入能隙表和图，不补零或插补温度。第 8 步 `results/points.csv` 给出逐点执行、收敛与能隙有效性，`gap_summary.csv`／`gap_states.csv` 和 `figures/` 保存有效数据；不生成 Tc 表和区间图。失败须显式 `--resume`，成功点重验后复用，活动／未知任务拒绝重提。省略 `task` 仍为原 Tc 模式，继续要求双初值和所选模式的 Tc 参数表；旧清单及严格 Tc 验证保留。详见 [0045](../docs/specs/0045-epw-gap-temperatures/spec.md)。

超导数值读取支持标准 E/D 指数和原生三位指数省略 E 的格式（如 1.7366215176-102）；仅接纳带小数点的明确尾数，星号、非有限及溢出仍拒绝。格式兼容不改变原生收敛、频率／状态映射和双初值判据，见 [0042](../docs/specs/0042-epw-fortran-exponent/acceptance.md)。

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

`operation_status=ok` 表示这次检查操作通过；`workflow_status=configured` 表示配置及脚本已准备。两者都不表示 QE／EPW 已运行、计算收敛或 Tc 已得到。`--status` 的 `reuse` 同时要求科学键、上游产物键和本阶段输出哈希有效，`update` 表示当前请求需要执行。活动任务和终态未知任务会阻止再次生成或提交，以保护现场。

## 改参和复用

| 修改 | 更新范围 |
| --- | --- |
| MPI、pool、分区、时限 | 只更新后续提交资源，已有相容科学结果保留 |
| `interpolation.k_grid/q_grid`，即 nkf／nqf | 第 7、8 步 |
| Wannier 能窗、初始投影、函数数目 | 第 5～8 步 |
| `phonon.q_grid`，即粗 nq | PH、声子后处理及第 6～8 步；SCF、NSCF、Wannier 保留 |
| 结构、赝势、截断能 | 从最早受影响的上游开始 |
| 比较展宽 | 原生 DOS 后处理及重绘，不重做拟合、PH 或 EPC |

路径点数改变需要补齐相应路径参考；它不改变已有 Wannier 变换。失效现场移动到 `epw-state/history/`，不会混合新旧数据。已成功的原生子尝试重验后复用，失败子尝试归档后重试。文件损坏、来源冲突和不可解析数据仍明确停止；差值大小不影响继续执行，不凭完成标记跳过来源核验。

每次提交在 `epw-state/attempts/` 保存本次完整配置和输入副本；排队期间改根配置不会改变已提交任务。后续提交从新的根配置建立新尝试，不再受旧 64 进程快照限制。当前整工作区串行，活动或未知任务不会重复提交。共享预算路径在配置中保留，用户无需手动配置 `LEDGER` 变量；累计消费不重置。

## 并行预检

生成、提交前和脚本启动时均校验 Slurm ntasks、MPI 数、pool 和 image。EPW 当前支持单 image，要求一个 MPI 对应一个 pool，且 pool 数不超过该次可分配 k 点数。粗网格读取实际 NSCF 点数，细网格使用本次 nkf。QE 采用自己的整除规则，允许每 pool 多个 MPI。

例如完整粗网格 `4×4×2=32` 点配 `MPI=64, pool=64` 会打印网格、32 点、64 MPI／pool 和空 pool 原因，然后非零退出。程序不会调整物理网格或并行参数来绕过错误。完整原生粗矩阵按 EPW 5.9 便携格式检查；`.ukk` 额外检查 k 点数和保留能带。失败的细矩阵重启现场不会带到不同资源的新尝试；完整细矩阵由原生求解器按原分片重新分配。

## 旧工作区导入

一次性导入前，在新根配置设置 `layout="numbered"`、当前目标 `work_dir`，保持要复用的科学参数与旧 QE 相容，资源可以不同：

```bash
epw_phonon --import-work epwflow/work
```

导入核验旧运行绑定、完整 QE 清单、文件集合与 SHA-256，复制 SCF、NSCF、三模式 PH 归集和已有声子后处理，建立新阶段清单和来源记录。旧来源不删除、不改写；已有 QE 不会重算。导入日志保存于 `epw-state/legacy-import.json`，中断的受控物化可根据 `importing.json` 继续。目标已有未知同名目录或旧任务未结束时停止，不覆盖。

未设置 `layout` 的历史工作区仍保留原固定绑定行为，原有 `--resume -w 旧work --mpi ...` 只服务旧失败粗阶段。日常新流程使用根配置；历史接口细节见 [0008 三程序设计](../docs/specs/0008-epwflow-foundation/three-programs.md)。

## 构建和验证

```bash
bash scripts/build-native.sh build -p epwflow --locked
# 先准备与计算节点兼容的原生运行制品，再仅安装本模块：
bash scripts/install-bin.sh --module epwflow /兼容制品目录
bash scripts/build-native.sh test -p epwflow -p qeflow --locked
```

`bin/` 已被 Git 忽略，按 `epwflow`、`qeflow`、`scflow`、`substitutionflow`、`vaspflow`、`wannierflow` 分组。`scripts/env.sh` 将模块路径加入当前终端 PATH，不要求在测试目录寻找可执行文件。

公开进程及脚本测试使用隔离替身，证明调度、布局、复用和数据防护，不认证科学数值。真实 MgB₂ 全链和原生 iso／aniso Tc 仍需真实科学验收；本需求不代用户提交真实任务。预算、来源保护及原生收敛检查继续有效，历史科学证据见 [0015 验收](../docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md)。


## 可见运行状态与删除阶段后重建（0025）

新版源码将每个编号阶段的完成回执放在本阶段 `completion.json`，第 3 步原生任务回执放在 `3.qe-electronic/receipts/`，第 5／6 步后处理回执为本阶段 `post-completion.json`。共享配置快照、提交尝试、活动任务、交付清单、锁和归档位于可见的 `epw-state/`；普通 `ls` 即可查看。默认全局预算注册表为 `~/epwflow-state/`，首次写入会保留旧默认注册表的 campaign 绑定；显式环境覆盖继续有效。预算账本位置仍由原配置指定，不清零累计消费。

```text
run/
├── 1.qe-scf/completion.json
├── 2.qe-nscf/completion.json
├── 3.qe-electronic/{completion.json,receipts/}
├── 4.qe-phonon-nosplit/completion.json
├── 5.epw-wannier/{completion.json,post-completion.json}
├── 6.epw-coarse/{completion.json,post-completion.json}
├── 7.epw-fine/completion.json
├── 8.epw-superconductivity/completion.json
└── epw-state/{active.json,attempts/,requests/,history/}
```

这些完成文件只在对应工作完成后存在。删除阶段目录或缺少必要产物时，该阶段显示 `update`；下一次正常提交重建该阶段，保留相容上游，并撤下过期下游回执。内容被修改、记录损坏、活动或未知提交仍会停止。状态查询只读，不会因查询自动迁移或重建。

已有 `.epwflow/` 的旧编号工作区需要先执行一次显式迁移：

```bash
epw_wannier --migrate-state
epw_wannier --stage wannier -s
```

迁移核对原任务终态及锁，保留原隐藏目录和预算原文，完整历史复制到 `epw-state/history/legacy-state/`。迁移输出列出 `reuse` 和 `rebuild`：不依赖隐藏路径的完整相容上游可继续复用；绑定旧隐藏路径的粗／细交付和旧冻结脚本仅作历史保存，后续须由公开命令重建，不改写哈希冒充新交付。迁移不会提交作业。未完成事务阻止正常运行，可再次执行迁移接续；不自动覆盖未知暂存现场。

旧非编号工作区继续使用 `--import-work` 导入新编号工作区。本项源码及隔离回归不代表已安装程序或现场数据已同步；安装与真实计算另行核对。验收依据见 [0025](../docs/specs/0025-epw-visible-run-state/acceptance.md)。


显式声子路径使用专用 matdyn 解析：QE 7.4.1 的未使用第四列可未初始化，按固定宽度跳过该列，坐标、全部频率和点序仍严格核验；普通加权格式保持有限非负权重检查。编号自动比较的原 CLI 身份仅作历史追溯，升级后复用完整科学来源与成功原生子任务，冻结请求保持不变。依据见 [0037](../docs/specs/0037-matdyn-path-unused-weight/acceptance.md)、[0038](../docs/specs/0038-comparison-orchestrator-upgrade/acceptance.md)。

共用比较渲染器将 G/GAMMA 显示为 Γ，在屏幕上分组合并拥挤的相邻端点文字；所有原竖线和科学曲线坐标保持。独立图与总览均使用该规则，显式 --plot 可离线重绘既有结果，见 [0039](../docs/specs/0039-comparison-path-labels/acceptance.md)。
