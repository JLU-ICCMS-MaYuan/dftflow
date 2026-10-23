# MgB₂ EPW 测试

本目录保存测试脚本和讲义；运行数据统一保存在 `../local_run/`。当前公开程序为 `epw_phonon`、`epw_wannier`、`epw_superconductivity`。人工操作见 [examples 说明](../../../examples/MgB2/sugon-amd/run/epwflow/README.md)，详细证据见 [0046～0051 全链验收](../../../docs/specs/0050-epw-stage-results/real-chain-validation.md)。

## 当前真实验收

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

`prepare` 拒绝覆盖已有目录，仅隔离结构和工作目录路径，保留科学参数、外部程序、资源和共享预算；冻结配置、程序及其 SHA-256。`run` 真正调用公开 `-s` 并等待 Slurm，因此会消费原共享预算。每次提交仍受累计 28800 节点秒／100 GiB、最多两个作业的约束，不按测试目录重置。

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
