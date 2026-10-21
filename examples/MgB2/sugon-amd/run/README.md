# MgB₂：sugon-amd 手动测试入口

当前用户指定的 EPW 工作区是本目录下的 [epwflow/](epwflow/README.md)，输入为 `epwflow/epwinput.toml`；共用结构为本目录 `POSCAR`，赝势为 `pp/`。此前放在父目录的编号阶段已由用户删除，不能把历史作业中的旧路径当作当前结果。

```bash
source "/data/home/mayuan/code/dftflow/scripts/env.sh"
cd "/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/epwflow"
epw_phonon -c epwinput.toml --check --json
epw_phonon -c epwinput.toml --status --json
```

`source` 将 `bin/` 下各产品目录加入当前终端 PATH；`-c` 选择用户指定的 TOML。`--check` 检查配置与并行，`--status` 核对复用条件，均不提交作业。各阶段提交、失败恢复、日志／数据目录及状态字段解释见 [EPW 操作说明](epwflow/README.md)，不需要手工配置 CALC、CFG、W 或 LEDGER。

| 模块 | 说明 |
| --- | --- |
| [epwflow](epwflow/README.md) | 本轮真实 QE／EPW debug 工作区 |
| [qeflow](qeflow/README.md) | QE 独立算例占位 |
| [scflow](scflow/README.md) | 超导后处理算例占位 |
| [substitutionflow](substitutionflow/README.md) | 元素替换算例占位 |
| [vaspflow](vaspflow/README.md) | VASP 算例占位 |
| [wannierflow](wannierflow/README.md) | 独立 Wannier 算例占位 |

程序仍只有三个 EPW 公开入口：`epw_phonon`、`epw_wannier`、`epw_superconductivity`。生成脚本没有自动严格 shell 选项；关键命令显式传播失败。每次提交读取当前 TOML 并冻结该次请求，排队后编辑不改变已提交任务；后续提交使用新配置。活动或终态未知的任务禁止重提，不删除未知锁，不修改冻结脚本规避哈希校验。

当前配置的粗 k 网格为 4×4×2，共 32 点，使用 32 MPI／32 pool。当前 EPW 路径每 pool 一个 MPI，不能只改成 64 pool。预算由 TOML 中已有 ledger 管理，累计消费和预留不会因目录变化清零。

历史问题与证据保留在 [0018 目录与续算](../../../../docs/specs/0018-epwflow-stage-workspaces/acceptance.md)、[0020 能带后处理](../../../../docs/specs/0020-qe-electronic-postprocess-resume/acceptance.md)、[0021 DOS 后处理](../../../../docs/specs/0021-qe-dos-pencil-resume/acceptance.md)、[0023 Wannier 原生启动](../../../../docs/specs/0023-epw-wannier-native-start/acceptance.md) 和 [0025 可见状态](../../../../docs/specs/0025-epw-visible-run-state/acceptance.md)。本轮实际版本、作业结果、数据保护和科学边界见 [0027 验收](../../../../docs/specs/0027-mgb2-epwflow-example-debug/acceptance.md)。协议测试通过、安装成功和科学验收分别记录。


## MPI 脚本写法

活动配置中的 MPI 启动器使用 `launcher = ["mpirun", "-np", "32"]`；环境脚本负责提供 MPI PATH，新任务普通调用 `mpirun`，无 `exec` 或 MPI 绝对路径。科学程序路径和并行参数保留。旧唯一任务脚本和冻结快照不改写；现用脚本迁移及安装指纹见 [0044 部署记录](../../../../docs/specs/0044-mpi-submit-command/deployment.md)。
