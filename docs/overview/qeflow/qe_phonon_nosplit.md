# qe_phonon_nosplit：不分 q 的声子流程入口

[返回 qeflow 索引](README.md)

## 功能与入口

使用一个 ph.x 任务计算网格中的不可约 q 点，并准备声子流程输入。完整 SCF/PH/后处理的算例验收依赖显式作业脚本。

等价入口：`qeflow qe-phonon-nosplit`。

## 使用方法

```bash
qe_phonon_nosplit -c input.toml
# 仅当已按算例生成并检查此脚本时，才提交它
sbatch nosplit_job.sh
```

`-c/--config` 默认 input.toml；此聚合入口没有 -i 结构选项，使用配置/工作目录中的结构。解析器虽提供 -r/-s，但当前存在下述限制。

## 输入、输出与衔接

默认 `[qe_phonon.scf].outdir` 为 `./tmp`，SCF 与 PH 初始化/分 q/合并共用该设置；显式设置其他目录会覆盖默认值。已有运行目录若显式使用 `./out`，继续按原配置读取，不自动搬迁或改写历史输入。

默认工作目录 `qe_phonon_nosplit/`，可用 `[qe_phonon].work_dir` 覆盖；准备 SCF、PH 与后处理输入。`nosplit_job.sh` 是算例准备脚本生成的文件，不是该 CLI 必然生成的文件。

## 使用边界

当前直接 -r 分支漏执行 q2r/matdyn 后即尝试收集；-s 路径有重复后处理及相对配置路径问题。不能将直接 -r/-s 宣称为已验证的一次全链路。用户应使用已检查的显式作业脚本，或按 [单步入口](qe_ph_nosplit.md) 与后处理顺序执行。

## 实现依据与关联

[CLI 定义与分派](../../../qeflow/src/bin/qeflow.rs)；[聚合入口实现](../../../qeflow/src/bin/qeflow.rs)。默认目录的回归与安装验证见 [0006 US4 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。

参见 [三种模式与实际验收边界](phonon-workflow.md)；[0006 验收](../../specs/0006-qe-phonon-submit-chain/acceptance.md)。
