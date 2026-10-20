# MgB₂ 在 sugon-amd 上的 EPW 手动测试

本目录用于逐阶段手动测试 `epw_phonon`、`epw_wannier`、`epw_superconductivity`。当前先测试 EPW；其他工作流的说明见相邻目录。小网格参数用于检查流程，不能作为已经收敛的 MgB₂ Tc。

## 1. 确认目录、程序和输入

在登录节点执行。以下变量只在当前终端有效，新开终端需重新设置：

```bash
REPO="/data/home/mayuan/code/dftflow"
RUN_DIR="$REPO/examples/MgB2/sugon-amd/run"
EPW_DIR="$RUN_DIR/epwflow"
EPW_BIN="$REPO/test_auto/MgB2/local_run/010-public-pbe/bin-window-contract"
CFG="$EPW_DIR/epwinput.toml"
W="$EPW_DIR/work"
LEDGER="$REPO/docs/local/epwflow-campaign.json"
cd "$EPW_DIR"

ls -l "$CFG" "$RUN_DIR/POSCAR" "$RUN_DIR/pp"
"$EPW_BIN/epw_phonon" --help
"$EPW_BIN/epw_wannier" --help
"$EPW_BIN/epw_superconductivity" --help
```

结构文件使用 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR`，赝势目录使用 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/pp`。两者均位于本目录的上一级 `run/`，无需复制到 `epwflow/`。

`EPW_BIN` 指向当前已有、供登录／计算节点使用的运行库兼容打包制品，并与本目录原有生成脚本的程序路径一致；它不是默认安装。若该目录已清理，先重新核对受测制品位置，不直接换成 PATH 中的旧程序。帮助输出只能确认入口可启动，不能证明 QE／EPW 在计算节点已通过运行检查。

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

### work/ 和 scratch/ 分别放什么

`work/` 是这次运行的完整工作目录：保存固定输入、提交和恢复记录、实际科学计算文件以及结果。它不只是存放日志的小目录；当前实现也将波函数、响应势等大文件保存在其下。

| 位置（相对于 `epwflow/`） | 内容与作用 |
| --- | --- |
| `work/input-init.toml`、`POSCAR-init`、`KPATH-init`、`run-binding.json` | 固定配置、结构和路径快照，以及输入／程序身份绑定；续算时用于确认来源一致 |
| `work/.epwflow/` | 生成脚本、提交尝试、Slurm 日志和 `workflow.json`；记录作业、阶段状态及实际 `calculation_dir` |
| `work/calculations/<尝试号>/` | 实际科学目录，含输入、输出日志、保存树、清单、独立参考和诊断结果；恢复可继续复用原科学目录 |
| `work/results/` | 完整流程收集的最终结果包和图表；第二阶段的比较结果另在实际科学目录的 `results/` 下 |
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

本例当前配置为 `amd7513`、1 节点、24 MPI、每阶段最长 7200 秒；SCF／NSCF 为 4×4×2，粗 q 为 2×2×1。电子插值最大／RMS 门为 0.05／0.02 eV，声子门为 50／20 cm⁻¹；以实际配置为准，不为跑通而放宽。

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

取得实际科学目录的方法如下；`workflow.json` 在首次登记执行后才存在：

```bash
cat "$W/.epwflow/workflow.json"
```

将其中 `calculation_dir` 的绝对路径赋给 `CALC`，不要照抄别次运行的尝试号：

```bash
CALC="此处替换为本次 calculation_dir 的绝对路径"
"$EPW_BIN/epw_phonon" --check -w "$CALC" --json
```

此时应有 `qe-data/manifest.json` 且数据检查为 `valid`。`no_data`、`invalid` 或 `legacy_unverified` 均不作为通过。

### 第二阶段：Wannier、粗网格和独立质量

仅在第一阶段完成后执行：

```bash
"$EPW_BIN/epw_wannier" -c "$CFG"
cat "$W/.epwflow/generated/interpolation.sh"
"$EPW_BIN/epw_wannier" -c "$CFG" -s
"$EPW_BIN/epw_wannier" --status -w "$W" --refresh --json
```

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

完整命令和当前边界见[模块手册](../../../../../epwflow/README.md)与[当前功能总览](../../../../../docs/overview/epwflow/README.md)。本 README 提供手动操作步骤，不记录作业完成状态；原生证据和工作流记录才是本次运行的核查依据。
