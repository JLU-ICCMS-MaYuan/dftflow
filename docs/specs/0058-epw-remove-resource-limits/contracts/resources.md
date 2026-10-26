# 配置与 CLI 契约

## 配置

```toml
[resources]
partition = "amd7513"
nodes = 1
mpi = 24
```

wall_seconds、storage_bytes 均为未知字段；报错指出配置文件路径与 resources 下的字段。不存在默认时长、存储额度或无限额开关。

## CLI

三个 EPW 程序不再接受 --wall-seconds 或 --storage-bytes。原有 --budget reserve 继续使用 --ledger、--label、--nodes、--mpi 登记身份；attach、status、reconcile 保持用途，时间／存储额度不再参与准入。

## Slurm

新生成脚本不写 #SBATCH --time；提交 argv 不传 --time 或 -t，也不注入替代超时。其他调度参数按当前逻辑生成；用户自行配置的调度规则不自动改写。

## 旧配置与快照

从用户维护的源配置中删除两字段，再生成新尝试。含旧字段的冻结快照直接拒绝；不指导手动修改带哈希快照，不承诺原尝试无缝恢复。历史科学文件、回执、账本及已提交脚本不批量重写。
