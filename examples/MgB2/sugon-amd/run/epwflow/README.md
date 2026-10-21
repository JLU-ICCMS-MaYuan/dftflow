# MgB₂ EPW 运行说明

## 进入目录并检查输入

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"
epw_phonon -c "epwinput.toml" --check --json
```

`source` 将仓库程序加入当前终端 PATH；`--check` 检查配置、输入文件和并行设置，不提交计算。以下命令统一读取本目录的 `epwinput.toml`。

## 按顺序计算

**每次 `-s` 最多提交一个原生任务。等待作业结束，再重复同一条命令，直至该阶段核验完成，才执行下一步。** 一个阶段可能包含多次原生计算；只执行一次提交命令不会自动跑完该阶段。

| 步骤 | 命令 | 作用 | 完成后的主要内容 |
| --- | --- | --- | --- |
| 1 | `epw_phonon -c "epwinput.toml" --stage scf -s` | 自洽计算 | `1.qe-scf/`：SCF 日志、电荷密度、QE 保存数据 |
| 2 | `epw_phonon -c "epwinput.toml" --stage nscf -s` | 完整均匀 k 网格非自洽计算 | `2.qe-nscf/`：本征值、波函数及 NSCF 日志 |
| 3 | `epw_phonon -c "epwinput.toml" --stage electronic -s` | QE 电子能带和总 DOS | `3.qe-electronic/`：能带、DOS 数据及后处理结果 |
| 4 | `epw_phonon -c "epwinput.toml" --stage phonon -s` | PH、q2r、matdyn 声子计算及后处理 | `4.qe-phonon-nosplit/`：动力学矩阵、响应势、声子色散／DOS、供 EPW 使用的 `save/` |
| 5 | `epw_wannier -c "epwinput.toml" --stage wannier -s` | Wannier 拟合及电子后处理 | `5.epw-wannier/`：Wannier 输入／日志／变换文件，电子能带和 DOS 图 |
| 6 | `epw_wannier -c "epwinput.toml" --stage coarse -s` | 粗网格电子声子耦合及 QE／EPW 常规对比 | `6.epw-coarse/`：粗网格矩阵元、比较数据与图 |
| 7 | `epw_superconductivity -c "epwinput.toml" --stage fine -s` | 细网格插值、α²F 和 λ | `7.epw-fine/`：细网格矩阵元、谱文件及交付清单 |
| 8 | `epw_superconductivity -c "epwinput.toml" --stage superconductivity -s` | 按当前配置求解 iso／aniso 并搜索 Tc | `8.epw-superconductivity/`：逐模式／温度／初值的输入、日志和 Δ／Z；搜索完成后才有有效 Tc 区间与最终结果包 |

查看提交命令给出的作业号及当前工作区状态：

```bash
sacct -j 作业号 --format=JobID,State,ExitCode,Elapsed
epw_phonon -c "epwinput.toml" --status --json
```

`sacct` 查看作业终态和退出码；`--status` 查询工作区状态，不提交作业。`COMPLETED` 后仍需重复原阶段命令核验科学输出。失败后查阅该任务日志；需要重试时，在原阶段命令中增加 `--resume`。活动或终态未知的任务不能重复提交。

去掉 `-s` 只准备当前可执行任务的输入和脚本；正式推进使用上述公开命令。

## 目录结构

```text
run/
├── POSCAR                         # 晶体结构
├── pp/                            # PBE 赝势
└── epwflow/
    ├── README.md                  # 本说明
    ├── epwinput.toml              # 统一输入
    ├── KPATH.in                   # 能带／声子路径
    ├── 1.qe-scf/
    ├── 2.qe-nscf/
    ├── 3.qe-electronic/
    │   ├── qe_eband/
    │   └── qe_eledos/
    ├── 4.qe-phonon-nosplit/save/
    ├── 5.epw-wannier/
    ├── 6.epw-coarse/
    ├── 7.epw-fine/
    ├── 8.epw-superconductivity/
    │   ├── iso/
    │   └── aniso/
    └── epw-state/                 # 提交、预算绑定和阶段状态
```

编号目录随计算生成。原生输入、提交脚本和日志位于实际计算目录；细网格、后处理及超导任务可能位于其下的尝试子目录。`out/` 等原生重读数据与清单需要保留，后续阶段和续算会使用。

## 关键参数提醒

- **路径**：`work_dir="."`、`layout="numbered"` 将阶段目录直接建在本目录；相对路径以 TOML 所在目录为基准，结构为 `../POSCAR`。赝势和外部程序目前使用绝对路径。
- **并行**：当前 NSCF 为 `4×4×2`，共 32 个 k 点；`launcher` 的 `-np 32`、`resources.mpi=32` 和 EPW 的 `-nk 32` 应保持一致。当前 EPW 路径每 pool 一个 MPI，pool 数不能超过可用 k 点数。
- **网格**：PH 粗 q 网格为 `2×2×1`，细 k／q 网格为 `8×8×4`／`4×4×2`。修改网格会影响相应阶段及下游复用。
- **Wannier**：`num_wann=5`、`selection_mode="energy_only"`、`guiding_centres=true`；保留初始投影，`B:pz` 在两个 B 原子上分别生成投影。
- **超导**：当前 `mode="both"`、`mu_star=0.05`、截断 `0.5 eV`、最多 1000 次迭代、相对收敛标准 `1e-8`、Broyden 混合 `0.7`。`gap_temperatures_k=[10,15,20]` 是 Tc 工作流的附加能隙温度，**不会将第 8 步改成只算 20／30／40 K 的单点模式**；当前搜索区间仍为 10～80 K。当前设置尚不能保证第 8 步正常完成。
- **资源**：每次提交受 `resources` 和共享 `[budget]` 账本限制；重算不会清零已有消费。`--check` 通过只代表输入检查通过，不代表八步计算已经完成。
