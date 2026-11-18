# 0075 验收记录

关联：[Issue #75](https://github.com/JLU-ICCMS-MaYuan/dftflow/issues/75)、[Spec](spec.md)、[方案](plan.md)。

## 验证边界

本轮实现四个源码阶段，并验证本地状态、脚本及公开进程协议。另在本机真实 Slurm 上运行两种模式的取消恢复测试；科学程序仍是隔离 I/O 替身，只证明真实调度器协议，不作为科学正确性的证据。真实 MgB₂ 的双模式并行计算及取消恢复尚未验收，不关闭 Issue。

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
