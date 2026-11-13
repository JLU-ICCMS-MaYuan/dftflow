# Ubuntu 上的 MgB₂ EPW 自测试

本目录是独立工作区，配置为 `input.toml`，所有阶段、日志、预算和结果均生成在本目录。Git 仅跟踪本输入和本文档；仓库现有 `/examples/` 忽略规则屏蔽其他新文件，不要强制添加计算数据。

## 输入与测试范围

- 复用仓库 `test_auto/MgB2/MgB2.vasp` 结构及 `test_auto/MgB2/pp/` 的 PBE 赝势，无须在本目录再跟踪结构或赝势。
- SCF／NSCF 网格 4×4×2，16 条带；PH 为 index 模式、2×2×1；细网格 k 为 8×8×4、q 为 4×4×2。截断能为 40／320 Ry。
- 5 个 Wannier 函数，B:pz 与三个键中心 s 初始投影；使用 `energy_only` 自动选窗，实际能窗须查看本次原生日志。
- 第 8 阶段为 iso／aniso 的 20／30／40 K 能隙计算，μ*=0.1，不执行 Tc 搜索。
- 资源为本机 `local` 分区、单节点、2 MPI。小网格用于检查执行链与图表，不代表 MgB₂ 的物理结果或网格已收敛。
- 独立账本为 `campaign/ledger.json`；没有自动扩大历史项目预算的含义。完整运行前检查生成脚本和本机资源限制。

## 本机环境

```bash
source "/home/mayuan/bin/dftflow-env.sh"
source "/home/mayuan/code/dftflow/scripts/env.sh"
cd "/home/mayuan/code/dftflow/examples/MgB2/ubuntu"
epw_phonon -c "input.toml" --check --json
```

编译及运行环境入口统一放在 `~/bin/`：`dftflow-env.sh` 内部加载 `intel-env.sh`，后者激活 Conda 构建环境和 Intel oneAPI。只需 source 前者。TOML 使用绝对路径，避免依赖 `~` 展开。仓库 `scripts/env.sh` 负责加载构建后的产品入口。

配置中的科学程序和环境脚本使用本机绝对路径；迁移机器时应修改。相对结构、赝势和 KPATH 路径均以输入文件所在目录为基准。

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

查看 `epw-state/` 阶段日志、各阶段原生日志和 `results/`。重点检查 SCF／PH 正常结束、Wannier 收敛、QE／EPW 能带与声子对比、α²F／λ 和两个方法的能隙收敛。作业成功退出不能代替科学收敛检查。

## 2026-11-13 准备检查

已验证 TOML 语法、引用文件、科学程序执行权限、Intel 运行库加载，以及 `local` 分区可用；加载环境后 `pw.x -help` 显示 QE 7.5。

当前环境找不到 `cargo`／`rustc`，仓库尚无 `bin/epwflow/` 及 EPW 三入口，因而未能运行 `epw_phonon --check`，也未提交科学作业。此输入基于仓库版本 2 测试配置整理，尚待真实 CLI 和原生链验证，不能声称计算通过。先准备 Rust 工具链并按仓库构建说明构建 EPW 入口，再从上面的配置检查开始。
