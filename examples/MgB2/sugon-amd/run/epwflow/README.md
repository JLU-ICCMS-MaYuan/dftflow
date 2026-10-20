# MgB₂ 在 sugon-amd 上的 EPW 手动测试

本目录用于逐阶段手动测试 `epw_phonon`、`epw_wannier`、`epw_superconductivity`。当前先测试 EPW；其他工作流的说明见相邻目录。小网格参数用于检查流程，不能作为已经收敛的 MgB₂ Tc。

## 0. 先看目录逻辑：输入、运行记录、科学计算分三层

**日常只需记住：在 `epwflow/` 核对输入，在 `work/.epwflow/` 查作业，在 `work/calculations/` 看实际计算。三个程序接力使用同一套数据，不会各建一套互不相干的工作目录。**

### 当前目录实际长什么样

下图按 **2026-10-20** 的实际目录核对，省略了大量波函数和逐 q 文件。自动编号只是程序生成的唯一标识，没有物理含义，无需记忆或手工命名。

```text
run/
├── POSCAR                                  # 共用结构
├── pp/                                     # 共用赝势
└── epwflow/                                # 本例的输入与操作入口
    ├── README.md                           # 本手册
    ├── epwinput.toml                        # 用户填写的配置
    ├── KPATH.in                             # 高对称路径
    ├── .epwflow/                            # 外层旧准备记录，与当前运行区分
    │   ├── generated/                      # 旧 24 进程脚本，指向外层 epwflow/
    │   └── preparation-history/
    │       └── 0a391efde412aa34-mpi24/      # 旧 work、配置和准备锁的归档
    └── work/                               # 当前这一次运行的完整工作目录
        ├── input-init.toml                  # 启动时固定的配置
        ├── POSCAR-init、KPATH-init           # 启动时固定的结构、路径
        ├── run-binding.json                 # 输入、程序及路径的身份记录
        ├── .epwflow/                        # 当前运行的调度与恢复记录
        │   ├── workflow.json                # 最新阶段、作业号、实际计算目录
        │   ├── generated/                   # 当前 64 进程的阶段脚本与依赖脚本
        │   ├── slurm/                       # 实际提交脚本、提交回执及 Slurm 日志
        │   └── history/                     # 前一次阶段执行记录
        ├── qe-data/manifest.json            # 第一阶段清单的副本，方便工作流查询
        └── calculations/
            └── 18e028b4840e50b5-2bbfe5-a/  # 当前实际科学目录，后续简称 CALC
                ├── qe-phonon/              # 复用 qeflow 的 SCF→PH→q2r→matdyn
                │   ├── scf.in、scf.out      # 自洽电子计算输入、输出
                │   ├── ph.in、ph.out        # 声子计算输入、输出
                │   ├── out/                # QE 保存树、波函数及响应数据
                │   └── phband/、phdos/      # QE 声子色散与 DOS 图
                ├── scf/snapshot/            # PH 前冻结的 SCF 保存树，供下游复用
                ├── nscf/                    # 完整均匀 k 网格的非自洽计算与 out/
                ├── qe-data/                 # 交给 EPW 的数据清单、q 点映射和 save/
                ├── quality-sources/         # 独立 QE 参考，用来检查插值误差
                │   ├── electron/            # 独立电子参考
                │   ├── ph-*/、dynmat-*/     # 独立声子参考与频率提取
                │   └── reference.json       # 参考来源清单
                ├── coarse/                  # 原生 EPW：Wannier 与粗网格计算现场
                │   ├── epw_coarse.in        # EPW 输入
                │   ├── epw.out、epw.err      # EPW 原始输出、错误
                │   ├── MgB2.win、MgB2.wout  # Wannier 输入、输出
                │   └── out/、save/          # 本阶段使用的电子态和响应数据
                └── native-attempts/         # 原生子步骤的执行、恢复与归档记录
```

`manifest.json` 是“这批数据包含哪些文件、来自哪里、校验是否通过”的清单，不是科学结果本身。当前 `work/qe-data/manifest.json` 与 `CALC/qe-data/manifest.json` 内容一致；前者是清单副本，并没有再复制一套完整计算。真正的数据位置由清单及 `workflow.json` 中的 `calculation_dir` 指定。

### 三个程序怎样接力

```text
epwinput.toml + ../POSCAR + ../pp/ + KPATH.in
    ↓ 固定输入到 work/，生成调度脚本
epw_phonon：QE 声子链 + 完整均匀 NSCF → qe-data 清单
    ↓ 复用同一个 CALC
epw_wannier：独立参考 + Wannier／粗网格 → 质量核验 + 比较图
    ↓ 前一阶段完成且质量通过后
epw_superconductivity：细网格 EPC → 超导求解 → 最终结果包
```

| 程序 | `work/.epwflow/generated/` 中的脚本 | 主要科学目录与完成依据 | | --- | --- | --- | | `epw_phonon` | `qe-data.sh` | `CALC/qe-phonon/`、`scf/`、`nscf/`、`qe-data/manifest.json` | | `epw_wannier` | `interpolation.sh` | `CALC/quality-sources/`、`coarse/`；成功后发布 `wannier/manifest.json` 和 `results/` 比较包 |
| `epw_superconductivity` | `all.sh` | 后续生成 `CALC/fine/`、`interpolation/`、`superconductivity/`；完整交付到 `work/results/` |

`submit-all.sh` 才是一次提交三个阶段依赖链的入口；`all.sh` 在这里是第三阶段的内部脚本名。脚本名、程序名和目录名没有完全统一，是阅读困难的一个实际原因。

**不要把上表的后续产物当作已经存在。** 本次核对时，第一阶段历史记录为成功，第二阶段最新记录为 `failed`，错误为 EPW 退出状态 1；实际科学目录已有 `coarse/`，但尚无 `wannier/manifest.json`、比较包 `CALC/results/`、`fine/` 或最终 `work/results/`。这表示第二阶段留下了执行现场，不能据目录存在认定阶段完成。后续实时状态按第 5、6 节查询。

### 为什么看起来重复、零散

| 容易困惑的地方 | 实际原因与读法 |
| --- | --- |
| 外层、`work/` 里都有 `.epwflow/` | 外层保留旧准备文件；当前以 `work/run-binding.json` 和 `work/.epwflow/workflow.json` 为准。两套 `chain.json` 的 `run_id` 不同，外层旧脚本不能作为本次提交入口 |
| 第二阶段作业号变了，科学目录却还是旧编号 | 第一阶段作业为 `1303823`，第二阶段为 `1304067`，后者仍指向上图同一 `CALC`。作业号标识 Slurm 分配，目录编号标识被复用的科学现场，两者不必相同 |
| `scf/` 里不是完整的 SCF 输入输出 | 实际 SCF 由共享声子链执行，日志在 `qe-phonon/scf.out`；`scf/` 保存供下游复用的冻结快照 |
| 多处都有 `out/`、`save/`，甚至更深的 `.epwflow/` | 不同原生阶段各有保存数据、暂存副本和启动日志；区分它们时要看完整父路径。例如 `nscf/out/` 与 `coarse/out/` 分属不同阶段 |
| 配置写了 `scratch/`，目录却不存在 | 顶层 `outdir` 目前只作为基址记录，实际大文件仍写在上述阶段目录；详见第 2 节 |
| 比较图和最终图不在同一处 | 第二阶段比较包在 `CALC/results/`；第三阶段完整交付在 `work/results/`。`qe-phonon/phband/` 等是更早的 QE 单独图 |

### 平时找什么，就看哪里

- **核对下一次新运行的参数**：看 `epwinput.toml`；当前已绑定运行使用 `work/input-init.toml`，不能改快照绕过一致性检查。
- **看提交到了哪一步**：看 `work/.epwflow/workflow.json` 的 `current_stage`、`state`、`error` 和 `calculation_dir`。
- **看排队脚本与作业错误**：看 `work/.epwflow/slurm/`。例如第二阶段调度错误在 `1304067.err`，原生报错在 `CALC/coarse/epw.out`、`epw.err`。
- **看科学结果与图**：先从运行记录读取 `CALC`（第 5 节有完整命令），再按阶段查上述位置。

这种布局同时照顾固定输入、作业恢复和原生程序各自的文件习惯；历史准备文件及不统一的命名又增加了阅读负担。本节按现状解释，不通过移动或重命名文件整理外观，因为运行绑定和清单中已有绝对路径及哈希。

## 1. 确认目录、程序和输入

在登录节点执行。以下变量只在当前终端有效，新开终端需重新设置：

```bash
REPO="/data/home/mayuan/code/dftflow"
RUN_DIR="$REPO/examples/MgB2/sugon-amd/run"
EPW_DIR="$RUN_DIR/epwflow"
EPW_BIN="$REPO/bin/epwflow"
CFG="$EPW_DIR/epwinput.toml"
W="$EPW_DIR/work"
LEDGER="$REPO/docs/local/epwflow-campaign.json"
cd "$EPW_DIR"

ls -l "$CFG" "$RUN_DIR/POSCAR" "$RUN_DIR/pp"
"$EPW_BIN/epw_phonon" --help
"$EPW_BIN/epw_wannier" --help
"$EPW_BIN/epw_superconductivity" --help
```

这些变量只是给路径起一个方便引用的名字。例如，后续写 `"$CFG"`，Shell 会将它替换为配置文件的完整路径；赋值本身不修改配置文件，也不启动计算。

| 变量或命令 | 含义 |
| --- | --- |
| `REPO` | DFTFlow 仓库根目录 |
| `RUN_DIR` | 本例的 `run/` 目录，存放共用的 POSCAR 和赝势 |
| `EPW_DIR` | 本例的 `epwflow/` 目录，存放 README、配置和本次 `work/` |
| `EPW_BIN` | 存放三个受测 EPW 工作流程序的目录；后续用完整路径调用，避免误用其他版本 |
| `CFG` | 本例配置文件 `epwinput.toml` 的完整路径；`-c "$CFG"` 指定读取它 |
| `W` | 本次运行的 `epwflow/work/` 目录；这里的赋值供命令引用，实际工作目录仍由配置中的 `work_dir` 决定 |
| `LEDGER` | 共享预算账本路径，供预算查询命令使用 |
| `cd "$EPW_DIR"` | 将当前终端的工作目录切换到本例 `epwflow/` |
| `ls -l ...` | 查看配置、结构和赝势路径是否存在及其文件信息 |
| 三条 `--help` | 查看三个程序的用法，确认入口可以启动 |

结构文件使用 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR`，赝势目录使用 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/pp`。两者均位于本目录的上一级 `run/`，无需复制到 `epwflow/`。

`EPW_BIN` 指向仓库 `bin/epwflow/` 中的运行库兼容安装副本；其他模块分别位于 `bin/qeflow/`、`bin/vaspflow/` 等目录。安装方法见[根 README](../../../../../README.md#安装更新与旧节点兼容)。不直接换成 PATH 中可能较旧的程序。帮助输出只能确认入口可启动，不能证明 QE／EPW 在计算节点已通过运行检查。

**已有运行的路径绑定**：历史生成脚本和工作流记录可能仍引用测试目录中的旧程序，本次保留这些文件及其哈希，不手改脚本或身份清单。新安装副本可用于只读状态和数据检查；已有生成脚本仍按原程序身份使用，不能从新路径对其重复生成，否则会报“已生成脚本身份不同”。新运行统一从 `bin/epwflow/` 生成；已有失败运行先排除科学错误，再按第 7 节显式恢复，不能通过换安装目录消除原计算失败。

本例的配置文件名是 **`epwinput.toml`**，因此所有新运行命令显式传 `-c`；程序默认寻找的名称仍是 `inputepw.toml`。

## 2. 核对配置中的相对路径

相对路径以配置文件所在目录解析。将配置移入本目录后，须核对 `epwinput.toml` 的以下字段；只替换对应字段，保留其余物理参数和程序配置，不重复添加 `[pseudos]` 表：

```toml
work_dir = "./work"
outdir = "./scratch"
structure = "../POSCAR"
kpath = "./KPATH.in"

[pseudos]
Mg = "../pp/mg_pbe_v1.4.uspp.F.UPF"
B = "../pp/b_pbe_v1.4.uspp.F.UPF"
```

这里将运行工作目录设为 `epwflow/work/`，将配置中的 `outdir` 基址设为 `epwflow/scratch/`。配置和 README 留在本目录，结构和赝势复用上一级 `run/` 中的文件。工作目录必须由程序初始化；**不要预先 `mkdir work`**，也不要将包含 README 和输入的本目录直接作为新 `work_dir`。原来的 `work_dir="./epwflow"` 会在本目录下再建立一层同名目录，与上述 `W` 不一致。

生成前检查的是实际配置文件，不能仅看本 README 中的示例；终端变量 `W` 只供后续命令引用，也不会覆盖配置中的 `work_dir`：

```bash
rg -n '^(work_dir|outdir)\s*=' "$CFG"
```

本例应输出 `work_dir = "./work"` 和 `outdir = "./scratch"`。即使已经 `cd` 到 `epwflow/`，`work_dir="./epwflow"` 仍会相对配置位置解析成 `epwflow/epwflow/`；改为 `./` 则会指向已有的输入目录，当前初始化逻辑也不接受没有完整运行绑定的已有目录。因此本例统一使用 `./work`，所有生成、状态和恢复命令的 `W` 都指向它。已有旧目录不因修改配置而自动迁移，其快照和受控脚本也不能手改后挪作新运行。

### work/ 和 scratch/ 分别放什么

`work/` 是这次运行的完整工作目录：保存固定输入、提交和恢复记录、实际科学计算文件以及结果。它不只是存放日志的小目录；当前实现也将波函数、响应势等大文件保存在其下。

| 位置（相对于 `epwflow/`） | 内容与作用 |
| --- | --- |
| `work/input-init.toml`、`POSCAR-init`、`KPATH-init`、`run-binding.json` | 固定配置、结构和路径快照，以及输入／程序身份绑定；续算时用于确认来源一致 |
| `work/.epwflow/` | 生成脚本、提交尝试、Slurm 日志和 `workflow.json`；记录作业、阶段状态及实际 `calculation_dir` |
| `work/calculations/<尝试号>/` | 实际科学目录，含输入、输出日志、保存树、清单、独立参考和诊断结果；恢复可继续复用原科学目录 |
| `work/qe-data/manifest.json` | 第一阶段清单的副本；实际数据仍在 `calculations/<尝试号>/` 下 |
| `work/results/` | 后续完整流程收集的最终结果包和图表；第二阶段的比较结果另在实际科学目录的 `results/` 下，不保证此刻已生成 |
| `scratch/` | 配置中声明的外部中间数据基址，对应顶层 `outdir`；当前三程序主路径尚未将实际中间数据写入此基址，因此它可能不存在或为空 |

**当前实际行为**：`outdir="./scratch"` 会被解析并记录为 `outdir_base`，但不能据此认为波函数都在 `scratch/`。当前共享 nosplit、完整 NSCF 和原生 EPW 分别使用实际科学目录下的 `qe-phonon/out/`、`nscf/out/`、`coarse/out/`、`fine/` 下各展宽子目录的 `out/` 等位置。例如第一阶段可能有：

```text
epwflow/work/calculations/<尝试号>/
├── qe-phonon/out/MgB2.save/  # 共享 QE 声子链使用的保存数据
├── scf/snapshot/            # PH 前冻结的独立 SCF 保存树
├── nscf/out/MgB2.save/      # 完整均匀 NSCF 的电子态与波函数
└── qe-data/manifest.json    # 第一阶段交付清单
```

上述内容随阶段推进生成，不要求手工建立。`scratch/` 为空不代表计算没有运行；应查看 `workflow.json` 中的实际科学目录、原生日志和阶段清单。更改顶层 `outdir` 当前也不会把这些阶段内的 `out/` 自动搬到其他磁盘。实现依据见 [共享声子链](../../../../../epwflow/src/qe_data/qeflow.rs)、[NSCF 执行](../../../../../epwflow/src/qe_data/nscf.rs)和[工作流执行](../../../../../epwflow/src/foundation/workflow.rs)。

运行和续算期间保留整个 `work/`；其中的 `.save`、波函数、响应势、SCF 快照和矩阵元不只是可随手删除的临时文件，下游计算及身份校验仍可能依赖它们。是否清理应在任务结束并完成结果归档后逐项核对，不能仅按目录名判断。当前说明仅解释目录用途，不执行清理或改变数据存放方式。

若 `pp/` 中实际文件名不同，按实际文件名填写元素映射；确认使用本例的 PBE 赝势，不能静默更换赝势而保留旧带号和科学结论。首次使用可从仓库已有路径文件复制 KPATH，仅在目标不存在时执行：

```bash
if [ ! -e "$EPW_DIR/KPATH.in" ]; then
    cp "$REPO/examples/MgB2/KPATH.in" "$EPW_DIR/KPATH.in"
fi
ls -l "$RUN_DIR/POSCAR" "$EPW_DIR/KPATH.in" \
    "$RUN_DIR/pp/mg_pbe_v1.4.uspp.F.UPF" \
    "$RUN_DIR/pp/b_pbe_v1.4.uspp.F.UPF"
```

这些配置说明针对新的 `work/` 运行。如果已有 `run-binding.json` 和固定快照，不修改快照、来源、受控脚本或已绑定的工作目录；恢复方法见第 7 节。本手册不迁移或清理旧记录。

## 3. 第一步检查：配置与共享预算

完成上述输入核对后，先执行以下命令，不提交作业：

```bash
"$EPW_BIN/epw_phonon" --check -c "$CFG" --json
"$EPW_BIN/epw_phonon" --budget status --ledger "$LEDGER" --json
```

第一条只做配置静态检查，不启动 QE／EPW，不提交 Slurm，也不生成固定快照或科学输入。通过时退出码为 0，典型输出如下：

```json
{
  "schema_version": 1,
  "operation_status": "ok",
  "workflow_status": "configured",
  "diagnostics": [],
  "environment": {
    "runtime_version": "unverified"
  }
}
```

### 返回字段是什么意思

| 字段 | 本次配置检查中的含义 |
| --- | --- |
| `schema_version: 1` | JSON 输出结构的版本号，不是 QE／EPW 的程序版本 |
| `operation_status: "ok"` | 本次检查命令正常完成，并通过了该命令负责的配置检查；不代表科学计算已经完成 |
| `workflow_status: "configured"` | 配置通过静态检查后给出的状态名称，不是另一项独立检查；不代表已生成输入、固定快照、提交作业或完成计算 |
| `diagnostics: []` | 本次返回没有诊断条目；当前成功输出不逐项列出检查明细，空列表不能证明所有运行和科学条件都已验证 |
| `environment.runtime_version: "unverified"` | 尚未实际验证科学程序的运行环境与运行版本 |

两个状态回答不同的问题：`operation_status` 表示“这条操作是否正常完成”，`workflow_status` 表示“它报告的配置或流程处于什么状态”。这里的 `configured` 来自成功读取、验证配置并解析路径后的固定返回值，并不是一次试算的结果。

### 配置检查具体检查什么

| 检查项 | 具体范围与本例 |
| --- | --- |
| 配置格式 | 配置文件可读，TOML 能解析，字段类型、必填参数和 `schema_version` 满足要求 |
| 参数基本约束 | 网格和相关数值范围合法；NSCF k 网格与粗 q 网格整除相容；细 k／q 网格相容；带数、排除带和 Wannier 数满足基本数量约束；已填写能窗的上下界及包含关系符合要求；收敛参数、质量阈值、温度范围等满足代码规定的约束 |
| 输入文件路径 | POSCAR、赝势，以及已指定或自动找到的 KPATH、已指定的参考清单等，是存在且可读的普通文件；相对路径按配置目录解析 |
| 程序和环境文件路径 | `pw.x`、`ph.x`、`epw.x`、已配置的可选程序及 MPI 启动程序路径存在、可读并有执行权限；已指定的环境脚本存在且可读 |
| 目录关系 | `work_dir` 与顶层 `outdir` 基址不同，预算账本不在工作目录内，结构／赝势来源不与运行工作树冲突等 |

例如，`structure="../POSCAR"` 会检查上一级 `run/POSCAR` 文件能否读取；它此时不会解析该结构的原子位置，也不会判断该结构是否适合作为最终 MgB₂ 科学输入。

### 哪些内容尚未检查

- 不解析 POSCAR 的具体结构或 UPF 赝势内容，不确认元素匹配、赝势科学适用性或物理参数收敛。
- 不实际运行 QE／EPW 或 MPI，不验证计算节点的动态库、MPI 通信及原生程序兼容性；例如 `epw.x` 文件有执行权限，实际启动时仍可能缺少动态库。
- 不向 Slurm 试提交，不确认排队和账户限制，也不核验剩余预算或磁盘可用空间；顶层目录能解析不代表容量和写入条件已通过。
- 不验证 SCF／Wannier 收敛、实际选窗中的逐 k 态数、插值误差、α²F、λ、能隙或最终 Tc；这些依赖后续实际计算和科学检查。

因此，看到 `ok` 和 `configured`，可以理解为“配置及所引用路径通过了当前静态检查”，不能理解为“整条计算链已经跑通”。以上说明针对无 `-w` 的 `--check -c`；后文带 `-w` 的检查针对阶段数据，其检查范围和 `workflow_status` 含义要按对应命令解读。

### 单独查看共享预算

第二条只读查看累计节点秒、未结算预留和存储基线。继续共用 `epic-0007` 的原账本与 8 节点小时／100 GiB 上限，不能另建账本重置消费。预算是否足以接受本次作业由提交时继续核验。

本例当前配置为 `amd7513`、1 节点、64 MPI、每阶段最长 7200 秒；SCF／NSCF 为 4×4×2，粗 q 为 2×2×1。电子插值最大／RMS 门为 0.05／0.02 eV，声子门为 50／20 cm⁻¹；以实际配置为准，不为跑通而放宽。

### 使用单节点全部 64 核

Slurm 查询确认 `amd7513` 每节点为 2 插槽×32 核×1 线程，共 64 个物理核。本例统一配置为 1 节点、64 个 MPI 进程，每进程单线程；生成脚本的 `--nodes=1`、`--ntasks=64` 对应整节点 64 核，`OMP_NUM_THREADS=1` 和 `MKL_NUM_THREADS=1` 避免每个 MPI 进程再启动额外计算线程。

| 配置位置 | 本例取值 | 作用 |
| --- | --- | --- |
| `resources.nodes` | `1` | 申请一个节点 |
| `resources.mpi` | `64` | 生成 Slurm 的 64 任务申请 |
| `programs.launcher` | `[..., "-np", "64"]` | QE／EPW 实际启动 64 个 MPI 进程 |
| `programs.epw.args` | `["-nk", "64"]` | EPW 使用 64 个 pool，每 pool 一个进程 |

当前 EPW 源码要求进程数等于 pool 数与 image 数的乘积，粗网格 shuffle 路径还要求每 pool 一个进程；不能仅把 MPI 进程改为 64，却保留原来的 `-nk 24`。这里沿用单 image 配置，同时设 `-nk 64`，不改变物理网格或数值标准。

**申请并启动 64 核不代表每一步都能持续达到 100% CPU 利用率。** 本例完整粗 k 网格只有 32 点，部分阶段存在空闲 pool 或串行工作；实际加速和 64 进程科学结果仍需计算后验证。本次资源调整及脚本检查不代替真实运行验收，也不自动提交作业。

资源参数同样属于固定配置，旧的 24 进程快照不能直接改写。此次在确认无活动作业、无科学执行记录后，将旧准备文件和已确认无存活所属进程的准备锁保留到 `.epwflow/preparation-history/0a391efde412aa34-mpi24/`，再用原 `work/` 路径生成新的 64 进程绑定。该归档只作历史证据，不直接提交其中仍引用原绝对路径的脚本。后续若已有执行或未知作业，先核对状态，再决定新运行位置，不套用本次准备阶段的处理。

## 4. 生成并检查第一阶段脚本

```bash
"$EPW_BIN/epw_phonon" -c "$CFG"
cat "$W/.epwflow/generated/qe-data.sh"
"$EPW_BIN/epw_phonon" --status -w "$W" --json
```

不加 `-s`／`-r` 时，仅固定输入并生成脚本，不调用科学程序、不提交 Slurm、不预留预算。核对脚本中的分区、节点数、MPI 数、时限、环境加载、工作目录和程序路径。

生成的 `input-init.toml`、`POSCAR-init`、`KPATH-init` 和 `run-binding.json` 用于绑定输入及程序身份。准备后 `workflow_complete: false` 是正常状态；没有作业记录时的 `in_progress` 也不代表已有作业在运行。

## 5. 按阶段提交与验收

### 第一阶段：QE 声子与 EPW 所需数据

确认脚本和预算后，在登录节点提交：

```bash
"$EPW_BIN/epw_phonon" -c "$CFG" -s
squeue -u "$USER"
"$EPW_BIN/epw_phonon" --status -w "$W" --refresh --json
```

`-s` 提交一个有界 Slurm 作业，执行共享 qeflow nosplit 的 SCF→PH→q2r→matdyn→声子图，并归档响应数据、准备完整均匀 NSCF。无需另行启动 qeflow。提交回执只证明调度器已接收作业。

`--refresh` 查询调度记录并结算已知终态，可能更新运行记录和账本；不运行科学计算。继续下一阶段前须看到对应边界 `boundary_complete: true`、`workflow_status: "succeeded"`，并核对清单与产物。

### 第一阶段完成后：读取实际计算目录

第一阶段完成后，若新开了终端或不确定变量是否还在，可以直接执行下面整段命令。它只切换终端目录、设置路径变量和读取运行记录，不创建目录、不移动数据、不重新计算，也不提交作业。`workflow.json` 在首次登记执行后才存在：

```bash
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"

EPW_BIN="/data/home/mayuan/code/dftflow/bin/epwflow"
CFG="$PWD/epwinput.toml"
W="$PWD/work"

CALC=$(python -c 'import json,sys; print(json.load(open(sys.argv[1]))["calculation_dir"])' "$W/.epwflow/workflow.json")
printf '%s\n' "$CALC"
```

| 命令 | 具体作用 |
| --- | --- |
| `cd ".../run/epwflow"` | 进入本例目录；后面的 `$PWD` 才会指向这里 |
| `EPW_BIN=".../bin/epwflow"` | 记住按模块安装的程序目录，后续通过 `"$EPW_BIN/epw_wannier"` 调用第二阶段程序 |
| `CFG="$PWD/epwinput.toml"` | `$PWD` 是当前目录的绝对路径；拼接后得到本例配置文件的完整路径 |
| `W="$PWD/work"` | 记住工作目录路径，后续从中读取状态和生成脚本；不会创建 `work/` |
| `CALC=$(python ...)` | 从本次运行记录中读取 `calculation_dir`，把实际计算目录的绝对路径存入变量 `CALC` |
| `printf '%s\n' "$CALC"` | 将读到的路径显示在终端，末尾换行；不写入文件 |

Python 这一行中，`-c` 表示执行后面的短代码；`sys.argv[1]` 是传入的 `workflow.json` 文件路径；`json.load(...)` 读取该文件，`["calculation_dir"]` 取出实际计算目录。外层的 `$(...)` 将 Python 打印的路径交给 `CALC`。因此这里的 Python 只负责读 JSON，不运行科学计算。

`W` 和 `CALC` 的区别是：`W` 指向整个 `work/`，`CALC` 指向其中某一次实际计算的目录，即 `work/calculations/<尝试号>/`。尝试号是程序自动生成的目录标识，无需手动填写；恢复运行时也可能沿用已有计算目录，所以直接读取 `calculation_dir`，不要用其他编号猜路径。变量是当前终端中的路径简称，新开终端后重新执行上述设置即可；它们不会写入 `.bashrc` 或改变全局程序安装。

确认 `printf` 显示的是本例 `work/calculations/` 下已有的绝对路径；若 Python 报错或路径为空，先核对 `W` 和运行记录，不能继续把空的 `CALC` 用于检查。随后检查第一阶段交付数据：

```bash
"$EPW_BIN/epw_phonon" --check -w "$CALC" --json
```

此时应有 `qe-data/manifest.json` 且数据检查为 `valid`。`no_data`、`invalid` 或 `legacy_unverified` 均不作为通过。

### 第二阶段：Wannier、粗网格和独立质量

仅在第一阶段完成且数据检查通过后执行；如果刚打开终端，先执行上面的变量设置段：

```bash
"$EPW_BIN/epw_wannier" -c "$CFG"
cat "$W/.epwflow/generated/interpolation.sh"
"$EPW_BIN/epw_wannier" -c "$CFG" -s
"$EPW_BIN/epw_wannier" --status -w "$W" --refresh --json
```

上面四条命令依次是：生成第二阶段脚本；显示脚本供核对；用 `-s` 实际提交第二阶段 Slurm 作业；查询并刷新阶段状态。最后一条不会等待作业结束，作业运行期间可再次执行以查看进度。

该阶段执行原生 Wannier／粗网格、同源独立参考和质量核验，并生成比较数据和诊断图；不执行细网格 EPC 或 α²F。检查固定窗与实际 `.wout` 四界、收敛、有限性和独立参考误差。失败时保留诊断，不能因图像存在或 Slurm 为 COMPLETED 就启动第三阶段。

通过后应有本次科学目录下的 `wannier/manifest.json`，且工作流记录包含已完成的插值边界。前两阶段整体 `workflow_complete` 仍可为 false。目录取值以最新 `workflow.json` 为准；不要把通用 `--check` 的 `no_data` 当作第二阶段已完成。

### 第三阶段：细网格 EPC 与超导

仅在第二阶段完成且质量通过后执行：

```bash
"$EPW_BIN/epw_superconductivity" -c "$CFG"
cat "$W/.epwflow/generated/all.sh"
"$EPW_BIN/epw_superconductivity" -c "$CFG" -s
"$EPW_BIN/epw_superconductivity" --status -w "$W" --refresh --json
```

`all.sh` 是第三阶段脚本的内部命名。此阶段核验粗网格清单，执行细网格 EPC、原生 α²F／λ、iso／aniso 求解及有界 Tc 搜索，再收集结果并出图。检查全部所需模式、收敛和结果包，`partial` 或未定位状态不算完整 Tc 通过。

```bash
"$EPW_BIN/epw_superconductivity" --results-check --results "$W/results" --json
```

只有完整工作流记录、科学检查和交付均通过，才能宣称本次全链完成。本例的真实 PBE 全链及完整 Tc 科学验收仍未完成；命令可调用不表示已经得到正确 Tc。

## 6. 查看日志和图表

```bash
cat "$W/.epwflow/workflow.json"
find "$W/.epwflow" -type f \( -name '*.out' -o -name '*.err' \)
find "$W/calculations" -type f \( -name 'scf.out' -o -name 'ph.out' -o -name 'epw.out' \)
find "$W/calculations" -path '*/results/figures/*' -type f
```

直接 `-s` 的提交尝试和日志位于 `.epwflow/slurm/`；手工提交生成脚本时，调度日志按脚本头部写到 `.epwflow/generated/`。原生科学输出位于 `calculations/<尝试号>/` 的阶段子目录，比较图和最终结果包的根目录不同。

比较图包括电子能带／DOS、声子色散／DOS 四张图及总览，通常有 PNG／SVG／PDF。严格元素／轨道投影仍暂缓，缺失面板应明确标注。已有结果包可以独立重绘，以下命令不运行 QE／EPW：

```bash
"$EPW_BIN/epw_wannier" --plot --results "$CALC/results" --formats png,svg,pdf
```

先确认该次比较结果包确实存在；不要对空目录执行重绘来代替科学阶段。

## 7. 失败恢复

先查看日志、刷新状态并确认原作业已终止。然后只执行实际失败阶段对应的一条命令：

```bash
# 第一阶段失败时
"$EPW_BIN/epw_phonon" --resume -w "$W" -s
# 第二阶段失败时
"$EPW_BIN/epw_wannier" --resume -w "$W" -s
# 第三阶段失败时
"$EPW_BIN/epw_superconductivity" --resume -w "$W" -s
```

恢复读取绑定目录内的固定快照，重验来源与成功产物，保留失败现场。活动或未知作业不重提，质量失败不能通过重试绕过。若恢复的是历史运行，先将 `W` 改成其 `run-binding.json` 所属目录，并核对该运行绑定的程序；不要把新 `work/` 与本目录旧绑定混用。

配置、结构、赝势或科学程序改变须另建运行，不能编辑 `input-init.toml` 消除身份冲突。内部 `.epwflow/slurm/` 脚本不手工重提；登录节点也不直接使用 `-r` 运行科学阶段。

## 8. 可选：一次生成三阶段依赖脚本

初次手动测试建议按第 5 节逐阶段核查。需要依赖链时，使用同一组程序和配置：

```bash
# 仅生成三阶段与总提交脚本
"$EPW_BIN/epw_phonon" --all -c "$CFG"
cat "$W/.epwflow/generated/submit-all.sh"
# 此命令会实际提交三阶段 afterok 依赖链
bash "$W/.epwflow/generated/submit-all.sh"
```

这是另一种提交方式，不与逐阶段 `-s` 混用。总脚本在登录节点用 `bash` 执行；总回执位于 `.epwflow/generated/chain-submission.json`。部分或未知提交须先核查，不删除回执后重提。依赖链提交成功也不替代逐阶段科学验收。

完整命令和当前边界见[模块手册](../../../../../epwflow/README.md)与[当前功能总览](../../../../../docs/overview/epwflow/README.md)。第 0 节是标明日期的目录核对截面，其余章节提供手动操作步骤；后续实时状态以原生证据和工作流记录为准。
