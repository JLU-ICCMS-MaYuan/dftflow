# MgB₂ EPW 运行说明

## 进入目录并检查输入

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"
epw_phonon -c "epwinput.toml" --check --json
```

`source` 将仓库程序加入当前终端 PATH；`--check` 检查配置、输入文件和并行设置，不提交计算。以下命令统一读取本目录的 `epwinput.toml`。

## 按顺序计算

**每次 `-s` 最多提交一个原生任务。等待作业结束，再重复同一条命令，看到 `workflow_complete: true` 后才执行下一步。** 一个阶段可能包含多次原生计算；只执行一次提交命令不会自动跑完该阶段。

### 第 1 步：自洽计算

```bash
epw_phonon -c "epwinput.toml" --stage scf -s
```

运行完成后，`1.qe-scf/` 保存 SCF 日志、电荷密度和 QE 保存数据。

### 第 2 步：完整均匀 k 网格非自洽计算

```bash
epw_phonon -c "epwinput.toml" --stage nscf -s
```

运行完成后，`2.qe-nscf/` 保存本征值、波函数和 NSCF 日志。

### 第 3 步：QE 电子能带和总 DOS

```bash
epw_phonon -c "epwinput.toml" --stage electronic -s
```

运行完成后，`3.qe-electronic/` 保存能带、DOS 数据及后处理结果。

### 第 4 步：声子计算及后处理

```bash
epw_phonon -c "epwinput.toml" --stage phonon -s
```

执行 PH、q2r 和 matdyn。运行完成后，`4.qe-phonon-nosplit/` 保存动力学矩阵、响应势、声子色散和 DOS；其中 `save/` 保存供 EPW 使用的数据。

### 第 5 步：Wannier 拟合及电子后处理

```bash
epw_wannier -c "epwinput.toml" --stage wannier -s
```

运行完成后，`5.epw-wannier/` 保存 Wannier 输入、日志、变换文件，以及电子能带和 DOS 图。

### 第 6 步：粗网格电子声子耦合及常规对比

```bash
epw_wannier -c "epwinput.toml" --stage coarse -s
```

运行完成后，`6.epw-coarse/` 保存粗网格矩阵元、QE／EPW 比较数据与图。

### 第 7 步：细网格插值、α²F 和 λ

```bash
epw_superconductivity -c "epwinput.toml" --stage fine -s
```

运行完成后，`7.epw-fine/` 保存细网格矩阵元、谱文件及交付清单。

### 第 8 步：20／30／40 K 的 iso／aniso 能隙

```bash
epw_superconductivity -c "epwinput.toml" --stage superconductivity -s
```

按 `task="gap"`、`mode="both"` 和温度列表依次执行六个单点。每个温度使用一个初始能隙，不搜索 Tc。`8.epw-superconductivity/` 的尝试子目录保存该点的 `epw.in`、`submit.sh`、`epw.out`、原生 Δ／Z 文件和 `attempt.json`。六点正常执行结束后，`results/` 保存 `points.csv` 逐点执行／收敛／数据状态、`gap_summary.csv` 能隙统计、`gap_states.csv` 状态明细及三格式图像；未收敛、NaN 或缺失能隙的数据会注明原因，不填零。

六点完成后，检查结果包及其来源：

```bash
epw_superconductivity --results-check --results "8.epw-superconductivity/results" --json
```

返回 `workflow_complete: true`、`source_verification: "verified"`，表示逐点执行和结果包核验完成；`gap_run.data_complete` 另列能隙数据是否全部有效。

### 查看作业与工作区状态

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
    │   ├── iso-020.00K-attempt-0001/  # 每个模式和温度独立尝试
    │   ├── aniso-020.00K-attempt-0004/
    │   ├── manifest.json              # 六点计划、尝试及完成状态
    │   └── results/                   # 表格、状态和 figures/
    └── epw-state/                 # 提交、预算绑定和阶段状态
```

编号目录随计算生成。原生输入、提交脚本和日志位于实际计算目录；细网格、后处理及超导任务可能位于其下的尝试子目录。`out/` 等原生重读数据与清单需要保留，后续阶段和续算会使用。

## 关键参数提醒

- **路径**：`work_dir="."`、`layout="numbered"` 将阶段目录直接建在本目录；相对路径以 TOML 所在目录为基准，结构为 `../POSCAR`。赝势和外部程序目前使用绝对路径。
- **并行**：当前 NSCF 为 `4×4×2`，共 32 个 k 点；`launcher` 的 `-np 32`、`resources.mpi=32` 和 EPW 的 `-nk 32` 应保持一致。当前 EPW 路径每 pool 一个 MPI，pool 数不能超过可用 k 点数。
- **网格**：PH 粗 q 网格为 `2×2×1`，细 k／q 网格为 `8×8×4`／`4×4×2`。修改网格会影响相应阶段及下游复用。
- **Wannier**：`num_wann=5`、`selection_mode="energy_only"`、`guiding_centres=true`；保留初始投影，`B:pz` 在两个 B 原子上分别生成投影。
- **超导**：`task="gap"` 按 `gap_temperatures_k=[20,30,40]` 算能隙；`mode="both"` 同时计算 iso／aniso，也可选 `iso` 或 `aniso`。`initial_gap_mev=10` 为单初值，`mu_star=0.05`、截断 `0.5 eV`、最多 1000 次迭代、相对收敛标准 `1e-8`。`mixing_beta=-0.3` 使用原生线性混合，正值使用 Broyden，系数绝对值须在 `(0,1]`。温度须为正数、精确到 0.01 K、不重复且不超过 999.99 K；截断须包含首个 Matsubara 频率。省略 `task` 仍走旧 Tc 搜索，需要原 Tc 参数表。
- **资源**：每次提交受 `resources` 和共享 `[budget]` 账本限制；重算不会清零已有消费。`--check` 通过只代表输入检查通过，不代表八步计算已经完成。
