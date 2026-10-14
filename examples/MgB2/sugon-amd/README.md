# MgB₂ / sugon-amd 声子全流程

## 配置与范围

本机使用 QE 7.4.1、Slurm amd7513 分区、单节点 64 MPI 进程，配置保存在 docs/local/MgB2-sugon-amd.env。

本例使用一次 SCF，然后自动执行分 q PH、合并、q2r、matdyn、收集与绘图。默认 index；另有 coords 和 nosplit。初始 k=4×4×2、q=2×2×1、截断能 40/320 Ry、Gaussian 展宽 0.02 Ry，用于流程验收，不代表物理参数收敛；不包含 EPC/Tc。

本地环境文件格式：

```bash
QE_BIN=/实际路径/qe-7.4.1-amd-oneAPI2024/bin
PP_DIR=/实际路径/赝势目录
ENV_SCRIPT=/实际路径/oneapi_setvars.sh
DFTFLOW_BIN=/实际路径/dftflow/bin
PARTITION=实际Slurm分区
NTASKS=64
EXCLUDE=
```

模板适用于 Slurm + Intel MPI/oneAPI；其他调度环境需先适配。PP_DIR 包含 b_pbe_v1.4.uspp.F.UPF、mg_pbe_v1.4.uspp.F.UPF；只复制这两份赝势。KPATH 使用仓库附带路径，不调用 VASPKit。路径不含空格及 TOML 引号字符。DFTFlow 不自动读取 .my_scriptrc.py。

## 一次启动

在仓库根目录执行：

```bash
bash examples/MgB2/prepare.sh sugon-amd index
cd examples/MgB2/sugon-amd/run/index
cat input.toml
cat submit_phonon.sh
bash submit_phonon.sh
squeue -u "$USER"
```

准备只生成输入和脚本，不提交。**submit_phonon.sh 是主节点上的提交入口，用 bash 执行，不用 sbatch 执行它**；入口安排内部 Slurm 作业。coords/nosplit 将上述两处 index 替换为相应模式即可。各模式使用不同 run 子目录；已有目录不会被准备脚本覆盖。旧 run 根目录中的文件保留，不自动迁移。

## 两种分 q 模式

- index：ldisp=true，start_q=last_q=i，按 QE 实际 dyn0 顺序拆分。
- coords：ldisp=false，显式输入 2π/a₀ 单位的笛卡尔 q 坐标。

前置作业执行一次 SCF，将包含波函数的完整数据保存为 scf_snapshot，然后用 only_init=true/lqdir=true 正常完成 PH 初始化。q 点目录独立复制快照，只运行 PH。子作业依赖前置成功；合并作业依赖所有 q 点成功，无需手动再次提交。nosplit 为单个作业内顺序计算的对照模式。

## 监控、结果与恢复

在该模式 run 目录：

```bash
cat qe_phonon/chain_jobs.json
tail -n 30 qe_phonon/scf.out
tail -n 30 qe_phonon/ph_init.out
cat qe_phonon/q_index_map.json
ls qe_phonon/phband/ qe_phonon/phdos/
```

内部 Slurm 日志在对应工作目录或 q 点目录的 slurm-作业号.out/.err。PH 日志在 qe_phonon/NN/split_ph.out；最终有声子谱/DOS CSV、PNG 及 qe_phonon/phonon_combined.jpg。只有最终产物校验通过才写 chain_complete.json；提交成功不等于计算完成。nosplit 日志与旧单作业方式一致。

分 q 流程会同时占用前置作业、各未完成 q 点作业和后处理作业的提交额度；至少需要 `未完成 q 点数 + 2` 个可提交名额。sugon-amd 实测会触发 AssocMaxSubmitJobLimit，运行两种模式时应逐套启动；较大 q 网格需先取得相应账户额度。提交额度不足时明确报错并保留记录，不无限重试或占用节点等待。

失败时后处理不运行。等已提交任务终止后再次 bash submit_phonon.sh 可跳过完成项、复用 SCF；已有活动作业不会重复提交。调度查询失败时退出而不猜测状态。部分提交的作业号保留。异常终止遗留 .chain-submit.lock 时，确认无活动提交进程后人工移除锁再重试。

配置、结构或模式改变需要新目录；生成后的绝对路径不能随意移动。运行输出均被 Git 忽略，手册和脚本纳管。详细规格与验收见 [Issue 0006](../../../docs/specs/0006-qe-phonon-submit-chain/spec.md)。

## 直接调用 dftflow 与单独初始化

`prepare.sh` 仅为算例生成结构、配置和提交脚本。准备好 `input.toml`、`POSCAR` 和 `KPATH.in` 后，可直接调用实际命令：

```bash
# 按索引或坐标分 q：二选一，在各自独立目录执行
qe_phonon_split_index -c input.toml -i POSCAR -s
qe_phonon_split_coords -c input.toml -i POSCAR -s

# 不分 q：准备输入；本例通过准备脚本生成的 nosplit_job.sh 完成全流程
qe_phonon_nosplit -c input.toml
sbatch nosplit_job.sh
```

分 q 命令不加 `-s`/`-r` 时只准备，加 `-s` 提交完整依赖链。`-c` 指定配置，`-i` 指定结构。不分 q 的直接 `-r`/`-s` 全流程尚有已知缺口（漏跑/重复后处理与路径处理）；本例保留显式 nosplit_job.sh 路径，修复不纳入本次工具拆分。

只想取得真实 QE 的 q 点列表，已有完整 SCF 保存数据时执行：

```bash
qe_ph_nosplit -c input.toml --dyn0       # 只生成 ph_init.in 和 run_ph_init.sh
qe_ph_nosplit -c input.toml --dyn0 -s    # 提交仅初始化作业
# 已获得计算资源时才使用：
qe_ph_nosplit -c input.toml --dyn0 -r
```

注意单步命令是 **qe_ph_nosplit**，完整流程命令是 **qe_phonon_nosplit**。
默认 `[qe_phonon.scf].outdir` 为 `./tmp`，SCF 与 PH 初始化/分 q/合并共用该设置；显式设置其他目录会覆盖默认值。已有运行目录若显式使用 `./out`，继续按原配置读取，不自动搬迁或改写历史输入。

`[qe_phonon].work_dir` 相对配置文件目录解析；SCF 保存树位于该工作目录的 `[qe_phonon.scf].outdir`，也可指定绝对 outdir。prefix 依次取 `[qe_phonon].prefix`、SCF prefix、结构化学式。需要 XML、电荷密度及波函数；此命令不补跑 SCF。

初始化设置 `ldisp=true, only_init=true, lqdir=true`，清除 q 点/表示范围限制，等待 QE 正常退出后检查 `ph_init.out` 和 `prefix.dyn0`。成功生成 `ph_init_complete.json` 只代表初始化完成；不计算完整声子、不提交分 q 点及后处理。已有 dyn0 在实际重新初始化前归档为 `prefix.dyn0.before-init-<时间戳>`，失败不复用旧列表。

自动分 q 流程已包含初始化，无需先手动执行该命令。自写 dyn0 生成器已拆为独立工具，主程序和自动流程不再调用它。
