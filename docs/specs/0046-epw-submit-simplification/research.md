# 0046 技术取舍

- 故障来自中断后残留的目录锁；按照最新要求直接移除其正常入口与执行阶段的获取／恢复，不实现锁恢复框架或替代互斥。作业号和未决回执是重复提交判断依据。
- 固定脚本会被重写，不能继续把已完成脚本哈希当作科学复用依据；只对新格式的终态放开脚本校验，活动脚本和科学证据仍校验。
- stdout 必须由科学程序直接写 `.out`。历史版本 1 保留原始日志回收分支；新版本 2 不复制 Slurm 日志。
- 保留 `.err` 分开记录科学 stderr，Slurm stdout/stderr 合并到同一个可覆盖的日志。种子参数型 Wannier 程序保持原调用接口。
- 不修改共享 qf-runner／qeflow 的全部脚本，不触碰预算账本、注册表及其锁；避免将单项修复扩展为全仓库重构。

- 后续用户选择仅保留 --chdir：它在正文执行前确定 WorkDir 与相对日志位置，支持从其他目录手工提交；正文 cd 不能更改调度器 WorkDir。复用现有 Slurm 指令生成器，避免重复引用和路径逻辑。SLURM_SUBMIT_DIR 始终表示调用 sbatch 时所在目录，两者应分开说明。

官方依据：[sbatch --chdir](https://slurm.schedmd.com/sbatch.html#OPT_chdir)、[squeue WorkDir／%Z](https://slurm.schedmd.com/squeue.html)，本轮已读取。
