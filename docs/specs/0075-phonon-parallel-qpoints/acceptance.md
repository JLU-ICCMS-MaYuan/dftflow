# 0075 验收记录

关联：[Issue #75](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/75)、[Spec](spec.md)、[方案](plan.md)。

## 验证边界

首次实现验证覆盖本地状态、脚本、公开进程协议，以及采用隔离 I/O 替身的真实 Slurm 取消恢复。2026-11-18 后续补齐原生 MgB₂ 的 index／coords 双模式真实并行计算、取消恢复、后处理和绘图，见本文“原生 MgB₂ 双模式真实 Slurm 验收”。本次只完成明确要求的真实测试，不关闭 Issue；首次无 dyn0 的提交时序差异与既有扩展回归边界仍保留。

首次无 dyn0 时的提交时序差异见 [方案](plan.md#初始化列表与提交时序)，仍待确认，不能将 FR-003／SC-001 的全部时序条件勾选为通过。

## 命令与结果

仓库构建包装器负责加载本机编译环境和统一缓存；以下单元命令增加 `--lib`，避免为单元筛选重复链接所有集成测试。

| 命令 | 结果 |
| --- | --- |
| `bash scripts/build-native.sh test --package epwflow --lib phonon_state -- --nocapture` | 2 通过：序列化／失败筛选／损坏保护、12 个写入者并发回执 |
| `bash scripts/build-native.sh test --package epwflow --lib phonon_parallel -- --nocapture` | 3 通过：afterok 字符串、两模式脚本及 shell 语法、控制器身份校验 |
| `bash scripts/build-native.sh test --package epwflow --lib` | 95 通过，4 项既有真实数据诊断忽略 |
| `bash scripts/build-native.sh test --package epwflow --test three_stage_execution phonon_parallel -- --nocapture` | 本轮新增的 6 个默认协议用例通过；真实 Slurm 用例默认忽略，另行显式执行 |
| `bash scripts/build-native.sh test --package epwflow --test three_stage_execution phonon -- --nocapture` | 14 通过、21 失败、1 忽略；新增并行用例全部通过，扩展回归未通过，见下节 |

真实 Slurm 测试命令：

```bash
DFTFLOW_PHONON_SLURM_MODE=index \
DFTFLOW_THREE_STAGE_TEST_ROOT="$PWD/test_auto/MgB2/local_run/0075-phonon-parallel-slurm" \
bash scripts/build-native.sh test --package epwflow --test three_stage_execution \
  phonon_parallel_real_slurm_cancel_and_update_dependency -- --ignored --nocapture

DFTFLOW_PHONON_SLURM_MODE=coords \
DFTFLOW_THREE_STAGE_TEST_ROOT="$PWD/test_auto/MgB2/local_run/0075-phonon-parallel-slurm" \
bash scripts/build-native.sh test --package epwflow --test three_stage_execution \
  phonon_parallel_real_slurm_cancel_and_update_dependency -- --ignored --nocapture
```

两次均通过。index 的 init／成功 q／取消 q／collect／重提 q 作业号为 65／66／67／68／69；coords 为 70／71／72／73／74。两模式 collect 作业号均保持不变，依赖改为成功 q 与重提 q，最终归集和绘图成功。测试只取消自己新建的作业；结束时无遗留活动作业。

真实队列原始证据位于忽略目录：

- `test_auto/MgB2/local_run/0075-phonon-parallel-slurm/public-chain-18e917c62ebf0b68-11f01-0/`（index）。
- `test_auto/MgB2/local_run/0075-phonon-parallel-slurm/public-chain-18e918008f1f21e0-12ae8-0/`（coords）。

它们使用单任务 I/O 替身，不是 12 路 MPI 科学计算。首次真实测试暴露了 accounting storage 未启用时 PENDING 无法查询的问题；新增声子模块的控制器回退已修正，未改共享 Slurm 或 nosplit 实现。

## 扩展回归失败与提交边界

扩展筛选涵盖旧 nosplit 链、旧绘图／交付和导入用例。21 个旧用例失败，多数缺少前置 SCF／NSCF XML 或依赖它们的交付；另有旧断言期待准备命令在空工作区自动执行全部上游。

选取 `phonon_chain_ph_and_save_failures_block_post_and_resume_once` 的现场，用更新前的公开 `bin/epwflow/epw_phonon` 重试，退出 1，仍报告同一个缺失 SCF XML。旧二进制中不含 `phonon-qpoints.json` 字符串。该代表性旧行为的证据保存于忽略目录 `docs/local/0075-phonon-parallel/prechange-nosplit-failure.json`；没有逐个证明其余所有失败与本轮无关。

不修改被用户排除的科学阶段来改变这些旧用例的结果。由于扩展验证未通过，按照 `AGENTS.md` 的“若验证失败……不得提交”要求，本轮不执行 git commit／push。源码、测试及文档留在工作树供续接；原有三份无关实现保持未暂存。

## 编译与脚本生成

编译与固定入口检查完成后在本节追加结果。脚本生成使用
`test_auto/MgB2/local_run/0075-phonon-parallel-generation/{index,coords}/input.toml`，
各自独立 `run/`，保留用户现有计算目录，不执行示例中的 `rm -rf work`。

## 会话

`session_id=01a1edc5-a07b-7c03-b532-b0c721cbd23c`

## 原生 MgB₂ 双模式真实 Slurm 验收（2026-11-18）

本次按用户明确要求，在 `test_auto` 中完成真实 Slurm 提交、q 点失败后续算及依赖核验；每个 q 点使用 6 路 MPI，不用科学程序替身。测试脚本为 `test_auto/MgB2/epwflow/phonon_parallel_0075.py`，现场根目录为 `test_auto/MgB2/local_run/0075-native-slurm-20261118-01/`。

### 固定输入与执行边界

- MgB₂ PBE，k 网格 `[4,4,2]`、q 网格 `[2,2,1]`，原生 dyn0 给出两个不可约 q 点；截断能 40／320 Ry。SCF 为 12 路 MPI；init、各 q、collect 申请 6 CPU，q 使用 `mpirun -np 6`、PH 单 pool，OMP／MKL 各 1 线程。每个作业申请 6 GiB 内存、15 分钟时限，两个 q 可在本机 12 CPU 节点同时运行。
- 从本机当前 examples 配置派生独立输入，原配置、历史数据及现有运行目录未修改。CLI 在提交前复制到本次忽略目录的 `runtime/`，来源为本次初始提交 `dc0b16a5` 的固定入口；基线记录 CLI 和原生科学程序身份，运行后再次核对，无科学程序被替换。
- 每种模式各执行一次新 SCF、一次 PH 初始化、两个 q 的原生计算；取消 q0002 的首次分配后，只重提该点。collect 内真实执行一次 q2r、两次 matdyn，并生成声子色散／DOS PNG。
- 只核对输入、日志、XML／动力学矩阵及其他允许产物。电荷密度和原始波函数不参与哈希、大小、存在性或集合核验。

### 作业及结果

| 模式 | SCF | init | 成功 q0001 | 取消的 q0002 → 续算 | 保留的 collect | 结果 |
| --- | --- | --- | --- | --- | --- | --- |
| index | 90 | 91 | 92 | 93 → 95 | 94 | 通过 |
| coords | 96 | 97 | 98 | 99 → 101 | 100 | 通过 |

Slurm 完成日志记录 93、99 为 `CANCELLED`，其余十个作业均为 `COMPLETED`、`ExitCode=0:0`。故障通过取消已进入原生 PH 的本次作业注入，不取消用户的其他任务。

### 验收证据

| 检查项 | 实际结果与证据 |
| --- | --- |
| 独立提交与 init 依赖 | 两模式全部 q 在 init 退出前入队，控制器记录 `afterok:<init>`；实际开始时间均晚于 init 成功结束。`evidence/initial-controllers.json` 与完成日志保留原始时序。首次公开命令返回时仍只保证 init 已提交，不据此解除 T007。 |
| 真实并行及六路计算 | 两个原始 q 分配重叠执行 index 5 秒、coords 7 秒；控制器和完成日志均为 6 CPU，所有最终 `ph.out` 明确显示 6 个 MPI processor cores。 |
| 失败阻断 collect | 取消 q0002 后，collect 保持 `PENDING / DependencyNeverSatisfied`；此时没有 q2r 输出。见 `evidence/blocked-collect.json`。 |
| 失败点续算 | 公开 `--resume -s` 只将 93 换为 95、99 换为 101；init、成功 q 和 collect 的作业号全部保留。collect 依赖分别更新为 `afterok:95`、`afterok:101`，未保留失败作业号。见 `evidence/resumed.json` 与 `resumed-collect-controller.json`。 |
| 成功点复用 | q0001 的 `ph.in`、`ph.out`、动力学矩阵 SHA-256 及纳秒修改时间均未变化；见 `evidence/successful-q-before.json`、各模式 `result.json`。 |
| 原生结果与交付 | 两模式每个 q 的九个频率、正常结束、三个后处理日志、最终生产核验均通过；声子 PNG 已实际打开检查。18 个逐模频率的跨模式最大差为 0.000310 cm⁻¹，仅作一致性观察，不替代网格收敛。 |
| 完成后的幂等 | `--status --json` 中 phonon 可复用；重复 `--stage phonon -s` 不新增 q／collect，状态文件不变。公开命令 stdout／stderr 均保存在 `evidence/`。 |
| 收尾 | 全部 12 个测试分配均有 Slurm 终态，无遗留活动作业。 |

汇总位于根目录 `result.json`，每种模式的 `result.json` 保存作业终态、执行重叠时间、九模频率、后处理日志与图片身份。图像分别为 `index/run/results/4.qe-phonon-index/phonon.png`、`coords/run/results/4.qe-phonon-coords/phonon.png`。

附加验证：Python 语法编译通过；`--release --locked --package epwflow --lib phonon_parallel` 三项通过，`phonon_state` 两项通过。首次 debug 测试构建被信号终止，未产出测试结果，不计为通过；上述 release 两组随后正常完成。此次没有修改 Rust 科学／调度实现，也没有重跑此前已记录的 21 项扩展回归失败，不能将整库质量门记为通过。

本次验收支持勾选 T006；未解决 T007 的需求时序确认，不自动勾选 T005／T008 的其余工作或关闭 #75。测试脚本和本次事实回写按仓库规则单独提交，不将历史失败描述为本次已修复；未推送或修改线上 Issue。
