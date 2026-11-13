# Ubuntu 上的 MgB₂ EPW 自测试

本目录是独立工作区，配置为 `input.toml`，阶段、日志和结果生成在本目录的 `run/`，账本放在同级 `campaign/`，所有运行数据仍位于 ubuntu 内。Git 仅跟踪本输入和本文档；仓库现有 `/examples/` 忽略规则屏蔽其他新文件，不要强制添加计算数据。

## 输入与测试范围

- 复用仓库 `test_auto/MgB2/MgB2.vasp` 结构及 `test_auto/MgB2/pp/` 的 PBE 赝势，无须在本目录再跟踪结构或赝势。
- SCF／NSCF 网格 4×4×2，16 条带；PH 为 index 模式、2×2×1；细网格 k 为 8×8×4、q 为 4×4×2。截断能为 40／320 Ry。
- 5 个 Wannier 函数，B:pz 与三个键中心 s 初始投影；使用 `energy_only` 自动选窗，实际能窗须查看本次原生日志。
- 第 8 阶段为 iso／aniso 的 20／30／40 K 能隙计算，μ*=0.1，不执行 Tc 搜索。
- Slurm header 申请本机 `local` 分区、单节点、12 任务；launcher 启动 12 MPI。小网格用于检查执行链与图表，不代表 MgB₂ 的物理结果或网格已收敛。
- 账本为 `campaign/ledger.json`，沿用程序要求的 `epic-0007` 身份；没有自动扩大历史项目预算的含义。完整运行前检查生成脚本和本机资源限制。

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

## 本次统一配置后的检查

正式 EPW 三入口的 `--check --json` 均返回 `operation_status=ok`、`workflow_status=configured`。首次通过正式入口生成 `run/1.qe-scf/scf.sh` 时，核对 header、`mpirun -np 2` 与 `~/bin/dftflow-env.sh`，未提交作业。gap 输入显式设置 `tc_linear=false` 和 `gap_edge=0.01` eV，当时 EPW 原生 pool 为 2；这些是输入完整性与并行约束，不是科学收敛结论。

全部正式命令已在固定 `bin/<模块>/` 覆盖。359 项 EPW 回归与 140 项其他模块回归通过；15 项既有暂挂测试不计通过。详见本次验收记录。Git 仍只纳管本目录的 `input.toml` 和本文档。

2026-11-13 按用户要求将当前配置调整为 12 MPI／12 Slurm 任务／12 EPW pool。三个正式入口重新检查通过。本次更新旧 SCF 脚本时，因已有作业记录触发 sacct 查询且本机 accounting storage is disabled，CLI 拒绝继续；现存 scf.sh 仍为 2 路，不能作为本次 12 路配置的有效执行脚本。恢复调度记账查询后需通过公开入口重新生成，不直接改写旧脚本和作业身份。本次没有提交科学作业。
