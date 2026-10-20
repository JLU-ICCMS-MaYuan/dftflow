# MgB₂ EPW 手动测试：旧现场与当前入口

当前 QE／EPW 测试直接在上一级 `run/` 使用编号目录和一份 `input.toml`。完整命令、每个参数的解释、work/scratch、状态字段以及旧数据导入见 [run 使用说明](../README.md)。本目录 `work/` 是保留的旧计算现场，不再作为日常新任务入口。

```bash
source /data/home/mayuan/code/dftflow/scripts/env.sh
cd /data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run

epw_phonon --status --json
epw_wannier --stage wannier --check --json
epw_wannier --stage wannier -s
```

这里的 `source` 将 `bin/epwflow` 等模块加入当前 PATH；`cd` 定位根配置；`--status` 核验可复用阶段，`--check` 只做配置和并行检查，`-s` 才提交计算。之后依次执行 `epw_wannier --stage coarse -s`、`epw_superconductivity --stage fine -s`、`epw_superconductivity --stage superconductivity -s`。每次等待当前作业结束，先核对结果；修改根 `input.toml` 后执行同一命令即可续算。

旧 `work/` 保存运行绑定、状态和 `calculations/<尝试号>/`；旧 `scratch/` 是配置的临时数据基址，实际数据路径由原生输入的 outdir 决定。新布局使用各编号阶段的 `out/` 和 `.epwflow/`，无需再设置这两个旧目录。旧来源及历史脚本保持原样，不手改固定快照或删除旧场景。

`operation_status="ok"` 表示这次操作成功；`--check` 检查配置字段、范围、输入/程序文件和 MPI/pool/image。`workflow_status="configured"` 表示配置/脚本已经准备。两者都不是计算完成或科学验收通过。`--status` 的 `reuse` 另核验科学参数、上游产物和输出哈希，`update` 才需要更新。

旧命令的 `CALC` 是 `work/.epwflow/workflow.json` 中 `calculation_dir` 的绝对路径；读取 JSON 后赋给变量，是为了定位旧实际计算目录。新流程由程序定位数据，不需要 `CALC`、`CFG`、`W`、`EPW_BIN` 或 `LEDGER`。账本路径已在根配置内，由程序管理共享消费。

共用结构为 `/data/home/mayuan/code/dftflow/examples/MgB2/sugon-amd/run/POSCAR`，赝势为同级 `pp/`。已有 SCF、NSCF、PH 和声子后处理通过一次性核验后导入编号目录；目录改版不会要求重跑它们。第 3 步新电子能带参考与 DOS 按需补齐。当前粗网格 32 个 k 点只支持至多 32 pool；配置 64 pool 会在原生程序启动前停止。

## 本次电子后处理失败后的恢复

PW 能带已完成而 bands.x 报 FFT 空进程时，修复后的程序保留成功 PW 数据，从后处理继续。进入上一级 run 后执行 `epw_wannier --stage wannier --resume -s`，生成新的冻结尝试；不要重提手工修改过的旧 submit.sh。原生任务完成证据位于 run/.epwflow/electronic/，详细根因、校验范围及命令解释见 [run 说明](../README.md#本次短暂运行后失败的原因与恢复)。


## DOS 再次失败的修复（0021）

1307064 已完成 bands 后处理，但 DOS 因 32 MPI 的 FFT 空进程而失败；新版为 DOS 单独启用 pencil，保留资源和物理参数。1307068 是手工 `sbatch submit.sh` 重提旧尝试，因旧作业绑定而被拒绝。请进入上一级 `run/` 再执行 `epw_wannier --stage wannier -s`（也可加 `--resume`），生成新尝试、校验并复用成功 PW／bands，从 DOS 继续。不要重提旧脚本或编辑清单；完整命令和科学验收边界见 [run 说明](../README.md#本次短暂运行后失败的原因与恢复)。
