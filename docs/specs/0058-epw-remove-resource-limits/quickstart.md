# 快速验证

1. 从源配置 [resources] 删除 wall_seconds 和 storage_bytes。
2. 对干净的临时工作区执行 epw_phonon --check -c input.toml；新生成配置与脚本不再含这两项或 Slurm 时间指令。
3. 任意加回一个旧字段，配置校验应明确指出文件与未知字段，且不启动程序。
4. 使用测试替身验证完整链与超额账本，不向真实 Slurm 提交。

```bash
CARGO_PROFILE_TEST_DEBUG=0 bash scripts/build-native.sh test -p epwflow --no-fail-fast
```

以上 test 命令不更新公开安装。当前机器已安装程序需要另行构建／发布才会生效，本项不把源码测试当作部署完成。
