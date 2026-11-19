# 调查与决策

> 2026-11-17 最新约定：下文旧预算登记、账本读写和全局两个作业限制已经撤销，相关旧任务不再执行。当前实现与接口以 [0058 追加需求](../0058-epw-remove-resource-limits/spec.md) 为准；历史科学证据保留。

QE／VASP／Wannier 共享 qf-config 的 slurm.header，EPW 独立维护 Resources 并在三类脚本、旧 workflow 的 sbatch argv、并行校验及账本中使用。采用已有 SlurmSection，避免新建资源解析层。Slurm 指令不再作为 MPI 命令生成来源。实际节点数从 accounting 取得，不解析用户 shell 内容估算资源。

当前发布器 install-bin.sh 已逐文件覆盖固定路径并清理临时目录，无需创建新安装机制；publish-artifacts.sh 另存 published 哈希记录，应按用户要求移除。历史科学来源身份与配置快照属于续算证据，不是编译版本，不在本项删除。
