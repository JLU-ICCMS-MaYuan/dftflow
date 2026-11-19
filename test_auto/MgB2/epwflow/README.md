# MgB₂ EPW 测试

本目录保存测试脚本和讲义；运行数据统一保存在 `../local_run/`。当前公开程序为 `epw_phonon`、`epw_wannier`、`epw_superconductivity`。人工操作见 [examples 说明](../../../examples/MgB2/sugon-amd/run/epwflow/README.md)，详细证据见 [0046～0051 全链验收](../../../docs/specs/0050-epw-stage-results/real-chain-validation.md)。

## 当前真实验收

2026-11-18，#75 的 index／coords 原生 MgB₂ 真实 Slurm 提交、取消单 q 后续算及依赖关系测试均通过。每个 q 使用 6 路 MPI、OMP／MKL 各 1 线程，SCF 使用 12 路 MPI；每种模式有两个不可约 q 点。现场位于 `../local_run/0075-native-slurm-20261118-01/{index,coords}/`，包含配置、原生日志、Slurm 依赖、作业终态、输出指纹及声子图；详细结论见 [0075 验收](../../../docs/specs/0075-phonon-parallel-qpoints/acceptance.md#原生-mgb₂-双模式真实-slurm-验收2026-11-18)。

在本机从仓库根重新执行，使用已有 Python 3.11+ 和 `toml`，不安装依赖：

```bash
python3 "test_auto/MgB2/epwflow/phonon_parallel_0075.py" \
  --root "test_auto/MgB2/local_run/0075-native-slurm-new"
```

脚本拒绝覆盖已有目录，固定测试 k 网格 `[4,4,2]`、q 网格 `[2,2,1]`，复制一份受测 CLI 作为本次运行快照，仅通过公开入口提交和续算；不会修改源配置。测试依赖本机 `local` 分区及 `jobcomp/filetxt` 完成日志，真实调用 `pw.x`、`ph.x`、`q2r.x`、`matdyn.x`，并在两种模式中各取消一个已启动的本次 q 作业。取消后 collect 必须等待，续算只能替换该失败 q 的作业号；成功 q 的输入、日志、动力学矩阵指纹与修改时间、init／collect 作业号均保留。`result.json` 保存终态、并行重叠时间、九模频率和图片身份；完成后重复公开命令不得新增作业。此项验证调度与恢复，不作为网格收敛或 Tc 验收。

`012-stage-results/` 使用 examples 当前 PBE 小网格配置，从 SCF 开始完成八阶段，含 iso／aniso 的 20、30、40 K 六点；未搜索 Tc、调整选窗或扫描网格。为了验证电子交付不依赖声子，测试顺序为 1 → 2 → 3 → 5 → 4 → 6 → 7 → 8。第 6 步后处理舍入误判修复后，显式恢复复用了成功粗网格检查点，旧制品原样保留。0051 合并版本随后补交付 15 张衰减图，源文件哈希及科学调用数保持不变；仓库与默认安装现已更新到同一受测制品。

`011-stage-results/` 保留首轮路径行截断和日志归档问题的诊断证据，不作为最终通过现场。`001`～`010` 是历史科学尝试或替身进程验证，按各自清单只读追溯；历史阈值、脚本和安装说明不代表当前执行规则。[0015 历史记录](../../../docs/specs/0015-epwflow-mgb2-real-validation/acceptance.md)继续保留。

## 重新执行真实测试

先按[构建说明](../../../scripts/build-native.sh)编译；交付构建会自动完成本机配置要求的兼容打包并更新仓库模块目录。测试应传入已经在计算节点可启动的制品；在运行期间不能覆盖绑定程序。以下命令从仓库根执行，使用已有 Python 3.11+，不安装 Python 依赖。

```bash
EPW_TEST_PYTHON="/data/home/mayuan/anaconda3/bin/python3"
EPW_TEST_CASE="$PWD/test_auto/MgB2/local_run/014-stage-results"
EPW_TEST_BIN="$PWD/bin/epwflow"

"$EPW_TEST_PYTHON" "test_auto/MgB2/epwflow/real_chain.py" prepare \
  --case "$EPW_TEST_CASE" --bin "$EPW_TEST_BIN"
"$EPW_TEST_PYTHON" "test_auto/MgB2/epwflow/real_chain.py" run --case "$EPW_TEST_CASE"
"$EPW_TEST_PYTHON" "test_auto/MgB2/epwflow/real_chain.py" audit --case "$EPW_TEST_CASE"
"$EPW_TEST_PYTHON" "test_auto/MgB2/epwflow/real_chain.py" portable --case "$EPW_TEST_CASE"
```

`prepare` 拒绝覆盖已有目录，仅隔离结构和工作目录路径，保留科学参数、外部程序、资源和共享账本；冻结配置、程序及其 SHA-256。`run` 真正调用公开 `-s` 并等待 Slurm，会记录实际消费。0058 取消程序内的累计节点时与存储额度，保留最多两个作业的登记约束；集群自身规则仍由 Slurm 管理。源配置必须删除 `resources.wall_seconds`、`resources.storage_bytes`，旧冻结快照不会自动改写；历史现场及当时的预算证据保持原样。

每阶段先核验作业内完成及即时发布，再重复公开命令检查没有新增科学任务。第 1／2 步按公开协议在作业结束后再次核验。第 8 步只提交两个方法作业，每种方法一次 EPW 执行完整温度列表。失败时立即停止，日志、回执和科学检查点保留，不自动取消、重复提交或更改参数。

只恢复一个已修复的失败阶段：

```bash
"$EPW_TEST_PYTHON" "test_auto/MgB2/epwflow/real_chain.py" run \
  --case "$EPW_TEST_CASE" --stage coarse --resume
```

如果确实需要换修复后的编排器，先确认本现场作业已终态，再执行 `upgrade --case "$EPW_TEST_CASE" --bin /修复制品目录 --reason "具体缺陷及修复原因"`。脚本保留全部旧程序和科学文件，把新程序放在新的修订目录；不会覆盖已提交脚本绑定。它不能绕过生产程序的输入和来源核验。

## 核验内容与现场位置

| 位置 | 内容 |
|---|---|
| `baseline.json`、`bin/` | 初始配置、外部程序和冻结编排器身份 |
| `active-programs.json`、`bin-revision-*` | 如有修复，保存原因与新旧程序身份 |
| `evidence/` | 每条公开命令、退出码、Slurm 终态和逐阶段复用检查 |
| `run/` | 八阶段原生科学输入、输出、固定脚本和日志 |
| `run/results/` | 第 3／4／5／6／8 步图片、数值表、绘图参数和来源；第 7 步原生谱供第 8 步图包使用 |
| `audit.json` | 科学任务数量、Mg／B 加和误差、六点有效性及程序路线 |
| `portable-results/`、`portable-results-0051/` | 首批和合并完整 results 的移动副本，空 PATH 重绘与核验 |
| `ledger-before.json`、`ledger-final.json` | 实际执行前后共享账本快照 |

`audit` 针对本固定 MgB₂ 验收：PW 三次（SCF、NSCF、路径 bands），PH 一次，postw90 一次且只承担电子 DOS，没有独立 `wannier90.x`。第 6 步因为 ASR 口径及所需 q 点不同，首次补一次 q2r 和 matdyn；其事前原因保存在比较目录，不能称为直接复用不相容数据。

0051 追加验收复用该现场已有 coarse，检查 15 张衰减图、原生逐点数据及科学调用不增加；未重做成功科学阶段。

`portable` 复制整个结果目录，在 PATH 不含科学程序的环境中执行公开重绘与结果检查，确认数值表哈希不变；已有副本会拒绝覆盖；用 `--portable-name portable-results-0051` 可另存新的验证副本。当前图和差值用于人工判断，不自动评级或判断粗网格收敛。

## 回归与证据边界

Rust 进程回归使用替身 QE／EPW／Slurm，检查失败恢复、未知作业、来源损坏、图像发布和幂等；真实链单独验证科学程序实际执行，二者不能互相替代。主工作区存在并行开发时，应从明确提交和本任务补丁建立源码快照，使用独立 Cargo target，避免编译中替换测试进程。

```bash
CARGO_PROFILE_RELEASE_DEBUG=0 bash "scripts/build-native.sh" test --locked --release \
  -p epwflow -p qeflow --no-fail-fast -- --test-threads=1
```

本轮冻结源码、构建日志、制品哈希、预算和作业列表见全链验收。线上 Gitee 仍由用户管理；本地通过不代替线上 Issue 关闭。

## 0055：电子八图与 QE 元素 PDOS

`014-electronic-plots/validation-v2/` 复用 `012-stage-results/` 已完成的 PBE 科学数据；只补必要 projwfc，输出八张双能窗电子图，不重跑 PW／PH／Wannier／EPW。成功原生投影即使后续解析失败也保留检查点，最后的 1370782 作业只复用并完成出图。来源的 69 个 NSCF 文件与 7 个直接参考文件哈希保持不变。实际过程、调用边界及待办见 [0055 验收](../../../docs/specs/0055-electronic-plot-windows-pdos/acceptance.md)。

新增脚本：

- `electronic_0055.sh <已构建的 electronic_0055_real 测试程序> <新输出目录>`：仅在已登记 Slurm 分配内执行；原始测试程序在仓库 `bin/.build`，兼容加载器来自仓库 `bin/.native`，不复制编排器到 test_auto。
- `verify_electronic_0055.py <results目录>`：以现有 Pillow 检查 8 张 PNG、3000×2100、300 DPI、相同能窗、红色费米线、蓝色插值及真实 Mg／B PDOS。该脚本只验证展示和文件完整性，不评定科学精度。

构建真实用例但不执行：

```bash
CARGO_PROFILE_RELEASE_DEBUG=0 bash scripts/build-native.sh test --release --locked \
  -p epwflow --test electronic_0055_real --no-run
```

按构建日志给出的测试程序路径，在已登记的 Slurm 分配内调用上述 Bash 脚本；新输出目录须位于 `test_auto/MgB2/local_run/`。恢复仅允许未完成现场，需显式设置 `DFTFLOW_0055_RESUME=1`；已完成现场拒绝覆盖。没有相容旧投影时该测试会实际运行 projwfc，不能在登录节点调用。重画已有包直接使用 `epw_wannier --plot --results <结果目录>`，不运行此真实后处理测试。


## 高对称端点路径

第 5 步沿用只写高对称端点对的 `KPATH.in`，例如：

```text
0 0 0 G
0.5 0 0 M

0.5 0 0 M
0.3333333333 0.3333333333 0 K
```

生成的 `wdata` 每条真实边只有一行，不列中间点或 NSCF 网格：

```text
begin kpoint_path
G 0 0 0 M 0.5 0 0
M 0.5 0 0 K 0.3333333333 0.3333333333 0
end kpoint_path
```

`5.epw-wannier.bands_num_points` 控制 Wannier90 首段间隔数，可按需要设置为 `100` 等正整数；后续段按实际长度比例取整，短段被舍入为零时提前报错。两侧能带保留完整原生点，仅实际同坐标才计算能量差，不注明共同点覆盖率；DOS 继续使用 postw90 均匀网格。第 6 步直接复用第 5 步电子表格。

以上说明新版源码行为；已有冻结脚本及计算不会自动改写。本轮只执行隔离替身回归，未切换安装或启动真实科学程序。详见 [0063 验收](../../../docs/specs/0063-epw-high-symmetry-path/acceptance.md)。
