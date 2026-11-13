# epwflow

仅支持按八阶段填写原生参数的 `schema_version=2`；名称、单位与实际输入对应关系见[参数说明](../docs/overview/epwflow/native-stage-config.md)。2026-10-26 的固定私有构建完成 PBE MgB₂ 八阶段及 iso／aniso 的 20／30／40 K 六点能隙验收；2026-10-27 原始声子导入版本已发布至正式入口，具体构建和 SiP₂ 验收见 [0010](../docs/specs/0010-epwflow-qe-data/acceptance.md)。Tc 与网格收敛不在这些结论中。

编号工作区新原生脚本使用 `<功能>.sh`（如 `scf.sh`、`bands.sh`、`epw_wannier.sh`），Slurm 两路日志合并到 `slurm-<功能>.log` 并覆盖写入。公开命令在实际计算目录调用 sbatch；脚本仅用 `#SBATCH --chdir=绝对目标目录`，正文没有 cd，也不重复传命令行 --chdir。手工可从其他目录通过脚本绝对路径提交，科学输出和相对 Slurm 日志仍在目标目录；`squeue -o "%i %j %Z"` 可查看 WorkDir。SLURM_SUBMIT_DIR 仍表示调用 sbatch 的原目录。直接 bash 执行不会解释 #SBATCH，需由调用方指定工作目录。历史冻结脚本保持原字节，细节见 [0046 验收](../docs/specs/0046-epw-submit-simplification/acceptance.md)。

## MPI 启动器

配置使用 `launcher = ["mpirun", "-np", "32"]`，并在 `programs.environment_script` 指定已有环境脚本。新作业先加载环境，再普通调用 `mpirun`；即使旧配置含 MPI 目录，新脚本也使用 PATH 查找，不增加 `exec`。登录节点无需先激活 MPI。科学程序路径和原参数仍按配置；其他自定义启动器沿用原规则。

当前编号工作区直接更新阶段现场，原文配置版本单独保留；新原生任务不再生成 submit.sh 阅读副本。[0044 规格与部署](../docs/specs/0044-mpi-submit-command/spec.md)。

## 计算时长与存储

DFTFlow 不额外限制计算时长和存储空间，集群自身规则仍由 Slurm 管理。历史账本保留消费、作业身份和并发登记；活动、未知作业检查继续有效。

## Slurm 文件头

与 QE／VASP 等模块统一使用 `[slurm].header`，删除整个 `[resources]`。MPI 参数仅在 launcher 中填写，Slurm 申请由 header 明确设置；程序不从 launcher 推导资源指令，也不通过 sbatch 参数覆盖资源申请。

```toml
[programs]
launcher = ["mpirun", "-np", "2"]

[slurm]
header = """
#SBATCH --partition=local
#SBATCH --nodes=1
#SBATCH --ntasks=2
"""
```

header 原文保留；默认作业名和日志指令位于它之前，用户同名指令优先。未填写 header 时使用 Slurm 缺省资源，不自动增加分区、节点数或任务数。MPI 进程数可以小于分配的任务数，不再强制相等；EPW 原生 pool／image 与实际 k 网格的必要检查仍保留。旧 `[resources]` 直接报错，应迁移源输入并重新生成脚本，不手改冻结快照。公开 `--nodes`／预算 `--mpi` 已移除，恢复时修改源配置中的 launcher 及原生 pool 参数。

构建成功后直接覆盖固定 `bin/<模块>/` 的命令，不保留编译版本目录或版本记录。详见 [0065](../docs/specs/0065-slurm-header-launcher/spec.md)。

## 编号工作区：原生任务与阶段交付

第 3／4 步提交一个阶段分配，在分配内完成依赖、原生后处理、核验、绘图和发布。第 1／2 步及第 5～8 步的 `-s` 直接提交就绪的原生任务；作业结束后重复同一公开命令，核验并推进下一子任务或交付。指定温度 `task="gap"` 为每个方法独立提交原生多温度作业，后续公开调用归集结果。`-r` 在已有 Slurm 分配内同步推进所选阶段。各步交付可从工作区根 `results/` 查看。

第 3 步两个 QE 子任务各完成即发布；第 5 步完成电子能带和总 DOS 叠图，不等待声子；第 6 步完成声子谱、声子 TDOS 和复用电子数据的总览。流程与恢复见 [0050](../docs/specs/0050-epw-stage-results/quickstart.md)及 [Overview 流程图](../docs/overview/epwflow/workflow.md)。

```bash
epw_phonon -c input.toml -s
epw_wannier -c input.toml -s
epw_superconductivity -c input.toml -s
```

阶段成功发布后再报告完成。失败修复后用相同目标加 `--resume -s`，成功科学检查点继续复用，只补失败后处理、缺图或发布。排队、运行中或终态未知时拒绝重提。Tc 沿用原搜索规则、双初值和次数上限，搜索进度写第 8 步日志，不扩大科学参数范围。

输出给出实际提交的功能脚本、作业号与工作目录。新原生任务的科学程序 stdout／stderr 均直接写指定 `.out`，运行时即可查看，不产生独立科学 `.err`；Slurm 自身日志指令沿用原设置。Wannier／postw90 保留种子参数接口。新脚本的命令类似：

```bash
#SBATCH --chdir=/path/to/1.qe-scf
source "/path/to/oneapi/setvars.sh" --force
mpirun -np 32 /path/to/pw.x < scf.in > scf.out 2>&1
```

当前任务、科学检查和续算记录统一写 `epw-state/` 八份中文阶段日志，Slurm 作业号仅放各计算目录同名 `.jobs`。电子后处理位于 `5.epw-wannier/comparison-native/electron/`，能带由 EPW 内嵌 Wannier90 输出，DOS 调用 postw90，原模型保持只读。

`epw_phonon --all` 只准备下一就绪脚本与 `native-plan.json` 待完成清单，零提交，不生成自动全流程总脚本。`--status [--json]` 只读区分运行中、已结束待核验、失败及已核验。

可手工修改脚本并 `sbatch` 独立 debug，但它不产生正式完成记录；准备或活动任务的脚本被修改时，正式命令仍拒绝接管。版本 2／3 任务结束并完成核验后，覆盖功能脚本或 Slurm 日志不影响科学复用；输入、来源和科学输出仍严格校验。版本 3 的两路日志身份指向同一 `.out`；版本 1／2 历史任务继续读取并验证原分离日志，不能删除其 `.err`。普通本地执行器保留原日志行为。旧封存脚本和默认安装不随源码修改自动更新。旧非编号布局的新 `-s` 拒绝继续提交包装脚本；使用新的编号工作目录并通过 `--import-work` 显式导入。`-r` 的既有分配执行路径保留。日志契约和验证见 [0053](../docs/specs/0053-epw-merged-program-output/acceptance.md)。

以下早期脚本／自动串联说明属于旧路径；编号工作区以本节为准。[0026 设计与验收](../docs/specs/0026-epw-direct-native-submit/plan.md)。

三个公开程序管理 QE 到超导的八个阶段。编号工作区使用根目录 `input.toml`，每次生成、提交或 `--resume` 都读取当前配置，按阶段核验参数和上游文件哈希，复用相容结果。

显式 `epw_phonon -c "epwinput.toml" --stage scf --resume -s` 强制覆盖已退出的 SCF 并开始新批次；保留目录 inode、脚本及打开的 Slurm 日志，不归档旧内容。活动或提交未知的作业不能覆盖。普通 `--stage scf -s` 核验并复用相容结果。

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run
# 修改 input.toml 后执行需要到达的阶段：
epw_wannier --stage wannier -s
```

MgB₂ 当前目录与数据导入说明见 [算例说明](../examples/MgB2/sugon-amd/run/README.md)。完整需求和验收见 [0018](../docs/specs/0018-epwflow-stage-workspaces/spec.md)；当前功能入口为 [Overview](../docs/overview/epwflow/README.md)。

## 清理历史计算

在编号工作区根执行，默认只预览；`--yes` 实际清空三类历史的全部内容：

```bash
epw_clear --dry-run
epw_clear --yes
# 在其他目录指定工作区：
epw_clear --work "/path/to/epwflow" --dry-run
```

八日志布局不再生成历史，`epw_clear` 报告无需清理。下列历史清理限制仅用于仍采用 `layout.json` 的旧工作区；现用计算、结果、配置版本及预算保留。

活动／未知作业、现用记录或链接引用历史、清理根及其祖先为链接时，删除前拒绝并说明具体原因；历史内部链接只删除链接本身。删除失败返回非零并列出路径。清理与计算／输入生成／恢复顺序执行。完整参数及边界见 [epw_clear](../docs/overview/epwflow/epw_clear.md)，验证见 [0057](../docs/specs/0057-epw-clear-history/acceptance.md)。

## 输入与布局

配置使用 `schema_version = 2`，以八个计算目录名为主阶段。主段直接填写主调用的原生参数，其他调用放在用途子段，例如 `[stages."5.epw-wannier".wannier90]` 与 `.postw90`。程序内部识别输入段；历史同层独立调用的版本 2 仍可读取，并保持原格式序列化；普通入口不接受早期程序／namelist 多层格式；显式 `--import-phonon` 可在严格核验旧 SCF/NSCF 后受控接续嵌套的版本 2 配置。相对路径按 TOML 所在目录解释，`work_dir = "."` 表示当前工作区。

```toml
schema_version = 2
layout = "numbered"
work_dir = "."
structure = "POSCAR"
kpath = "KPATH.in"

[stages."1.qe-scf"]
ecutwfc = 40.0  # Ry
ecutrho = 320.0 # Ry
occupations = "smearing"
smearing = "gaussian"
degauss = 0.02  # Ry

[stages."2.qe-nscf"]
ecutwfc = 40.0  # 此次 pw.x 独立设置，不从 SCF 继承
# 其余科学参数也须分别填写；本片段不是完整配置。
```

八主阶段完整示例见 [SiP₂ 配置](../examples/SiP2/epwinput.toml)，历史平层示例见 [MgB₂ 配置](../examples/MgB2/sugon-amd/run/epwflow/epwinput.toml)，字段和单位见[原生阶段参数说明](../docs/overview/epwflow/native-stage-config.md)。所有科学调用读取对应阶段配置；修改 NSCF 截断能不会改写 SCF。数据交接必须匹配的前缀、粗细网格、Wannier 维数等出现冲突时，执行前列出两处配置和值。顶层 `workflow` 单独保存自动选窗、输入来源、PH 拆分及搜索设置。自动绑定在程序内部登记，无需填写 derived 表；同一目标同时手填和内部推导会被拒绝。第 3 步使用 `3.qe-electronic` 主段及 `.bands/.dos/.projwfc` 子段，第 5 步使用 `5.epw-wannier` 主段及 `.wannier90/.postw90` 子段。新旧主段不能混写，未知用途、程序／namelist 的额外嵌套明确拒绝。配置简化及验证见 [0061](../docs/specs/0061-epw-flat-stage-config/spec.md)。

数组使用原生名，例如 `amass`、`temps`，生成 Fortran 索引数组；`ATOMIC_SPECIES` 保存原生卡片文本；SCF 的 `K_POINTS` 保存类型及正文，NSCF 的 `K_POINTS = [A, B, C]` 仅填写三个正整数。Rust 按官方 `kmesh.pl` 的公式、顺序及精度展开到实际 `nscf.in`，配置中不保存长点表。SCF 与 NSCF 必须同维度、零位移。输入文件不再前置参数来源注释；原文配置使用编号版本保存，当前及计算使用的版本由阶段日志说明；内部算法类型不是另一份用户配置。

```text
run/
├── input.toml、POSCAR、pp/、KPATH.in
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
├── 7.epw-fine/
├── 8.epw-superconductivity/
│   ├── iso/
│   └── aniso/
├── results/                 # 按阶段即时发布图、数据、参数及来源
│   ├── index.json
│   ├── 3.qe-electronic/
│   ├── 4.qe-phonon-nosplit/
│   ├── 5.epw-wannier/
│   ├── 6.epw-coarse/
│   └── 8.epw-superconductivity/
└── epw-state/
```

实际计算目录保存运行到该任务时生成的原生输入、功能脚本和日志。分 q、细网格展宽及各温度尝试也在自己的目录生成提交脚本。依赖上游产物才能确定的输入和子任务在运行到该阶段时生成；未得到 NSCF 时不会伪造 EPW 选窗。脚本固定工作目录，能够从其他目录提交。一个阶段分配只执行目标范围内缺少的任务，不生成自动推进全部物理阶段的总提交脚本。

第 1～4 步复用 qeflow／共享输入、执行及后处理组件，SCF 只做一次并保留独立快照。NSCF 为 EPW 所需的完整均匀网格。`qe_eband` 使用 `pw.x bands` 和 `bands.x`，`qe_eledos` 使用 `dos.x`。PH 三种模式都严格校验 q 点、动力学矩阵、dvscf 和 phsave 后归集到第 4 步 `save/`。

第 5 步由 EPW 内嵌 Wannier90 拟合并通过 `wdata` 输出能带，按真实 `*_band.kpt` 核对共同点位；不同晶格方向导致直线段不能有效合并时，采用完整目标坐标的端点配对压缩输入，再按坐标恢复原路径，不减少路径/网格目标点，也不取消原有容量检查；不再执行独立 `wannier90.x restart=plot`。只有电子 DOS 使用 `[programs.postw90]`，缺省为 `pw.x` 同目录的 postw90；读取同次拟合模型，不重新拟合。QE 路径参考读取第 3 步，DOS 参考读取第 2 步均匀网格；两侧保持相同网格、Fermi 零点、Gaussian 核宽及非磁标量每态两电子计数。实际四个能窗、路径分段、最大差和未配对态同时保留。图和来源发布到 `results/5.epw-wannier/`，原始模型留在第 5 步。

独立拟合输入不声明粗网格插值使用的 `filkf/filqf`。后处理自动从校验后的同一 NSCF 提取晶格、原子、网格、原 k 点顺序及排除后的能带，补齐 DOS 所需 postw90 的 `.win/.eig`；无需用户维护另一份输入。原生 DOS 使用 `dos_smr_type = gauss`。除退出码外，还检查本次日志和 `.werr`，避免 Fortran 报错却零退出时错误放行。后处理失败保留拟合 `.wout/.chk/.ukk`，进度归第 5 步日志；下次公开命令只重试缺失后处理，直接替换失败产物，保留打开的 Slurm 日志，不创建归档。历史依据见 [0023 验收](../docs/specs/0023-epw-wannier-native-start/acceptance.md)，当前状态布局见 [0064](../docs/specs/0064-epw-readable-state-logs/acceptance.md)。

`[stages."5.epw-wannier".wannier90] guiding_centres = true` 可显式启用原生 W90 的中心相位引导；`false` 显式关闭。不设置时保留原生默认值和旧科学身份。程序同时核对原生日志中的实际采用值；修改此项只使 Wannier 及依赖它的阶段更新，完整相容的 QE／PH 继续复用。它用于局域化数值控制，不代替原生收敛和数据完整性检查，也不保证任意材料收敛。依据见 [0030](../docs/specs/0030-wannier-guiding-centres/acceptance.md)。

独立 Wannier 交付同时校验并绑定 `.bvec/.mmn` 的网格、排除后带空间及完整有限数记录；粗网格只复制同一收据绑定的变换与配套数据。DOS 后处理关闭 `write_bvec`，保护已绑定文件。旧收据未绑定配套数据时只将第 5 步及其下游判为需要更新，原现场归档，QE／PH 保留；已绑定文件损坏仍报错。见 [0033](../docs/specs/0033-wannier-velocity-handoff/acceptance.md)。

Slurm 日志属于运行输出；旧原生输入记录中的标准 `slurm-数字.out/err` 项不作为固定科学输入核验。两种阶段归档共用当前日志同 inode 入口维护，见 [0035](../docs/specs/0035-native-retry-slurm-logs/acceptance.md)。EPW 会改写粗网格自己的 `out/prefix.wfcN` 工作副本；只从已冻结且校验通过的原生输入识别前缀与 `./out`，保留初始身份用于追溯，失败后归档并从已校验来源重建。原始 NSCF、portable SAVE、响应及其余输入仍严格核验，见 [0036](../docs/specs/0036-epw-mutable-wavefunction-scratch/acceptance.md)。

第 6 步核验并复制第 5 步 `.ukk`，设置 `wannierize=false` 执行粗网格 EPC，生成声子谱、声子 TDOS 叠图，并读取第 5 步电子数据生成四面板总览和事实最大差值。第 6 步不新增电子参考 PW；声子后处理核对 ASR，不相容时事前说明最少补算原因，成功力常数与频率分别缓存。第 7 步生成细网格矩阵元、α²F、λ；第 8 步根据 `stages."8.epw-superconductivity".workflow.task` 运行指定温度能隙或既有 Tc 搜索。第 3 步提供 QE 元素电子 PDOS；插值侧元素／轨道投影仍未提供。图表和差值只展示事实，科学合理性由用户判断。

## 声子后处理与作业日志

编号声子目录的 q2r／matdyn 失败后，修复原因并执行一次 `epw_phonon -c INPUT --stage phonon --resume -s`，程序直接重验第 4 步 save 与阶段日志，自动继续余项。参数相容且响应完整时复用 PH；q2r 从已核验的 save 副本恢复 dyn0，缺数值输出时只补对应子任务。不能仅凭 JOB DONE 宣告完成，损坏文件明确停止。历史依据见 [0028](../docs/specs/0028-phonon-postprocess-checkpoint/acceptance.md)及 [0048](../docs/specs/0048-epw-phonon-stage-chain/quickstart.md)，当前协议见 [0064](../docs/specs/0064-epw-readable-state-logs/spec.md)。

重算保留阶段根目录与打开的 Slurm 日志 inode，已打开的句柄继续写入同一文件，阶段目录中可持续查看日志；其他可替换内容直接覆盖，不创建历史目录。目录与日志保护的历史依据见 [0029](../docs/specs/0029-active-slurm-log-archive/acceptance.md)。

## 电子后处理失败后恢复

编号工作区将 `pw.x bands`、`bands.x`、`dos.x` 及投影进度记录在 `epw-state/3.qe-electronic.log`，任务号保存在第 3 步 `.jobs`，不保存 `receipts/`。后处理失败后，直接重验并复用成功 PW 保存树，只重试未完成的原生任务；MPI 数量变化不要求重跑成功的 PW。科学参数不相容时更新受影响步骤，原生数据损坏则说明具体原因并停止。

`bands.x` 和 `dos.x` 各自使用当前 MPI 启动数量及 `-pd .true.`，避免读取保存数据时 slab FFT 出现空进程；不会继承 PW／EPW 的 pool 参数。其他程序参数保持独立，不改变物理网格、能窗或截断能。

```bash
epw_wannier --stage wannier --resume -s
```

电子 PW 能带、bands、DOS 与投影分别在第 3 步日志记录续算。直接校验实际输入、原生日志、XML 和保存树；后处理失败只接续缺失子项，成功 PW 数据保留。不创建失败历史或额外 receipts。

工作区推进和日志合并锁定稳定目录 inode，进程结束自动释放，不创建永久工作流锁文件。日志和 `.jobs` 保留提交中断与未知回执，活动／未知时禁止重提。预算及结果发布锁各自保留。

同一阶段的独立产物最多使用 8 路完整哈希核验，流式读取使用 1 MiB 堆缓冲；减少共享盘串行等待，不省略文件或跨调用缓存。缺失与损坏仍分别处理，见 [0041](../docs/specs/0041-stage-output-verification-io/spec.md)。

## 常规对比最大差值（0043）

不再生成 `quality-sources`、独立参考 NSCF／直接 PH／dynmat、抽样点、质量报告或绑定清单；`[interpolation.quality]` 已删除，旧配置中的该节会被拒绝。粗 EPC 不再输出抽样 band_plot，常规绘图重读仍保留。

`comparison/metrics.json` 的 `differences` 分别保存电子路径、电子 DOS 网格、声子路径、声子 DOS 网格最大绝对差值。电子使用共同 Fermi 零点和实际 Outer 比较指标窗；图像另用阶段默认／完整价带显示窗，较少一侧全部匹配较多一侧的有序子集，使平方差之和最小，记录配对与未配对态数；这种能量配对不代表同一本征态。声子以 q2r/matdyn 为 QE 对照，在相同 q 点比较全部 3N 个排序频率，统一 cm⁻¹ 并保留虚频符号。

每项包括坐标、带／排序模式编号和两侧原值；路径最大值标紫，DOS 图注明最大值的网格坐标。无配对时最大值为 null，终端显示“无可配对数据”。差值没有阈值、RMS、评级或告警，也不影响退出码、发布和下游执行。文件、来源、单位、坐标、有限数、作业登记、原生执行和求解收敛检查继续有效。

新比较包使用版本 2，四张独立图和总览仍可自包含移动重绘；版本 1 历史包可只读重绘，不补算四项差值。历史工作区、冻结记录和旧报告保持；已有 SCF／NSCF／PH 可通过 `--import-work` 导入新编号工作区，导入不复制抽样参考。默认安装不随源码更新。设计及验证见 [0043](../docs/specs/0043-epw-comparison-max-differences/acceptance.md)，取代 [0040](../docs/specs/0040-interpolation-quality-warnings/spec.md) 的独立核验要求。

指定温度能隙在第 8 步分别填写两个方法的完整原生输入。以下仅展示有关字段，其他开关、网格和求解参数也必须独立填写：

```toml
[stages."8.epw-superconductivity".workflow]
task = "gap"
mode = "both"

[stages."8.epw-iso"]
muc = 0.13
nstemp = 3
temps = [20.0, 30.0, 40.0] # K
gap_edge = 0.01           # eV
broyden_beta = -0.3

[stages."8.epw-aniso"]
muc = 0.13
nstemp = 3
temps = [20.0, 30.0, 40.0] # K
gap_edge = 0.01           # eV
broyden_beta = -0.3
```

执行 `epw_superconductivity -c "input.toml" --stage superconductivity -s`，在前置数据可复用时一次提交 iso／aniso 两个独立 Slurm 作业，无方法间依赖。每种方法只调用一次 EPW，`nstemp` 和完整 `temps` 列表包含全部配置温度，无需逐温度重复命令。每个方法作业内部归集本批结果，最后完成的方法自动出图并发布到 `results/8.epw-superconductivity/`；不重算已完成点。`--status --json` 的 `native_methods` 分别显示两个作业；实际启动时间由调度器决定，每个作业使用配置资源并单独登记共享预算。

`muc`、`gap_edge` 和求解控制必须显式配置，程序不注入科学默认值。两种方法独立读取自己的参数与温度；第 7 步也有自己的 `nstemp/temps`。温度列表为 1～50 个互不重复的 0.01～999.99 K 值，按配置顺序传递；频率截断须容纳首个 Matsubara 频率。负 `broyden_beta` 为原生线性混合、正值为 Broyden，绝对值须在 `(0,1]`。

```text
8.epw-superconductivity/
├── iso-mu0.13-20-40/     # epw.in、epw.sh、epw.out/err 与所有温度原生输出
├── aniso-mu0.13-20-40/
├── manifest.json
└── results/
```

目录使用实际 μ* 和配置列表首尾温度，去掉数值冗余零，不按温度建立子目录。新点清单为 schema 4，旧 schema 3 继续核验；结果包保留 `points.csv`、有效的 `gap_summary.csv`／`gap_states.csv` 与图，不生成 Tc 表。执行完成、收敛和数据有效性分别记录，缺失或 NaN 不补零。

EPW 原生多温度循环可能因不收敛提前停止，另一方法独立运行。保留已完成前缀，明确记录未执行温度；修正原因后用同命令加 `--resume -s`，只把剩余点交给新的原生批次。每个成功温度的科学数据保持，失败温度在显式恢复时覆盖；不保留失败批次的旧输入或日志副本。活动或未知回执不重提；第二个提交失败也保留第一个作业号。`workflow.task="tc"` 显式选择原有搜索和双初值规则。设计及源码验证见 [0049](../docs/specs/0049-epw-multitemperature-submit/spec.md)，历史安装及六单点真实运行见 [0045](../docs/specs/0045-epw-gap-temperatures/acceptance.md)；初次 0049 未自动部署；后续真实双批次六点均有效，见 [联合验收](../docs/specs/0050-epw-stage-results/real-chain-validation.md)。

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
| MPI、pool、分区 | 只更新后续提交资源，已有相容科学结果保留 |
| `interpolation.k_grid/q_grid`，即 nkf／nqf | 第 7、8 步 |
| Wannier 能窗、初始投影、函数数目 | 第 5～8 步 |
| `phonon.q_grid`，即粗 nq | PH、声子后处理及第 6～8 步；SCF、NSCF、Wannier 保留 |
| 结构、赝势、截断能 | 从最早受影响的上游开始 |
| 比较展宽 | 原生 DOS 后处理及重绘，不重做拟合、PH 或 EPC |

路径点数改变补齐相应路径参考，不改变已有 Wannier 变换。受影响的派生现场直接更新，不创建历史。成功子任务重验复用，失败子任务在显式恢复后覆盖重试。格式、维度和来源相容性仍检查；差值供用户判断。

每次配置驱动 CLI 检查原文；变化后在工作区 `epw-state/` 保存下一份 `NNNN-config.toml`，连续相同不重复。阶段提交脚本引用该原文版本，排队期间编辑当前输入不会改变该分配使用的参数。日志区分当前输入和计算使用的版本；不再保存展开配置、attempts 或请求指纹。预算消费不重置。

## 并行预检

生成、提交前和脚本启动时均校验 Slurm ntasks、MPI 数、pool 和 image。EPW 当前支持单 image，要求一个 MPI 对应一个 pool，且 pool 数不超过该次可分配 k 点数。粗网格读取实际 NSCF 点数，细网格使用本次 nkf。QE 采用自己的整除规则，允许每 pool 多个 MPI。

例如完整粗网格 `4×4×2=32` 点配 `MPI=64, pool=64` 会打印网格、32 点、64 MPI／pool 和空 pool 原因，然后非零退出。程序不会调整物理网格或并行参数来绕过错误。完整原生粗矩阵按 EPW 5.9 便携格式检查；`.ukk` 额外检查 k 点数和保留能带。失败的细矩阵重启现场不会带到不同资源的新尝试；完整细矩阵由原生求解器按原分片重新分配。

## 旧工作区导入

一次性导入前，在新根配置设置 `layout="numbered"`、当前目标 `work_dir`，保持要复用的科学参数与旧 QE 相容，资源可以不同：

```bash
epw_phonon --import-work epwflow/work
```

旧工作区导入核验科学数据后物化到新编号目录，来源不删除或改写。来源和转换记录在 `epw-state/imports/`，不再输出共享 qe-data 清单或旧根层 legacy-import 状态。目标未知同名内容或旧作业未结束时停止。

未设置 `layout` 的历史工作区仍保留原固定绑定行为，原有 `--resume -w 旧work --mpi ...` 只服务旧失败粗阶段。日常新流程使用根配置；历史接口细节见 [0008 三程序设计](../docs/specs/0008-epwflow-foundation/three-programs.md)。

## 构建和验证

```bash
bash scripts/build-native.sh build --release -p epwflow --bins --locked
# 成功后已自动发布到 bin/epwflow；当前终端加载仓库入口：
source "/path/to/dftflow/scripts/env.sh"
bash scripts/build-native.sh test -p epwflow -p qeflow --locked
```

构建会先完成本机配置要求的兼容打包与启动检查，活动引用存在时拒绝替换；测试不更新已发布 CLI。详见 [0017](../docs/specs/0017-build-binaries-layout/quickstart.md)。`bin/` 已被 Git 忽略，按 `epwflow`、`qeflow`、`scflow`、`substitutionflow`、`vaspflow`、`wannierflow` 分组。`scripts/env.sh` 将模块路径加入当前终端 PATH，不要求在测试目录寻找可执行文件。

公开进程及脚本测试使用隔离替身，证明调度、布局、复用和数据防护，不认证科学数值。后续授权的真实 MgB₂ PBE 八阶段及 iso／aniso 三温度 gap 已完成，见 [联合验收](../docs/specs/0050-epw-stage-results/real-chain-validation.md)；原生 Tc 搜索不在该轮验收范围。作业登记、来源保护及原生收敛检查继续有效，历史科学证据见 [0015 验收](../docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md)。


## 八阶段可读状态与重建

`epw-state/` 固定保存八份纯文本日志：

```text
1.qe-scf.log
2.qe-nscf.log
3.qe-electronic.log
4.qe-phonon-nosplit.log    # 随实际模式改为 coords 或 index
5.epw-wannier.log
6.epw-coarse.log
7.epw-fine.log
8.epw-superconductivity.log
0000-config.toml         # 首次读取的配置原文
0001-config.toml         # 与上一版不同才新增
imports/                 # 外部声子导入时保留
```

阶段日志包含中文现状、科学结果检查、命名子任务、当前配置版本、计算目录、失败原因和下一步。各阶段目录中的同名 `.jobs` 仅记录子任务和 Slurm 作业号；不增加 `continue.log`。布局、声子分配记录在第 4 步；iso／aniso 和温度分别写第 8 步。

删除必要产物后，下一次公开推进会检查并补齐，保留相容上游；改变上游会使受影响下游待更新。结果包保留其自包含科学清单和图参，用于核验和离线重绘，不作为任务续算状态。

旧可见状态自动进行一次性退出，先核对旧作业终态，不要求已归档的旧输入仍存在。旧隐藏编号布局使用 `epw_wannier --migrate-state`，未知内容明确保留并报出路径；旧非编号数据使用 `--import-work`。不复制旧历史或重置预算。当前改动的源码、测试与安装边界见 [0064 验收](../docs/specs/0064-epw-readable-state-logs/acceptance.md)。


显式声子路径使用专用 matdyn 解析：QE 7.4.1 的未使用第四列可未初始化，按固定宽度跳过该列，坐标、全部频率和点序仍严格核验；普通加权格式保持有限非负权重检查。编号自动比较直接读取科学来源，已完成子任务的续算位置保存于第 6 步日志，不生成冻结请求。依据见 [0037](../docs/specs/0037-matdyn-path-unused-weight/acceptance.md)、[0038](../docs/specs/0038-comparison-orchestrator-upgrade/acceptance.md)。

共用比较渲染器将 G/GAMMA 显示为 Γ，在屏幕上分组合并拥挤的相邻端点文字；所有原竖线和科学曲线坐标保持。独立图与总览均使用该规则，显式 --plot 可离线重绘既有结果，见 [0039](../docs/specs/0039-comparison-path-labels/acceptance.md)。

## QE 分阶段即时出图（0047）

第 3 步每个电子子任务校验成功即生成自己的 QE 单独图：`qe_eband/plots/electronic-bands.png` 与 `qe_eledos/plots/electronic-dos.png`，同时保存 CSV。电子来源使用已核验 NSCF 的 Fermi 零点，不等待声子或 Wannier，也不要求拟合能窗。能带保留断开路径与高对称点标签。

第 4 步 q2r／matdyn 后处理结束后，在 `4.qe-phonon-<模式>/plots/` 生成 `phband/phonon_band.png`、`phdos/phonon_dos.png` 和联合图。声子 DOS 同时显示 TDOS 与按结构汇总的元素贡献，原子／元素 CSV 可追溯；必须有完整逐原子投影，缺列明确报错。

第 5 步继续生成 QE／Wannier 电子能带和总 DOS 叠图。第 6 步声子谱继续比较 QE／EPW，声子 DOS 的独立图与总览仅比较 TDOS，不展示元素曲线或元素投影缺失占位。第 3／4 步的单独图保留。QE 元素电子 PDOS 已接入第 3 步；EPW 元素／轨道 PDOS 及插值侧元素声子投影仍未提供。

`-r` 在该子任务结束时绘图。第 3／4 步 `-s` 在阶段分配内完成后处理；第 5～8 步再次调用公开命令归集原生结果、补图并发布。失败恢复沿用 `--resume`，成功子任务和绘图位置进入阶段日志，不生成独立完成回执。状态查询不补图，配置内容变化仍会触发原文备份。

实现与隔离进程／真实数据只读出图证据见 [0047 验收](../docs/specs/0047-qe-stage-plots/acceptance.md)。初次源码交付未部署或新增科学作业；后续 0046～0051 的实际全链与安装边界见 [联合验收](../docs/specs/0050-epw-stage-results/real-chain-validation.md)。

## 逐阶段结果与独立重绘

`results/` 位于计算工作区根，与编号目录同级。第 3 步的 `qe_eband/`、`qe_eledos/` 分别发布；第 4 步保存单侧声子谱、TDOS／元素 DOS；第 5 步是双窗电子四图；第 6 步是声子两图及四面板总览；第 8 步是已有超导图。第 7 步没有独立既有图，不创建空包或额外任务。每个包带表格、图参、来源和文件校验清单。

```bash
epw_superconductivity --results-check --results "results" --json
epw_superconductivity --plot --results "results"
```

完整目录移动后可独立重绘；包内核验不把已不可访问的原始来源宣称为已验源。旧版可核验的比较采样只读整理到新电子目录；缺少真实坐标／模型来源时明确报告，不能为目录迁移重拟合。源码更新不自动改安装、冻结脚本或正在运行的工作区。

## 粗网格实空间衰减图（0051）

第 6 步原生核验成功后，同一阶段分配独立生成 `decay.H`、`decay.dynmat`、`decay.epmate`、`decay.epmatp` 的四张图和 2×2 总览，先于其他比较后处理。第 5 步只拟合 Wannier，尚不产生完整四类量；不等待第 7 步，也不新增科学计算或作业。

查看 `results/6.epw-coarse/decay/decay-overview.png`，独立图主体为 `decay-H`、`decay-dynmat`、`decay-epmate`、`decay-epmatp`，全部提供 PNG／SVG／PDF。横轴为实空间距离 Å，纵轴按原生文件为 Ry，使用紫色散点和十进制对数。重复距离逐行保留，零值保留在数据并注明未绘制，坏数和来源冲突拒绝。

阶段内派生目录为 `6.epw-coarse/decay/`；发布包附原始副本、逐行绘图数据、参数及来源。完整 `results/` 移动后可用上面的 `--plot --results` 独立重绘。图像失败后原目标加 `--resume -s` 仅补派生结果，缺原生 decay 文件则列缺项，不自动重跑 EPW。图仅供人工判断，不提供阈值、评级或放行门。使用与验收见 [0051 快速说明](../docs/specs/0051-epw-real-space-decay/quickstart.md)和[验收记录](../docs/specs/0051-epw-real-space-decay/acceptance.md)；后续合并版已完成真实联合验收并部署默认安装，详见 [联合交付记录](../docs/specs/0050-epw-stage-results/real-chain-validation.md)。

## 电子图的默认双能窗与配色

第 3 步交付 4 张 QE 图，第 5 步交付 4 张 QE／EPW 对比图，共 8 张。每阶段分别提供能带与 DOS 的两种范围：默认文件 `electronic-bands.png`／`electronic-dos.png` 在第 3 步使用 E_F±5 eV，第 5 步使用实际 dis_win_min−E_F～dis_win_max−E_F；`electronic-bands-valence.png`／`electronic-dos-valence.png` 使用共同 QE NSCF 最低占据态至 E_F＋5 eV。最低态包含赝势显式半芯态，表示已计算网格的最小值。显示范围不修改实际拟合能窗或原比较指标。

全部电子图、总览电子面板及独立重绘画出红色费米线；蓝色只表示 EPW／Wannier 插值。QE 用深灰，元素和窗口用绿／橙／褐／紫等其他颜色，普通文字黑色。四个实际窗口的值仍列在图例，超出显示范围的窗口不扩大坐标轴。默认电子 PNG 从数值矢量直接生成 3000×2100，PNG 密度为 300.0248 DPI，不放大旧图片。

QE 参考曲线统一使用深灰实线，涵盖能带、DOS、声子比较及总览；EPW／Wannier 使用蓝色短虚线后绘。第 3 步 TDOS 为实线，各元素 PDOS 使用不同长度的短虚线。多曲线图中参考／插值线宽为 4∶3（约粗 33%），图例同步；电子费米线仍为红色，重绘不改变科学数据。

第 3 步 DOS 同图画 TDOS＋Mg／B 等真实元素 PDOS，第 5 步仍只比较 TDOS。投影缺失时，在已核验 NSCF 波函数的独立副本内运行必要 `projwfc -pd .true.`，不重算 PW／PH／Wannier／EPW；程序默认从 pw.x 同目录解析。TDOS 读取同次 projwfc `.pdos_tot` 的 DOS 列，保证与 PDOS 原生同网格；原 dos.x 表保留。逐 atom/wfc 的 LDOS 只累加一次，保留投影残差，不强制归一。当前仅支持非磁标量投影，缺波函数或不支持的自旋明确报错，不自动重算 NSCF。

投影失败时已成功的能带及 TDOS 图保留并注明缺项；修复后恢复只接续投影和派生产物。投影程序已成功而解析或发布失败时，从第 3 步日志和实际原生输出接续。独立重绘只读取包内 `.plot.json`，不调用原生程序。设计、调用证据和验证入口见 [0055 验收](../docs/specs/0055-electronic-plot-windows-pdos/acceptance.md)。

## NSCF 完整网格与 FFT 接续（0060）

NSCF 完成及复用时，程序直接核对实际 `nscf.in` 的声明点数和坐标行数、输出 XML 的 `nks` 和电子态记录数，并检查点序、重复点、坐标与权重。SCF 的对称约化数量可以不同：SCF 16 点、NSCF 输入及输出各 36 点属于合法场景。导入交付后也从计算目录重验，不要求共享状态清单。

若 NSCF 添加或改变 k 点，程序中断，不发布完成记录。按错误提示将 `stages."2.qe-nscf".calculation` 显式改为 `"bands"`，按既有失败恢复流程只重新运行 NSCF。仍调用 `pw.x`；`bands.x` 是后处理程序。程序不自动改模式或重试。

SCF、NSCF 的 `SYSTEM` 支持 `nr1/nr2/nr3`（密度 FFT）与 `nr1s/nr2s/nr3s`（平滑 FFT）；每套独立设置，但必须写全三个正整数。不填时由 QE 决定，阶段间不静默复制。程序对照两份实际 XML 的密度和平滑 FFT，任一不同即停止并报告来源和尺寸；PH 响应继续按绑定 SCF 来源、文件身份及尺寸核验。k 点网格和实空间 FFT 是两种网格。

0060 最初只修改源码和受控测试；后续 0010 已整合发布正式 CLI 并完成 SiP₂ 声子导入出图，未重算 PH，见本页外部导入说明及验收记录。配置与恢复示例见 [0060 操作说明](../docs/specs/0060-epw-nscf-grid-contract/quickstart.md)。

## 外部声子 save 单命令导入（0010）

已完成当前 SCF/NSCF 时，可直接使用独立历史 PH save：

```bash
epw_phonon --config "epwinput.toml" --import-phonon "external-phonon/save" -s
```

一个阶段作业自动完成内容识别、完整 q 星/扰动/响应势/FFT 校验、文本到 QE XML 转换、独立归集、q2r、色散及 DOS matdyn、元素 DOS/声子谱绘图和完成登记；来源只读，不调用 SCF/NSCF/PH，不在失败时回退重算。旧 ifc 不复用。SAVE 相对配置目录解析。

`--check` 不提交任务或改写科学数据；配置仍按 CLI 观察到的内容变化保存原文版本。无 `-s` 只准备；失败后相同命令加 `--resume`。重复完成请求不重提，只缺图时只补图；来源变化或产物损坏拒绝。显式导入可受控读取旧嵌套版本 2 TOML，不改写用户输入；外部来源和转换信息保存在 `epw-state/imports/`，进度进入第 4 步日志。历史赝势/截断能记录按用户例外不设门槛，报告明确未核验范围。

声子交付成功后使用 `epw_wannier --config "epwinput.toml" --stage wannier -s`，可补缺失的第三阶段电子参考，已有 SCF/NSCF/PH 复用。实际结果和路径见 [SiP₂ 说明](../examples/SiP2/README.md) 与 [0010 验收](../docs/specs/0010-epwflow-qe-data/acceptance.md)。

## 未启用 Slurm accounting 的本机环境

不必为了查询作业状态安装 accounting 数据库。本机已有 jobcomp/filetxt 时，CLI 在 sacct 明确报告未启用 accounting 后，使用控制器确认正在运行的分配，使用原生完成日志查询真实终态、节点数、用时与退出码。完成日志只读，临时查询副本自动清理；没有可用证据时仍拒绝重提，不绕过状态检查。

旧 Vanderbilt 二进制 .uspp 与默认文本读取分支不兼容时，可以保留同一赝势模型转换为 UPF；不能只改后缀。当前 MgB₂ 的同版本 QE 原生转换、数组往返核对及真实 SCF 结果见 [Ubuntu 手册](../examples/MgB2/ubuntu/readme.md) 和 [0066 验收](../docs/specs/0066-mgb2-legacy-pseudo-slurm/acceptance.md)。

## 电荷密度和波函数不核对

按用户明确要求，SCF／NSCF 的原始电荷密度和波函数不参与完整性核对，包括 charge-density 的 dat／hdf5／xml、wfc 的 dat／hdf5、prefix.wfcN 和旧 evc.dat。复用、完成检查、导入、电子后处理和复制交接均不对这些文件计算哈希，也不比较大小、非空、存在性或文件集合。历史记录中的对应条目在内存中忽略，不重写历史文件，不设严格模式开关。

实际复制仍传递这些数据；QE／EPW／projwfc 在计算时仍读取所需文件，实际复制或原生读取错误照常报告。结构、赝势、程序、原生输入、XML、日志、收敛及其他科学产物仍核验。当前目录、12 MPI、Slurm header／launcher 和覆盖构建约定保持，见 [0067](../docs/specs/0067-unchecked-density-wavefunctions/spec.md)。
