# 快速验证

当前配置不含 `[resources]` 或 `[budget]`。Slurm 文件头由 `[slurm].header` 提供，MPI 启动参数由 launcher 提供；程序不再提供预算命令、固定项目身份、账本路径或全局两个作业限制。

1. 从可维护的源输入中删除整个 `[budget]` 区段，不修改带哈希的历史快照或已有科学结果。
2. 在干净临时工作区执行 `epw_phonon --check -c input.toml`，应通过且不创建账本或 campaign 注册表。
3. 加回旧 `[budget]`，检查应明确报告文件与未知字段，不启动科学程序。
4. 三个程序的 `--help` 不含 `--budget`、`--ledger`、`--label`、`--token`、`--job`；传入这些参数应直接拒绝。
5. 用隔离替身运行提交、Slurm 分配内执行、状态及恢复回归。旧账本损坏或锁被占用不影响执行，三个独立工作区可使用同一状态目录提交；活动／未知作业仍不能重复提交。

```bash
bash "scripts/build-native.sh" test --release --locked -p epwflow -j 6
```

以上测试不发布公开命令，也不向真实 Slurm 提交科学作业。测试通过后重新构建覆盖固定入口：

```bash
bash "scripts/build-native.sh" build --release --locked -p epwflow --bins -j 6
```

核对构建产物与 `bin/epwflow/` 的固定命令一致，并再次检查安装后的帮助及旧参数拒绝行为。历史账本只留作资料，程序不再读写。
