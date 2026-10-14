# MgB₂ / sugon-amd QE 声子测试

## 机器配置

本机已核对 QE 7.4.1 的 `pw.x`、`ph.x`、`q2r.x`、`matdyn.x` 存在。采用 `.my_scriptrc.py` 中启用的 `amd7513` 分区，1 节点、64 MPI 进程，排除 node02、node21、node07；QE 路径按用户要求升级至 `qe-7.4.1-amd-oneAPI2024`。本地配置已写入 `docs/local/MgB2-sugon-amd.env`。

从仓库根目录操作。DFTFlow 不会自动加载 `.my_scriptrc.py`，也不要执行该文件来生成配置。共享脚本只加载指定的本地环境文件。

本地文件 `docs/local/MgB2-sugon-amd.env` 使用以下格式（除 sugon-amd 已配置外，需填写实际值）：

```bash
QE_BIN=/实际路径/qe-7.4.1-amd-oneAPI2024/bin
PP_DIR=/实际路径/赝势目录
ENV_SCRIPT=/实际路径/oneapi_setvars.sh
DFTFLOW_BIN=/实际路径/dftflow/bin
PARTITION=实际Slurm分区
NTASKS=64
EXCLUDE=
```

`PP_DIR` 必须包含 `b_pbe_v1.4.uspp.F.UPF` 和 `mg_pbe_v1.4.uspp.F.UPF`；脚本复制这两份至运行目录，避免多版本匹配歧义。示例附带仓库 MgB₂ 参考测试的 `KPATH.in`，准备时直接复制，不需要调用 VASPKit。当前模板要求路径不含空格或 TOML 引号字符。

## 准备与检查

```bash
# 在 dftflow 仓库根目录执行
bash examples/MgB2/prepare.sh sugon-amd
cd examples/MgB2/sugon-amd/run
cat input.toml
cat qe_phonon/scf.in
cat qe_phonon/ph.in
cat qe_phonon/KPATH.in
cat submit_phonon.sh
```

准备命令不提交任务。生成的 `input.toml` 和提交脚本使用绝对路径，因此生成后不要移动运行目录。已有 `run/` 时脚本拒绝覆盖，应先归档。

## 提交计算

在该机器允许提交任务的主节点执行：

```bash
# 当前目录为 examples/MgB2/sugon-amd/run
sbatch submit_phonon.sh
squeue -u "$USER"
```

一个作业顺序执行 SCF、PH、q2r、两次 matdyn、收集与绘图。SCF/PH 用配置的 MPI 进程数，q2r/matdyn 单进程执行。计算环境在作业内加载，OMP/MKL 各 1 线程。当前 `qe_phonon_nosplit -r` 未完整执行所有后处理步骤，自动 `-s` 路径存在重复后处理及配置路径衔接问题，因此本手册使用生成输入后显式 `sbatch` 的流程。

## 监控与结果

```bash
tail -n 30 qe_phonon/scf.out
tail -n 30 qe_phonon/ph.out
rg 'JOB DONE' qe_phonon/*.out
ls qe_phonon/phband/
ls qe_phonon/phdos/
```

作业日志位于 `run/MgB2_ph-<作业号>.out` 和 `.err`；各 QE 步骤日志位于 `run/qe_phonon/*.out`。声子谱、DOS 的 CSV 和图像分别位于 `phband/`、`phdos/`。终止标记只说明程序正常结束，仍需检查 SCF 和声子收敛及结果合理性。

## 参数与验收边界

初始 k 网格 `4×4×2`、声子 q 网格 `2×2×1`、DOS 插值网格 `2×2×2`，截断能 40/320 Ry，Gaussian 展宽 0.02 Ry。这些是流程测试参数，正式结果需单独完成结构、截断能、k/q 网格、展宽和阈值收敛验证。`ldisp=true` 明确开启声子网格计算。

本目录有可运行材料不等于计算验收已通过；只有真实作业成功后才能记录声子计算完成。
