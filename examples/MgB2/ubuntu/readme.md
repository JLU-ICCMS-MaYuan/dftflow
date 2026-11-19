# Ubuntu 上的 MgB₂ EPW 自测试

本目录是独立工作区，配置为 `input.toml`，阶段、日志和结果生成在本目录的 `run/`，所有运行数据仍位于 ubuntu 内。Git 仅跟踪本输入和本文档；仓库现有 `/examples/` 忽略规则屏蔽其他新文件，不要强制添加计算数据。

## 输入与测试范围

- 按用户要求保留仓库 test_auto/MgB2/MgB2.vasp 结构和原 PBE 赝势模型，当前输入读取本目录 `pp/`；其余主要科学参数按 Mori Tutorial Wed.9 第 16～19 页对齐。因此本配置不是 PDF 的完整 LDA 复现。
- SCF 网格 8×8×8，NSCF 网格 6×6×6、16 条带；PH 为 index 模式、3×3×3；细网格 k 为 40×40×40、q 为 20×20×20。PDF 第 16 页正文写 SCF 12³，但展示的 scf.in 为 8³；与 chem 一样采用展示输入。
- 截断能为 40／320 Ry（320 Ry 沿用 PBE 设置）；SCF／NSCF 使用 mp 展宽 0.05 Ry、conv_thr=1e-9，声子 tr2_ph=1e-16。
- 5 个 Wannier 函数，B:pz 与三个键中心 s 初始投影；使用 `energy_only` 自动选窗，实际能窗须查看本次原生日志。
- 第 8 阶段保留 iso／aniso 两种方法，均使用 PDF 的 μ*=0.05、10～55 K 每隔 5 K 共 10 个温度；wscut=0.5 eV、nsiter=500、conv_thr_iaxis=1e-3，启用 Padé，不执行 Tc 搜索。PDF 的 temps=10 55 等距写法已展开为当前接口所需的完整列表；PDF 未显式设置的 npade、broyden_beta 取 EPW 原生默认值 90、0.7。
- Slurm header 申请本机 `local` 分区、单节点、12 任务；launcher 启动 12 MPI，OMP／MKL 线程各为 1。教程参数不代表 MgB₂ 的物理结果或网格已收敛。
- 输入已删除 `[budget]`，程序不再读写 `campaign/ledger.json` 或绑定固定项目名。现有 `campaign/` 只保留历史资料；完整运行前检查生成脚本和本机资源限制。

2026-11-19 已拉取最新代码并重新构建固定入口，三个科学入口的配置检查通过；没有提交新的科学计算，也没有改写已有 run。PDF 与 chem 冲突时采用 PDF 的明确数值，PBE 赝势和现有结构按用户决定保留。Wannier 采用 500 次局域化迭代、PDF 的初始投影与绝对冻结上界 8.8 eV；排除带 1～4 和其余三个能窗边界沿用 chem／PBE 设置，尚须根据新计算能谱核对。下文带日期的验收和旧能窗分析是历史记录，不能作为本次配置的科学验收。

## 本机环境

```bash
source "/home/mayuan/bin/dftflow-env.sh"
source "/home/mayuan/code/dftflow/scripts/env.sh"
cd "/home/mayuan/code/dftflow/examples/MgB2/ubuntu"
epw_phonon -c "input.toml" --check --json
```

编译及运行环境入口统一放在 `~/bin/`：`dftflow-env.sh` 内部加载 `intel-env.sh`，后者激活 Conda 构建环境和 Intel oneAPI。只需 source 前者。TOML 使用绝对路径，避免依赖 `~` 展开。仓库 `scripts/env.sh` 负责加载构建后的产品入口。

配置中的科学程序和环境脚本使用本机绝对路径；迁移机器时应修改。相对结构、赝势和 KPATH 路径均以输入文件所在目录为基准。

## 本机并行约定

本机 CPU 为 Intel Core i7-8700，1 个插槽、6 个物理核、每核 2 线程，共 12 个逻辑 CPU；Slurm local 节点登记 12 CPU。用户要求本机后续所有支持并行的科学计算统一使用 12 路 MPI。配置使用 `mpirun -np 12`，Slurm header 使用 `--ntasks=12`；EPW 当前单 image 路径使用 `-nk 12`，OMP／MKL 线程保持 1，避免线程数再相乘。串行程序不会因设置 launcher 就获得并行能力，不将生成脚本或检查命令宣称为 12 路科学计算。此约定只针对当前 Ubuntu 主机，不覆盖其他机器配置。

## 调度与启动配置

整个 `[resources]` 已删除。`[slurm].header` 原样提供 Slurm 文件头；`programs.launcher` 独立指定 MPI 启动参数，程序不自动生成资源指令。分配数量可大于实际启动数量。EPW 原生 pool 参数仍须满足其算法约束。

```toml
[programs]
launcher = ["mpirun", "-np", "12"]

[slurm]
header = """
#SBATCH --partition=local
#SBATCH --nodes=1
#SBATCH --ntasks=12
"""
```

重新编译成功后覆盖固定命令，不保留编译版本。构建入口为 `bash scripts/build-native.sh build --release --workspace --locked`，在仓库根目录运行。

## 按阶段运行

在上一阶段完成并核验后执行下一条。`-s` 提交 Slurm 作业；第 5～8 步作业结束后重复相同命令进行核验及后处理。不要一次连续提交下面所有命令。

```bash
epw_phonon -c "input.toml" --stage scf -s
epw_phonon -c "input.toml" --stage nscf -s
epw_phonon -c "input.toml" --stage electronic -s
epw_phonon -c "input.toml" --stage phonon -s
epw_wannier -c "input.toml" --stage wannier -s
epw_wannier -c "input.toml" --stage coarse -s
epw_superconductivity -c "input.toml" --stage fine -s
epw_superconductivity -c "input.toml" --stage superconductivity -s
```

查看 `run/epw-state/` 阶段日志、各阶段原生日志和 `run/results/`。重点检查 SCF／PH 正常结束、Wannier 收敛、QE／EPW 能带与声子对比、α²F／λ 和两个方法的能隙收敛。作业成功退出不能代替科学收敛检查。

## 2026-11-13 准备检查

已验证 TOML 语法、引用文件、科学程序执行权限、Intel 运行库加载，以及 `local` 分区可用；加载环境后 `pw.x -help` 显示 QE 7.5。

初次准备时环境找不到 `cargo`／`rustc` 和 EPW 入口，未运行配置检查或提交科学作业。此输入基于仓库版本 2 测试配置整理，尚待真实 CLI 和原生链验证，不能声称计算通过。当前工具链及构建验证以 [0065 验收](../../../docs/specs/0065-slurm-header-launcher/acceptance.md) 为准，再从上面的配置检查开始。

## 0065 初次统一配置后的检查

正式 EPW 三入口的 `--check --json` 均返回 `operation_status=ok`、`workflow_status=configured`。首次通过正式入口生成 `run/1.qe-scf/scf.sh` 时，核对 header、`mpirun -np 2` 与 `~/bin/dftflow-env.sh`，未提交作业。gap 输入显式设置 `tc_linear=false` 和 `gap_edge=0.01` eV，当时 EPW 原生 pool 为 2；这些是输入完整性与并行约束，不是科学收敛结论。

全部正式命令已在固定 `bin/<模块>/` 覆盖。359 项 EPW 回归与 140 项其他模块回归通过；15 项既有暂挂测试不计通过。详见本次验收记录。Git 仍只纳管本目录的 `input.toml` 和本文档。

2026-11-13 按用户要求将当前配置调整为 12 MPI／12 Slurm 任务／12 EPW pool。当时三个正式入口检查通过，但重生成被 accounting 未启用阻断。该阻塞现已在 0066 修复，当前脚本和真实完成的作业 4 均为 12 路；先前 2 路失败日志保存在 repair/failed-scf-evidence/。

## 旧 Mg 赝势修复与真实 SCF

2026-11-13 已完成一次 12 MPI 真实 SCF（Slurm 作业 4）。9 次迭代收敛，最终自由能为 −138.11356494 Ry，QE WALL 为 53.75 秒，Slurm 记录 60 秒、1 节点，退出码 0:0；原生日志包含 JOB DONE。公开入口再检查返回 stage_complete=true、workflow_complete=false。仅证明此 SCF 成功，不表示网格、Wannier、声子或超导结果收敛。

旧 mg_pbe_v1.4.uspp 是 Vanderbilt 7.3.6 的 Fortran 未格式化二进制文件。QE 7.5 的默认 read_ps_new 先尝试 UPF，旧 Vanderbilt 分支则按 .vdb／.van 后缀读取文本；原 .uspp 因而误入旧 NC 文本读取器。并非原势模型失效，也不是文件权限问题。

本地 repair/legacy_binary_uspp.f90 按同版本 QE upflib/read_uspp.f90 的 !unf 注释恢复二进制读取及三个合并记录，repair/convert_legacy.f90 调用同版本原生 write_upf／read_ps_new 转换并重读核对。保持 PBE、10 个价电子、821 径向点、3 个原子波函数和 7 个 beta；r、rab、vloc、rho_at、beta、dion、qqq、qfunc、qfcoef、chi 全部满足 1e-12×(1+数组最大绝对值) 的往返误差标准。未重新生成另一套赝势，原始 Mg／B 文件保持不变。

转换源代码、单份本地转换命令、输出和 conversion.log 位于忽略的 repair/；生成文件不纳入 Git，删除后须依据上述同版本源码重新转换。当前路径是 repair/pp/mg_pbe_v1.4.UPF 和 repair/pp/b_pbe_v1.4.uspp.F.UPF。原始文件校验记录为 repair/original-pseudos.sha256。

本机没有 Slurm accounting 数据库，但启用了 jobcomp/filetxt。新版 CLI 在 sacct 明确报告未启用 accounting 时，使用控制器核验正在运行的作业，或对完成日志的 0600 临时副本调用原生 sacct --completion --file，读取真实终态、NNodes、ElapsedRaw 和 ExitCode；副本查询后删除。不修改系统配置或原完成日志，不猜测缺失任务终态。

后续手动继续 NSCF：

    epw_phonon -c "input.toml" --stage nscf -s

不要再次对已完成 SCF 使用 --resume，除非确实需要重算。详细证据见 [0066 验收](../../../docs/specs/0066-mgb2-legacy-pseudo-slurm/acceptance.md)。

## 电荷密度与波函数不核对（0067）

原始电荷密度和波函数不做哈希、大小、非空、存在性或集合核对，旧记录对应条目也忽略；实际复制与原生读取保留，其他元数据和科学检查保留。本次目录保持现状，不删除 campaign 或 repair/pp。正式 CLI 已覆盖，现场 SCF 复用打开这两类文件的次数由 3934 降为 0，未提交新计算。完整原话和验收见 [0067](../../../docs/specs/0067-unchecked-density-wavefunctions/spec.md)。


## 历史记录：按 Mori 相对费米能设置 PBE 能窗

此前输入保留 5 个 Wannier 函数，排除原始带编号 1–4 的 Mg 2s/2p 半芯态。以下为旧输入能窗（本次冻结上界已改为 PDF 的 8.8 eV），单位为相对 QE 能量零点的绝对 eV：

- dis_win_min = −3.40915530，dis_win_max = 26.41812126。
- dis_froz_min = −3.40915530，dis_froz_max = 10.51412358。

外窗覆盖当前 NSCF 排除半芯带后的能谱并在两端各留 0.02 eV。Mori 第 19 页给出冻结上界 8.8 eV，第 20 页示例费米能为 7.469822726 eV；本次以其相对位置约 E_F＋1.330177274 eV 作为 PBE 初始窗口，平移到当前 NSCF E_F=9.183946302 eV。此平移是本次选窗起点，不代表不同赝势的能带严格刚性平移或拟合已经通过。

对当前 32 个均匀 k 点，保留 12 条候选带，外窗每点 12 态，冻结窗每点 4～5 态，满足 frozen≤5≤outer，并包含 E_F±0.4 eV。原 SCF／NSCF 设置不变。本次没有执行拟合；仍需检查你后续运行的原生收敛与能带对比。


组合图放在各阶段 plot/，并发布到 results 对应阶段的 plot/。第 3、5 步各两张电子图：electronic.png 使用 input.toml 中 [plot] 的两个独立上下限（默认 −3／＋3 eV，相对费米能），electronic-from-lowest-to-highest.png 使用 QE 路径全部能带极值。第 4、6 步各一张 phonon.png，声子谱与 DOS 共用 0 到最高路径频率的纵轴。逐带范围与衰减诊断保留。

## 独立采样与已有频率复用（0043）

QE 与 EPW 沿同一物理高对称路径，各自保留原生采样；不要求内部点一一对应。新生成 QE 声子比较采用第 4 步 `points_per_edge`，EPW 采用第 6 步。当前现场保存的 QE `matdyn.in` 与 `reference.freq` 为 215 点、9 模式，自身一致，可以复用；当前配置展开的 935 点不强加给这份旧结果。

旧频率不为差值补点或重采样；仅已有共同坐标参与差值，空配对不阻止出图。改变比较密度不使已完成粗 EPC 失效，只有相应比较后处理按新请求更新。电子内嵌曲线由 `bands_num_points` 控制，不要求与第 3／6 步密度相同。比较图不绘制紫色误差标记，Wannier 窗口线保持。

本次未执行新的科学作业。若现场尚缺普通 EPW 出图重读结果，继续使用公开 `epw_wannier -c "input.toml" --stage coarse -s` 补齐，再运行 `epw_superconductivity -c "input.toml" --stage fine -s`；这不表示需要重算 SCF、PH、Wannier 或粗 EPC。

## 程序路径与参数

活动 `input.toml` 使用 `[programs]` 下的 `pw_x`／`pw_args` 等平铺字段，后处理路径逐个显式声明。各 args 独立，空列表不追加参数；需要 pencil 时在对应程序明确设置 `-pd`、`.true.`。本机 launcher 仍为 12 路 MPI，EPW pool 为 12，全部 EPW 阶段仅允许单 image。已有 run 内冻结输入和脚本没有自动改写，不能将活动配置检查视为旧工作区续算或科学验收通过。

## 当前输入格式与公共参数（0061）

当前 `input.toml` 使用八个主阶段及独立用途子段。`[baseparams]` 集中填写 `ATOMIC_SPECIES` 二维数组、`prefix`、`ecutwfc/ecutrho` 和 QE DOS／PDOS 的 `Emin/Emax/DeltaE`；仅适用调用回填，阶段显式值优先。截断能单位 Ry，后三项单位 eV；它们不是 `[plot]` 显示范围或 Wannier 能窗。

`nk/nq/nkf/nqf` 各为三个正整数，只放所属阶段或用途子段；不得与同一网格的 `nk1` 等方向字段混写，不在公共表继承。SCF 的 `K_POINTS` 原生卡片、NSCF 的 `K_POINTS=[6,6,6]` 及 postw90 的 `dos_kmesh` 保持各自接口。当前 PH 为 `nq=[3,3,3]`，EPW 粗 k/q 为 `[6,6,6]`／`[3,3,3]`，细 k/q 为 `[40,40,40]`／`[20,20,20]`。

此前格式迁移只调整配置表达并删除不再消费的双初值／温度搜索 workflow 设置；当时保留 num_wann=5、排除带 1～4、四个绝对能窗、12 MPI、QE／EPW 独立路径密度及 20／30／40 K 原生多温度列表，未触发科学计算或改写历史配置。本次有效参数见文首。

本次检查发现原 `repair/pp/` 已缺失，已按 0066 恢复：Mg 从原二进制势以 QE 原生 readvan／write_upf 转换为 UPF，再用原生 read_ps_new 读取核对，主要物理数组的最大归一化差为 1.54×10⁻¹⁵，小于 10⁻¹²；B 使用原文件的相对链接。`repair/pp/` 是实际赝势输入，运行前需保留，不跟踪转换产物。未执行 SCF 或其他科学计算。
